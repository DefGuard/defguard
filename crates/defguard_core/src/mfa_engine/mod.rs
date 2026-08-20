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
        Device, User, WireguardNetwork,
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
    enterprise::db::models::openid_provider::OpenIdProvider,
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::GatewayCommand,
    mfa_engine::{
        authorize::{build_authorized_gateway_network_info, create_new_session, emit_event},
        method::{InitiateError, Verdict, VerifyError, initiate, verify},
        types::{FinishOutcome, Proof, StartOutcome, StepStarted},
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
    /// arm step 0 (send the email code or mint the challenge), and persist the durable session.
    pub async fn start(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user: &User<Id>,
        flow_id: Id,
        steps: Vec<Vec<VpnClientMfaMethod>>,
        selected_method: VpnClientMfaMethod,
    ) -> Result<StartOutcome, Status> {
        // Reject a selected method the user has not set up. Every arm mirrors the exact error
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
                // the business license is active, so reaching this arm means the gate passed.
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

        // Arm step 0: send the email code or mint the biometric / mobile-approve challenge.
        let ctx = MfaSessionContext {
            location: location.clone(),
            device: device.clone(),
            user: user.clone(),
        };
        let challenge =
            initiate(&self.pool, &ctx, selected_method)
                .await
                .map_err(|err| match err {
                    InitiateError::EmailCode(e) => {
                        error!("Failed to generate email MFA code: {e}");
                        Status::internal("MFA code")
                    }
                    InitiateError::Database(e) => {
                        error!("Database error: {e}");
                        Status::internal("database error")
                    }
                    InitiateError::Mail(e) => {
                        error!(
                            "Failed to send email MFA code for user {}: {e}",
                            user.username
                        );
                        Status::internal("unexpected error")
                    }
                    InitiateError::BiometricNotConfigured => Status::invalid_argument(
                        "Select MFA method is not available for the device.",
                    ),
                    InitiateError::InvalidPublicKey(e) => {
                        error!(
                            "Start biometric MFA failed. Challenge creation failed. Reason: {e}"
                        );
                        Status::invalid_argument("Invalid public key")
                    }
                })?;
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
            selected_method,
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

    /// Arm the current step. Not reachable from any handler until the proxy's step-start route
    /// lands; the full state machine is implemented when that route is wired.
    pub async fn step_start(
        &self,
        _token: String,
        _method: VpnClientMfaMethod,
    ) -> Result<StepStarted, Status> {
        Err(Status::unimplemented("step start is not available"))
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
