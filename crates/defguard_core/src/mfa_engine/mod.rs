//! Connect-time multi-step MFA engine.
//!
//! The engine owns the step cursor and attempt lifecycle over the durable
//! [`VpnClientMfaSession`](defguard_common::db::models::vpn_client_mfa_session::VpnClientMfaSession)
//! store. The gRPC handlers in `grpc::proxy::client_mfa` stay thin adapters that convert the
//! frozen proto messages to and from the domain types here; the engine never sees a proto message.

use std::net::IpAddr;

use defguard_common::db::{
    Id,
    models::{
        Device, Settings, User, WireguardNetwork,
        biometric_auth::BiometricAuth,
        device::WireguardNetworkDevice,
        mfa_flow::MfaFlow,
        vpn_client_mfa_session::{
            MfaAttribution, MfaSessionContext, StepOutcome, VPN_MFA_SESSION_TIMEOUT,
            VpnClientMfaSession,
        },
        vpn_client_session::VpnClientMfaMethod,
    },
};
use sqlx::PgPool;
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};
use tonic::Status;

use crate::{
    enterprise::{db::models::openid_provider::OpenIdProvider, is_business_license_active},
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::GatewayCommand,
    mfa_engine::{
        authorize::{build_authorized_gateway_network_info, create_new_session, emit_event},
        method::{InitiateError, Verdict, VerifyError, initiate, verify},
        types::{
            FinishOutcome, Proof, StartOutcome, StartRejectionReason, StartResult, StepRejection,
            StepStarted,
        },
    },
};

pub mod authorize;
pub mod method;
pub mod types;

/// The connect-time MFA engine.
///
/// Holds only the pool and the two channel senders it needs to run a flow to completion: it mints
/// the session, verifies proofs, advances the step cursor, and - at the final step - authorizes
/// the peer, sends the gateway command, and emits the audit events.
pub struct MfaEngine {
    pool: PgPool,
    gateway_tx: Sender<GatewayCommand>,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
}

impl MfaEngine {
    #[must_use]
    pub fn new(
        pool: PgPool,
        gateway_tx: Sender<GatewayCommand>,
        bidi_event_tx: UnboundedSender<BidiStreamEvent>,
    ) -> Self {
        Self {
            pool,
            gateway_tx,
            bidi_event_tx,
        }
    }

    /// Begin a single-step login: validate the selected method against the user's configuration,
    /// initiate step 0 (send the email code or mint the challenge), and persist the durable session.
    pub async fn start(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user: &User<Id>,
        flow_id: Id,
        steps: Vec<Vec<VpnClientMfaMethod>>,
        selected_method: VpnClientMfaMethod,
    ) -> Result<StartOutcome, Status> {
        // Reject a selected method the user has not set up. Every branch mirrors the exact error
        // vocabulary of the legacy single-step path.
        match selected_method {
            VpnClientMfaMethod::Biometric => {
                if BiometricAuth::find_by_device_id(&self.pool, device.id)
                    .await
                    .map_err(|_| Status::internal("unexpected_error"))?
                    .is_none()
                {
                    return Err(Status::invalid_argument(
                        "Select MFA method is not available for the device.",
                    ));
                }
            }
            VpnClientMfaMethod::MobileApprove => {
                let result = BiometricAuth::find_by_user_id(&self.pool, user.id)
                    .await
                    .map_err(|_| Status::internal("unexpected error"))?;
                if result.is_empty() {
                    return Err(Status::invalid_argument(
                        "selected MFA method is not available",
                    ));
                }
            }
            VpnClientMfaMethod::Totp => {
                if !user.totp_enabled {
                    error!("TOTP not enabled for user {}", user.username);
                    return Err(Status::invalid_argument(
                        "selected MFA method is not available",
                    ));
                }
            }
            VpnClientMfaMethod::Email => {
                if !user.email_mfa_enabled {
                    error!("Email MFA not enabled for user {}", user.username);
                    return Err(Status::invalid_argument(
                        "selected MFA method is not available",
                    ));
                }
            }
            VpnClientMfaMethod::Oidc => {
                // No license check here: the caller's first-step filter already drops OIDC unless
                // the business license is active, so reaching this branch means the gate passed.
                if OpenIdProvider::get_current(&self.pool)
                    .await
                    .map_err(|err| {
                        error!("Failed to get current OpenID provider: {err}",);
                        Status::internal("unexpected error")
                    })?
                    .is_none()
                {
                    error!("OIDC provider is not configured");
                    return Err(Status::invalid_argument(
                        "selected MFA method is not available",
                    ));
                }
            }
        }

        self.start_session(location, device, user, flow_id, steps, selected_method)
            .await
    }

    /// Begin a multi-step login: validate the submitted plan against the resolved flow and the
    /// user's configuration, then initiate step 0 and persist the durable session. A refused plan
    /// returns sparse rejections and creates no session, token, or event.
    pub async fn start_multi_step(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user: &User<Id>,
        flow_id: Id,
        steps: Vec<Vec<VpnClientMfaMethod>>,
        selected_methods: Vec<VpnClientMfaMethod>,
    ) -> Result<StartResult, Status> {
        let business = is_business_license_active();

        // A multi-step flow (2+ steps) requires a business license; fail closed.
        if steps.len() > 1 && !business {
            return Err(Status::failed_precondition(
                "multi-step MFA is not available for this location",
            ));
        }
        if selected_methods.len() != steps.len() {
            return Err(Status::invalid_argument(
                "MFA plan length does not match the location's flow",
            ));
        }

        // Freeze the license-filtered snapshot: OIDC is a business-tier method.
        let filtered_steps: Vec<Vec<VpnClientMfaMethod>> = steps
            .iter()
            .map(|step| {
                step.iter()
                    .copied()
                    .filter(|method| *method != VpnClientMfaMethod::Oidc || business)
                    .collect()
            })
            .collect();

        let smtp_configured = Settings::get_current_settings().smtp_configured();
        let mut rejections = Vec::new();
        for (index, (chosen, allowed)) in selected_methods
            .iter()
            .zip(filtered_steps.iter())
            .enumerate()
        {
            let chosen = *chosen;
            if allowed.is_empty() {
                rejections.push(StepRejection {
                    step: index as u32,
                    reason: StartRejectionReason::StepEmptyAfterLicense,
                });
            } else if !allowed.contains(&chosen) {
                rejections.push(StepRejection {
                    step: index as u32,
                    reason: StartRejectionReason::MethodNotInStep,
                });
            } else if !matches!(chosen, VpnClientMfaMethod::Totp | VpnClientMfaMethod::Email)
                || !chosen
                    // TODO: oidc_configured is hardcoded false because the method boundary above
                    // rejects OIDC before this check. Compute it from the business license and
                    // the current OpenID provider once OIDC is allowed as a multi-step method.
                    .is_configured(&self.pool, user, device.id, smtp_configured, false)
                    .await
                    .map_err(|err| {
                        error!("Failed to check MFA method configuration: {err}");
                        Status::internal("unexpected error")
                    })?
            {
                rejections.push(StepRejection {
                    step: index as u32,
                    reason: StartRejectionReason::StepUnavailable,
                });
            }
        }

        if !rejections.is_empty() {
            return Ok(StartResult::Rejected(rejections));
        }

        let outcome = self
            .start_session(
                location,
                device,
                user,
                flow_id,
                filtered_steps,
                selected_methods[0],
            )
            .await?;
        Ok(StartResult::Accepted(outcome))
    }

    /// Initiate step 0 and persist the durable session, shared by the single-step and multi-step
    /// paths.
    async fn start_session(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user: &User<Id>,
        flow_id: Id,
        steps: Vec<Vec<VpnClientMfaMethod>>,
        method: VpnClientMfaMethod,
    ) -> Result<StartOutcome, Status> {
        // Initiate step 0: send the email code or mint the biometric / mobile-approve challenge.
        let ctx = MfaSessionContext {
            location: location.clone(),
            device: device.clone(),
            user: user.clone(),
        };
        let challenge = initiate(&self.pool, &ctx, method)
            .await
            .map_err(|err| Self::map_initiate_error(err, &user.username))?;
        let response_challenge = challenge
            .as_ref()
            .map(|challenge| challenge.challenge.clone());

        let mut conn = self.pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            Status::internal("unexpected error")
        })?;
        let (_session, outcome) = VpnClientMfaSession::<Id>::start(
            &mut conn,
            location.id,
            device.id,
            user.id,
            flow_id,
            steps,
            method,
            challenge,
            VPN_MFA_SESSION_TIMEOUT,
        )
        .await
        .map_err(|err| {
            error!("Failed to start MFA session: {err}");
            Status::internal("unexpected error")
        })?;

        Ok(StartOutcome {
            token: outcome.token,
            challenge: response_challenge,
            superseded_token_hash: outcome.superseded_token_hash,
        })
    }

    /// Map an [`InitiateError`] to a gRPC status, preserving the legacy error vocabulary.
    fn map_initiate_error(err: InitiateError, username: &str) -> Status {
        match err {
            InitiateError::EmailCode(e) => {
                error!("Failed to generate email MFA code: {e}");
                Status::internal("MFA code")
            }
            InitiateError::Database(e) => {
                error!("Database error: {e}");
                Status::internal("database error")
            }
            InitiateError::Mail(e) => {
                error!("Failed to send email MFA code for user {username}: {e}");
                Status::internal("unexpected error")
            }
            InitiateError::BiometricNotConfigured => {
                Status::invalid_argument("Select MFA method is not available for the device.")
            }
            InitiateError::InvalidPublicKey(e) => {
                error!("Start biometric MFA failed. Challenge creation failed. Reason: {e}");
                Status::invalid_argument("Invalid public key")
            }
        }
    }

    /// Initiate the current step: send the email code or mint the challenge and bind it to a
    /// fresh attempt. A re-call for the already-initiated method returns the existing attempt id
    /// without re-sending.
    pub async fn step_start(
        &self,
        token: String,
        method: VpnClientMfaMethod,
    ) -> Result<StepStarted, Status> {
        let Some(session) = VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, &token)
            .await
            .map_err(|err| {
                error!("Failed to find MFA session: {err}");
                Status::internal("unexpected error")
            })?
        else {
            error!("Client login session not found");
            return Err(Status::invalid_argument("login session not found"));
        };

        if !session.current_step_methods().contains(&method) {
            return Err(Status::invalid_argument(
                "MFA method is not in the current step",
            ));
        }

        // Idempotent re-call: return the existing attempt for this method without re-running
        // initiate (no second email or push).
        if let Some(ephemeral) = session.ephemeral_state.as_ref() {
            let ephemeral = &ephemeral.0;
            if ephemeral.selected_method == method {
                return Ok(StepStarted {
                    step_attempt_id: ephemeral.step_attempt_id.clone(),
                    challenge: ephemeral
                        .biometric_challenge
                        .as_ref()
                        .map(|challenge| challenge.challenge.clone()),
                });
            }
        }

        let Some(ctx) = session.load_context(&self.pool).await.map_err(|err| {
            error!("Failed to load MFA session context: {err}");
            Status::internal("unexpected error")
        })?
        else {
            error!("MFA session references a missing location, device, or user");
            return Err(Status::internal("unexpected error"));
        };

        let smtp_configured = Settings::get_current_settings().smtp_configured();
        if !method
            // oidc_configured is moot here: the session's steps are TOTP or Email only.
            .is_configured(&self.pool, &ctx.user, ctx.device.id, smtp_configured, false)
            .await
            .map_err(|err| {
                error!("Failed to check MFA method configuration: {err}");
                Status::internal("unexpected error")
            })?
        {
            return Err(Status::failed_precondition(
                "MFA method is not configured for this user",
            ));
        }

        let challenge = initiate(&self.pool, &ctx, method)
            .await
            .map_err(|err| Self::map_initiate_error(err, &ctx.user.username))?;

        let mut conn = self.pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            Status::internal("unexpected error")
        })?;
        let step_attempt_id = session
            .begin_attempt(&mut conn, method, challenge.clone())
            .await
            .map_err(|err| {
                error!("Failed to begin MFA attempt: {err}");
                Status::internal("unexpected error")
            })?;

        Ok(StepStarted {
            step_attempt_id,
            challenge: challenge
                .as_ref()
                .map(|challenge| challenge.challenge.clone()),
        })
    }

    /// Verify a proof, advance the step cursor, and - once the flow completes - authorize the
    /// peer and return the minted preshared key together with the method that satisfied the step.
    pub async fn finish(
        &self,
        token: String,
        proof: Proof,
        ip: IpAddr,
    ) -> Result<(FinishOutcome, VpnClientMfaMethod), Status> {
        let Some(session) = VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, &token)
            .await
            .map_err(|err| {
                error!("Failed to find MFA session: {err}");
                Status::internal("unexpected error")
            })?
        else {
            error!("Client login session not found");
            return Err(Status::invalid_argument("login session not found"));
        };

        let Some(ctx) = session.load_context(&self.pool).await.map_err(|err| {
            error!("Failed to load MFA session context: {err}");
            Status::internal("unexpected error")
        })?
        else {
            error!("MFA session references a missing location, device, or user");
            return Err(Status::internal("unexpected error"));
        };

        let Some(ephemeral_state) = session.ephemeral_state.as_ref() else {
            error!("No MFA attempt in progress");
            return Err(Status::invalid_argument("no MFA attempt in progress"));
        };
        let ephemeral = ephemeral_state.0.clone();

        let context = BidiRequestContext::new(
            ctx.user.id,
            ctx.user.username.clone(),
            ip,
            format!("{}", ctx.device),
        );

        let verdict = verify(&self.pool, &ctx, &ephemeral, &proof).await;

        let MfaSessionContext {
            location,
            device,
            user,
        } = ctx;
        let method: VpnClientMfaMethod = ephemeral.selected_method;

        let mut mobile_auth_device_name: Option<String> = None;
        match verdict {
            Ok(Verdict::Proved) => {
                if method == VpnClientMfaMethod::MobileApprove {
                    let auth_pub_key = proof.auth_pub_key.as_deref().ok_or_else(|| {
                        error!("Mobile approve auth pub key missing after successful verification");
                        Status::internal("unexpected error")
                    })?;
                    mobile_auth_device_name =
                        BiometricAuth::find_device(&self.pool, user.id, auth_pub_key)
                            .await
                            .map_err(|err| {
                                error!(
                                    "Failed to find mobile approve device for user {}: {err}",
                                    user.id
                                );
                                Status::internal("unexpected error")
                            })?
                            .map(|auth_device| auth_device.name);
                }
            }
            Ok(Verdict::NotYet) => {
                emit_event(
                    &self.bidi_event_tx,
                    BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location,
                                device,
                                method: method.into(),
                                message: "tried to finish OIDC MFA login but they haven't \
                                    completed OIDC authentication yet"
                                    .to_owned(),
                            },
                        )),
                    },
                )?;
                return Err(Status::failed_precondition(
                    "OIDC authentication not completed yet",
                ));
            }
            Ok(Verdict::Failed { message }) => {
                emit_event(
                    &self.bidi_event_tx,
                    BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location,
                                device,
                                method: method.into(),
                                message: message.to_owned(),
                            },
                        )),
                    },
                )?;
                self.record_failure(session).await?;
                return Err(Status::unauthenticated("unauthorized"));
            }
            Err(VerifyError::MalformedProof { status, event }) => {
                if let Some(event_message) = event {
                    emit_event(
                        &self.bidi_event_tx,
                        BidiStreamEvent {
                            context,
                            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                                DesktopClientMfaEvent::Failed {
                                    location,
                                    device,
                                    method: method.into(),
                                    message: event_message.to_owned(),
                                },
                            )),
                        },
                    )?;
                }
                return Err(Status::invalid_argument(status));
            }
            Err(VerifyError::DeviceNotOwned) => {
                return Err(Status::invalid_argument("Arguments invalid"));
            }
            Err(VerifyError::MissingChallenge) => {
                if method == VpnClientMfaMethod::Biometric {
                    return Err(Status::internal("Challenge not found in MFA session"));
                }
                return Err(Status::invalid_argument("Challenge not found in session"));
            }
            Err(VerifyError::Db(err)) => {
                error!("Failed to verify MFA proof: {err}");
                return Err(Status::internal("unexpected error"));
            }
        }

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        let Ok(Some(network_device)) =
            WireguardNetworkDevice::find(&mut *transaction, device.id, location.id).await
        else {
            error!("Failed to fetch network config for device {device} and location {location}");
            return Err(Status::internal("unexpected error"));
        };

        let (advance, snapshot) = session.advance(&mut transaction).await.map_err(|err| {
            error!("Failed to advance MFA session: {err}");
            Status::internal("unexpected error")
        })?;
        if advance != StepOutcome::Complete {
            error!("MFA session did not complete after its single step: {advance:?}");
            return Err(Status::internal("unexpected error"));
        }

        let flow_name = MfaFlow::find_by_id(&mut *transaction, snapshot.flow_id)
            .await
            .map_err(|err| {
                error!("Failed to resolve MFA flow for attribution: {err}");
                Status::internal("unexpected error")
            })?
            .map(|flow| flow.title);

        let key = WireguardNetwork::genkey();

        let vpn_client_session = create_new_session(
            &self.gateway_tx,
            &self.bidi_event_tx,
            &mut transaction,
            &location,
            &user,
            &device,
            true,
            key.public.clone(),
        )
        .await
        .map_err(|err| {
            error!("Failed to create new VPN client session for device {device} in location {location}: {err}");
            Status::internal("unexpected error")
        })?;
        debug!("Created new VPN client session: {vpn_client_session:?}");

        let gateway_network_info =
            build_authorized_gateway_network_info(network_device, key.public.clone());

        debug!("Sending `peer_create` message to gateway");
        let event =
            GatewayCommand::VpnSessionAuthorized(location.id, device.clone(), gateway_network_info);
        self.gateway_tx.send(event).map_err(|err| {
            error!("Error sending WireGuard event: {err}");
            Status::internal("unexpected error")
        })?;

        info!(
            "Desktop client login finished for {} at location {} with method {:?}",
            user.username, location.name, method
        );
        emit_event(
            &self.bidi_event_tx,
            BidiStreamEvent {
                context,
                event: BidiStreamEventType::DesktopClientMfa(Box::new(
                    DesktopClientMfaEvent::Success {
                        location,
                        device,
                        attribution: MfaAttribution {
                            snapshot,
                            flow_name,
                        },
                        mobile_auth_device_name,
                    },
                )),
            },
        )?;

        // Delete the in-progress session atomically with the authorization.
        session.delete(&mut *transaction).await.map_err(|err| {
            error!("Failed to delete MFA session: {err}");
            Status::internal("unexpected error")
        })?;

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction while finishing desktop client login.");
            Status::internal("unexpected error")
        })?;

        Ok((
            FinishOutcome::Completed {
                preshared_key: key.public.clone(),
            },
            method,
        ))
    }

    /// Record a proof-verification failure, deleting the session once the per-step cap is reached
    /// so a subsequent finish fails closed.
    async fn record_failure(&self, session: VpnClientMfaSession<Id>) -> Result<(), Status> {
        let mut conn = self.pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            Status::internal("unexpected error")
        })?;
        let at_cap = session
            .increment_failed_attempts(&mut conn)
            .await
            .map_err(|err| {
                error!("Failed to record MFA failure: {err}");
                Status::internal("unexpected error")
            })?;
        if at_cap {
            session.delete(&mut *conn).await.map_err(|err| {
                error!("Failed to delete MFA session: {err}");
                Status::internal("unexpected error")
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use chrono::{TimeDelta, Utc};
    use defguard_common::{
        db::{
            Id,
            models::{
                Device, DeviceType, User, WireguardNetwork,
                device::WireguardNetworkDevice,
                mfa_flow::{LocationMfaFlowAssignment, MfaFlow},
                settings::initialize_current_settings,
                vpn_client_mfa_session::{VpnClientMfaSession, hash_token},
                vpn_client_session::VpnClientMfaMethod,
                wireguard::ServiceLocationMode,
            },
            setup_pool,
        },
        testing::smtp::configure_working_smtp,
    };
    use ipnetwork::IpNetwork;
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };
    use tokio::sync::{broadcast, mpsc};
    use tonic::Code;

    use super::MfaEngine;
    use crate::{
        enterprise::{
            license::{License, LicenseTier, SupportType, set_cached_license},
            limits::{Counts, set_counts},
        },
        events::BidiStreamEvent,
        grpc::{GatewayCommand, proto::enterprise::license::LicenseLimits},
        mfa_engine::types::{StartRejectionReason, StartResult},
    };

    fn set_test_license_business() {
        let license = License::new(
            "test".to_owned(),
            true,
            Some(Utc::now() + TimeDelta::days(1)),
            Some(LicenseLimits {
                users: 100,
                devices: 100,
                locations: 100,
                network_devices: Some(100),
            }),
            None,
            LicenseTier::Enterprise,
            SupportType::Basic,
            Vec::new(),
        );
        set_cached_license(Some(license));
        set_counts(Counts::new(1, 1, 1, 1));
    }

    fn clear_test_license() {
        set_cached_license(None);
    }

    fn make_engine(
        pool: PgPool,
    ) -> (
        MfaEngine,
        mpsc::UnboundedReceiver<BidiStreamEvent>,
        broadcast::Receiver<GatewayCommand>,
    ) {
        let (gateway_tx, gateway_rx) = broadcast::channel(8);
        let (bidi_event_tx, bidi_event_rx) = mpsc::unbounded_channel();
        (
            MfaEngine::new(pool, gateway_tx, bidi_event_tx),
            bidi_event_rx,
            gateway_rx,
        )
    }

    async fn create_user(pool: &PgPool) -> User<Id> {
        User::new(
            "mfa-engine-test-user".to_owned(),
            Some("pass123"),
            "Tester".to_owned(),
            "MfaEngine".to_owned(),
            "mfa-engine-test@example.com".to_owned(),
            None,
        )
        .save(pool)
        .await
        .expect("failed to create user")
    }

    async fn create_device(pool: &PgPool, user_id: Id) -> Device<Id> {
        Device::new(
            "mfa-engine-test-device".to_owned(),
            "mfa-engine-test-pubkey".to_owned(),
            user_id,
            DeviceType::User,
            None,
            true,
        )
        .save(pool)
        .await
        .expect("failed to create device")
    }

    async fn create_mfa_location(pool: &PgPool) -> WireguardNetwork<Id> {
        WireguardNetwork::new(
            "mfa-engine-test-location".to_owned(),
            51820,
            "vpn.example.com".to_owned(),
            None,
            [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
            true,
            false,
            false,
            false,
            true, // mfa_enabled
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, 0, 1)), 24).unwrap()])
        .expect("failed to set location address")
        .save(pool)
        .await
        .expect("failed to create location")
    }

    async fn attach_device_to_location(pool: &PgPool, location_id: Id, device_id: Id) {
        WireguardNetworkDevice::new(
            location_id,
            device_id,
            vec![IpAddr::V4(Ipv4Addr::new(10, 10, 0, 10))],
        )
        .insert(pool)
        .await
        .expect("failed to attach device to location");
    }

    async fn create_and_assign_flow(
        pool: &PgPool,
        location_id: Id,
        steps: Vec<Vec<VpnClientMfaMethod>>,
    ) {
        let mut tx = pool.begin().await.expect("failed to begin tx");
        let (flow, _) = MfaFlow::create(&mut tx, "mfa-engine-test-flow".to_owned(), steps)
            .await
            .expect("failed to create flow");
        MfaFlow::assign_to_location(
            &mut tx,
            location_id,
            &[LocationMfaFlowAssignment {
                flow_id: flow.id,
                is_default: true,
                group_ids: Vec::new(),
            }],
        )
        .await
        .expect("failed to assign flow");
        tx.commit().await.expect("failed to commit tx");
    }

    async fn resolve_flow(
        pool: &PgPool,
        location_id: Id,
        user_id: Id,
    ) -> (Id, Vec<Vec<VpnClientMfaMethod>>) {
        let mut conn = pool.acquire().await.expect("failed to acquire conn");
        let (flow, steps) = MfaFlow::resolve_for_user(&mut conn, location_id, user_id)
            .await
            .expect("failed to resolve flow")
            .expect("flow should resolve");
        (flow.id, steps.into_iter().map(|s| s.methods).collect())
    }

    async fn session_count(pool: &PgPool, location_id: Id, device_id: Id) -> i64 {
        sqlx::query_scalar!(
            "SELECT count(*) FROM vpn_client_mfa_session WHERE location_id = $1 AND device_id = $2",
            location_id,
            device_id,
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .unwrap_or(0)
    }

    #[sqlx::test]
    async fn test_start_multi_step_valid_totp_email_plan_returns_token(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_test_license_business();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let _smtp = configure_working_smtp(&pool).await;

        let location = create_mfa_location(&pool).await;
        create_and_assign_flow(
            &pool,
            location.id,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await;
        let mut user = create_user(&pool).await;
        user.enable_totp(&pool)
            .await
            .expect("failed to enable TOTP");
        user.enable_email_mfa(&pool)
            .await
            .expect("failed to enable email MFA");
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;

        let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
        let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());

        let result = engine
            .start_multi_step(
                &location,
                &device,
                &user,
                flow_id,
                step_methods,
                vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
            )
            .await
            .expect("start should succeed");
        let StartResult::Accepted(outcome) = result else {
            panic!("expected an accepted plan")
        };
        assert!(!outcome.token.is_empty());
        assert!(outcome.superseded_token_hash.is_none());
        assert!(
            VpnClientMfaSession::<Id>::find_active_by_token(&pool, &outcome.token)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[sqlx::test]
    async fn test_start_multi_step_supersedes_prior_session(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_test_license_business();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let _smtp = configure_working_smtp(&pool).await;

        let location = create_mfa_location(&pool).await;
        create_and_assign_flow(
            &pool,
            location.id,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await;
        let mut user = create_user(&pool).await;
        user.enable_totp(&pool)
            .await
            .expect("failed to enable TOTP");
        user.enable_email_mfa(&pool)
            .await
            .expect("failed to enable email MFA");
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;

        let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
        let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
        let plan = vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email];

        let first = engine
            .start_multi_step(
                &location,
                &device,
                &user,
                flow_id,
                step_methods.clone(),
                plan.clone(),
            )
            .await
            .expect("first start should succeed");
        let StartResult::Accepted(first_outcome) = first else {
            panic!("expected an accepted plan")
        };

        let second = engine
            .start_multi_step(&location, &device, &user, flow_id, step_methods, plan)
            .await
            .expect("second start should succeed");
        let StartResult::Accepted(second_outcome) = second else {
            panic!("expected an accepted plan")
        };

        assert_eq!(
            second_outcome.superseded_token_hash,
            Some(hash_token(&first_outcome.token))
        );
        assert!(
            VpnClientMfaSession::<Id>::find_active_by_token(&pool, &first_outcome.token)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            VpnClientMfaSession::<Id>::find_active_by_token(&pool, &second_outcome.token)
                .await
                .unwrap()
                .is_some()
        );
    }

    #[sqlx::test]
    async fn test_start_multi_step_rejects_out_of_boundary_method(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_test_license_business();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");

        let location = create_mfa_location(&pool).await;
        create_and_assign_flow(
            &pool,
            location.id,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Biometric],
            ],
        )
        .await;
        let mut user = create_user(&pool).await;
        user.enable_totp(&pool)
            .await
            .expect("failed to enable TOTP");
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;

        let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
        let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

        let result = engine
            .start_multi_step(
                &location,
                &device,
                &user,
                flow_id,
                step_methods,
                vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Biometric],
            )
            .await
            .expect("start should return a rejection, not an error");
        let StartResult::Rejected(rejections) = result else {
            panic!("expected a rejected plan")
        };
        assert_eq!(rejections.len(), 1);
        assert_eq!(rejections[0].step, 1);
        assert_eq!(rejections[0].reason, StartRejectionReason::StepUnavailable);
        assert!(
            event_rx.try_recv().is_err(),
            "a rejection must not emit an event"
        );
        assert_eq!(session_count(&pool, location.id, device.id).await, 0);
    }

    #[sqlx::test]
    async fn test_start_multi_step_unlicensed_fails_closed(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        clear_test_license();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");

        let location = create_mfa_location(&pool).await;
        create_and_assign_flow(
            &pool,
            location.id,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;

        let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
        let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

        let err = engine
            .start_multi_step(
                &location,
                &device,
                &user,
                flow_id,
                step_methods,
                vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
            )
            .await
            .expect_err("an unlicensed multi-step plan must fail closed");
        assert_eq!(err.code(), Code::FailedPrecondition);
        assert_eq!(
            err.message(),
            "multi-step MFA is not available for this location"
        );
        assert!(
            !err.message().contains("no valid license"),
            "the license gate message must not contain 'no valid license'"
        );
        assert!(event_rx.try_recv().is_err());
        assert_eq!(session_count(&pool, location.id, device.id).await, 0);
    }

    #[sqlx::test]
    async fn test_start_multi_step_rejects_unconfigured_method(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        set_test_license_business();
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");

        let location = create_mfa_location(&pool).await;
        create_and_assign_flow(
            &pool,
            location.id,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await;
        // TOTP is configured; email is not, so only the email step is unavailable.
        let mut user = create_user(&pool).await;
        user.enable_totp(&pool)
            .await
            .expect("failed to enable TOTP");
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;

        let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
        let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());

        let result = engine
            .start_multi_step(
                &location,
                &device,
                &user,
                flow_id,
                step_methods,
                vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email],
            )
            .await
            .expect("start should return a rejection, not an error");
        let StartResult::Rejected(rejections) = result else {
            panic!("expected a rejected plan")
        };
        assert_eq!(rejections.len(), 1);
        assert_eq!(rejections[0].step, 1);
        assert_eq!(rejections[0].reason, StartRejectionReason::StepUnavailable);
        assert!(event_rx.try_recv().is_err());
        assert_eq!(session_count(&pool, location.id, device.id).await, 0);
    }
}
