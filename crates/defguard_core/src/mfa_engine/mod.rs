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
            MFA_FAILED_ATTEMPT_CAP, MfaAttribution, MfaSessionContext, StepOutcome, StepsSnapshot,
            VPN_MFA_SESSION_TIMEOUT, VpnClientMfaSession,
        },
        vpn_client_session::VpnClientMfaMethod,
    },
};
use sqlx::{PgConnection, PgPool};
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};

use crate::{
    enterprise::{db::models::openid_provider::OpenIdProvider, is_business_license_active},
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::GatewayCommand,
    mfa_engine::{
        authorize::{EventChannels, build_authorized_gateway_network_info, create_new_session},
        error::{FinishError, StartError, StepError},
        method::{InitiateError, Verdict, VerifyError, initiate, verify},
        types::{
            FinishOutcome, Proof, StartOutcome, StartRejectionReason, StartResult, StepRejection,
            StepStarted,
        },
    },
};

pub mod authorize;
pub mod error;
pub mod method;
pub mod types;

/// The connect-time MFA engine.
///
/// Holds only the pool and the outbound channels it needs to run a flow to completion: it mints
/// the session, verifies proofs, advances the step cursor, and - at the final step - authorizes
/// the peer, sends the gateway command, and emits the audit events.
///
/// License gating happens only at `start`: it freezes the license-filtered step snapshot, and an
/// in-flight flow is allowed to run to completion even if the license lapses mid-flow (deliberate;
/// see ticket 01). `step_start` and `finish` therefore carry no license gate.
pub struct MfaEngine {
    pool: PgPool,
    channels: EventChannels,
}

/// The side effects of a completed flow, built inside the transaction but dispatched by the
/// caller **after** it commits.
///
/// Authorizing the peer on the gateway and recording the success event cannot be rolled back. If
/// they were dispatched inside the transaction and the commit then failed, the gateway would hold
/// an authorized peer and the audit log would claim success while the `VpnClientSession` row
/// vanished - an authorized tunnel with no record of it. Dispatching after the commit fails the
/// other way instead: the database is the record, and a failed dispatch surfaces as an error
/// rather than as silent access.
struct CompletedFlow {
    outcome: FinishOutcome,
    gateway_command: GatewayCommand,
    event: BidiStreamEvent,
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
            channels: EventChannels::new(gateway_tx, bidi_event_tx),
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
    ) -> Result<StartOutcome, StartError> {
        // Reject a selected method the user has not set up. `is_configured` is shared with
        // `start_multi_step` and `step_start` so the paths cannot disagree.
        //
        // Email needs `smtp_configured` too: `initiate` hands the code to `send_and_forget`, so a
        // send failure has no way back to the client and an unusable mailer must be caught here.
        //
        // OIDC needs no license check - the caller's first-step filter drops it when unlicensed.
        let smtp_configured = Settings::get_current_settings().smtp_configured();
        let oidc_configured = self.oidc_configured().await.map_err(|err| {
            error!("Failed to get current OpenID provider: {err}");
            StartError::Internal
        })?;
        if !selected_method
            .is_configured(
                &self.pool,
                user,
                device.id,
                smtp_configured,
                oidc_configured,
            )
            .await
            .map_err(|err| {
                error!("Failed to check MFA method configuration: {err}");
                StartError::Internal
            })?
        {
            // Biometric reports a device-scoped message, the rest a generic one. Which method
            // gets which string is client-visible.
            if selected_method == VpnClientMfaMethod::Biometric {
                error!("Biometric MFA is not configured for device {}", device.id);
                return Err(StartError::BiometricNotConfigured);
            }
            error!(
                "MFA method {selected_method:?} is not configured for user {}",
                user.username
            );
            return Err(StartError::MethodNotAvailable);
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
    ) -> Result<StartResult, StartError> {
        let business = is_business_license_active();

        // A multi-step flow (2+ steps) requires a business license; fail closed.
        if steps.len() > 1 && !business {
            error!(
                "Multi-step MFA requires a business license; location {} has a {}-step flow",
                location.name,
                steps.len()
            );
            return Err(StartError::MultiStepNotAvailable);
        }
        if selected_methods.len() != steps.len() {
            error!(
                "MFA plan length {} does not match the {}-step flow of location {}",
                selected_methods.len(),
                steps.len(),
                location.name
            );
            return Err(StartError::PlanLengthMismatch);
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
        let oidc_configured = self.oidc_configured().await.map_err(|err| {
            error!("Failed to get current OpenID provider: {err}");
            StartError::Internal
        })?;
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
            } else if (steps.len() > 1
                && !matches!(chosen, VpnClientMfaMethod::Totp | VpnClientMfaMethod::Email))
                || !chosen
                    .is_configured(
                        &self.pool,
                        user,
                        device.id,
                        smtp_configured,
                        oidc_configured,
                    )
                    .await
                    .map_err(|err| {
                        error!("Failed to check MFA method configuration: {err}");
                        StartError::Internal
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
    ) -> Result<StartOutcome, StartError> {
        // Initiate step 0: send the email code or mint the biometric / mobile-approve challenge.
        let ctx = MfaSessionContext {
            location: location.clone(),
            device: device.clone(),
            user: user.clone(),
        };
        let challenge = initiate(&self.pool, &ctx, method).await.map_err(|err| {
            log_initiate_error(&err, &user.username);
            StartError::from(err)
        })?;
        let response_challenge = challenge
            .as_ref()
            .map(|challenge| challenge.challenge.clone());

        let mut conn = self.pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            StartError::Internal
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
            StartError::Internal
        })?;

        Ok(StartOutcome {
            token: outcome.token,
            challenge: response_challenge,
            superseded_token_hash: outcome.superseded_token_hash,
        })
    }

    /// Whether OIDC is configured for this deployment: a business license plus a configured
    /// OpenID provider. Shared by `start_multi_step` and `step_start` so both paths agree with
    /// the descriptor builder's source for `oidc_configured`.
    async fn oidc_configured(&self) -> sqlx::Result<bool> {
        if !is_business_license_active() {
            return Ok(false);
        }
        Ok(OpenIdProvider::get_current(&self.pool).await?.is_some())
    }

    /// Initiate the current step: send the email code or mint the challenge and bind it to a
    /// fresh attempt.
    ///
    /// Every call runs the same fixed sequence regardless of the step's state - there is no
    /// branch for an already-initialized step. A re-call is therefore a legal switch (to a
    /// different method) or a retry (the same method), and both re-run `initiate` and mint a
    /// fresh attempt id: that is what makes "resend the code" work. The abandoned attempt's side
    /// effects are not cancelled; stale callbacks no-op on the superseded attempt id.
    ///
    /// A re-call does not touch `failed_attempts` - that counter bounds wrong proofs, not
    /// initialization. Bounding re-initiation (and so the mail/push it triggers) is tracked
    /// separately in DefGuard/defguard#3585.
    pub async fn step_start(
        &self,
        token: String,
        method: VpnClientMfaMethod,
    ) -> Result<StepStarted, StepError> {
        let Some(session) = VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, &token)
            .await
            .map_err(|err| {
                error!("Failed to find MFA session: {err}");
                StepError::Internal
            })?
        else {
            error!("Client login session not found");
            return Err(StepError::SessionNotFound);
        };

        if !session.current_step_methods().contains(&method) {
            error!("MFA method {method:?} is not in the current step");
            return Err(StepError::MethodNotInStep);
        }

        let Some(ctx) = session.load_context(&self.pool).await.map_err(|err| {
            error!("Failed to load MFA session context: {err}");
            StepError::Internal
        })?
        else {
            error!("MFA session references a missing location, device, or user");
            return Err(StepError::Internal);
        };

        let smtp_configured = Settings::get_current_settings().smtp_configured();
        let oidc_configured = self.oidc_configured().await.map_err(|err| {
            error!("Failed to get current OpenID provider: {err}");
            StepError::Internal
        })?;
        if !method
            .is_configured(
                &self.pool,
                &ctx.user,
                ctx.device.id,
                smtp_configured,
                oidc_configured,
            )
            .await
            .map_err(|err| {
                error!("Failed to check MFA method configuration: {err}");
                StepError::Internal
            })?
        {
            error!(
                "MFA method {method:?} is not configured for user {}",
                ctx.user.username
            );
            return Err(StepError::MethodNotConfigured);
        }

        let challenge = initiate(&self.pool, &ctx, method).await.map_err(|err| {
            log_initiate_error(&err, &ctx.user.username);
            StepError::from(err)
        })?;

        let mut conn = self.pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            StepError::Internal
        })?;
        let step_attempt_id = session
            .begin_attempt(&mut conn, method, challenge.clone())
            .await
            .map_err(|err| {
                error!("Failed to begin MFA attempt: {err}");
                StepError::Internal
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
    ) -> Result<(FinishOutcome, VpnClientMfaMethod), FinishError> {
        let Some(session) = VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, &token)
            .await
            .map_err(|err| {
                error!("Failed to find MFA session: {err}");
                FinishError::Internal
            })?
        else {
            error!("Client login session not found");
            return Err(FinishError::SessionNotFound);
        };

        let Some(ctx) = session.load_context(&self.pool).await.map_err(|err| {
            error!("Failed to load MFA session context: {err}");
            FinishError::Internal
        })?
        else {
            error!("MFA session references a missing location, device, or user");
            return Err(FinishError::Internal);
        };

        let Some(ephemeral_state) = session.ephemeral_state.as_ref() else {
            error!("No MFA attempt in progress");
            return Err(FinishError::UninitializedStep);
        };
        let ephemeral = ephemeral_state.0.clone();

        let context = BidiRequestContext::new(
            ctx.user.id,
            ctx.user.username.clone(),
            ip,
            format!("{}", ctx.device),
        );

        let verdict = verify(&self.pool, &ctx, &ephemeral, &proof).await;

        let method: VpnClientMfaMethod = ephemeral.selected_method;

        let mut mobile_auth_device_name: Option<String> = None;
        match verdict {
            Ok(Verdict::Proved) => {
                if method == VpnClientMfaMethod::MobileApprove {
                    let auth_pub_key = proof.auth_pub_key.as_deref().ok_or_else(|| {
                        error!("Mobile approve auth pub key missing after successful verification");
                        FinishError::Internal
                    })?;
                    mobile_auth_device_name =
                        BiometricAuth::find_device(&self.pool, ctx.user.id, auth_pub_key)
                            .await
                            .map_err(|err| {
                                error!(
                                    "Failed to find mobile approve device for user {}: {err}",
                                    ctx.user.id
                                );
                                FinishError::Internal
                            })?
                            .map(|auth_device| auth_device.name);
                }
            }
            Ok(Verdict::NotYet) => {
                self.channels.emit_event(BidiStreamEvent {
                    context,
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: ctx.location.clone(),
                            device: ctx.device.clone(),
                            method: method.into(),
                            message: "tried to finish OIDC MFA login but they haven't \
                                    completed OIDC authentication yet"
                                .to_owned(),
                        },
                    )),
                })?;
                return Err(FinishError::OidcNotCompleted);
            }
            Ok(Verdict::Failed { message }) => {
                self.channels.emit_event(BidiStreamEvent {
                    context,
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: ctx.location.clone(),
                            device: ctx.device.clone(),
                            method: method.into(),
                            message: message.to_owned(),
                        },
                    )),
                })?;
                self.record_failure(session).await?;
                return Err(FinishError::Unauthorized);
            }
            Err(VerifyError::MalformedProof { message, event }) => {
                if let Some(event_message) = event {
                    self.channels.emit_event(BidiStreamEvent {
                        context,
                        event: BidiStreamEventType::DesktopClientMfa(Box::new(
                            DesktopClientMfaEvent::Failed {
                                location: ctx.location.clone(),
                                device: ctx.device.clone(),
                                method: method.into(),
                                message: event_message.to_owned(),
                            },
                        )),
                    })?;
                }
                return Err(FinishError::MalformedProof { message });
            }
            Err(VerifyError::MissingChallenge) => {
                if method == VpnClientMfaMethod::Biometric {
                    return Err(FinishError::MissingBiometricChallenge);
                }
                return Err(FinishError::MissingChallenge);
            }
            Err(VerifyError::Db(err)) => {
                error!("Failed to verify MFA proof: {err}");
                return Err(FinishError::Internal);
            }
        }

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            FinishError::Internal
        })?;

        // Advance the satisfied step, bound to the exact step and attempt the proof was minted
        // for. A stale or duplicate advance matches zero rows (someone else already advanced
        // this step, or the proof is for a superseded attempt); a non-final advance returns
        // `Advanced` without authorizing or deleting the session.
        let current_step = session.current_step;
        let step_attempt_id = proof.step_attempt_id.as_deref();
        let Some((advance, snapshot)) = session
            .advance(&mut transaction, current_step, step_attempt_id, method)
            .await
            .map_err(|err| {
                error!("Failed to advance MFA session: {err}");
                FinishError::Internal
            })?
        else {
            error!("Stale MFA attempt: the step was already advanced or the attempt is superseded");
            return Err(FinishError::StaleAttempt);
        };
        if let StepOutcome::Advanced { next_step } = advance {
            transaction.commit().await.map_err(|_| {
                error!("Failed to commit transaction while advancing MFA flow.");
                FinishError::Internal
            })?;
            return Ok((
                FinishOutcome::Advanced {
                    next_step: next_step as u32,
                },
                method,
            ));
        }

        let completed = self
            .complete_flow(
                &mut transaction,
                session,
                snapshot,
                &ctx,
                context,
                mobile_auth_device_name,
            )
            .await?;

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction while finishing desktop client login.");
            FinishError::Internal
        })?;

        // Only now that the authorization is durable: tell the gateway to create the peer and
        // record the success. See `CompletedFlow` for why the order matters.
        debug!("Sending `peer_create` message to gateway");
        self.channels
            .gateway_tx
            .send(completed.gateway_command)
            .map_err(|err| {
                error!("Error sending WireGuard event: {err}");
                FinishError::Internal
            })?;

        info!(
            "Desktop client login finished for {} at location {} with method {method:?}",
            ctx.user.username, ctx.location.name
        );
        self.channels.emit_event(completed.event)?;

        Ok((completed.outcome, method))
    }

    /// Complete the flow: mint the preshared key, create the VPN client session, and delete the
    /// in-progress MFA session. This is the single place a preshared key is minted or a peer is
    /// authorized.
    ///
    /// Everything here is transactional. The two side effects that are not - the gateway command
    /// and the success event - are returned in [`CompletedFlow`] for the caller to dispatch once
    /// the transaction has committed.
    async fn complete_flow(
        &self,
        transaction: &mut PgConnection,
        session: VpnClientMfaSession<Id>,
        snapshot: StepsSnapshot,
        ctx: &MfaSessionContext,
        context: BidiRequestContext,
        mobile_auth_device_name: Option<String>,
    ) -> Result<CompletedFlow, FinishError> {
        let Ok(Some(network_device)) =
            WireguardNetworkDevice::find(&mut *transaction, ctx.device.id, ctx.location.id).await
        else {
            error!(
                "Failed to fetch network config for device {} and location {}",
                ctx.device, ctx.location
            );
            return Err(FinishError::Internal);
        };

        let flow_name = MfaFlow::find_by_id(&mut *transaction, snapshot.flow_id)
            .await
            .map_err(|err| {
                error!("Failed to resolve MFA flow for attribution: {err}");
                FinishError::Internal
            })?
            .map(|flow| flow.title);

        let key = WireguardNetwork::genkey();

        let vpn_client_session = create_new_session(
            &self.channels,
            &mut *transaction,
            &ctx.location,
            &ctx.user,
            &ctx.device,
            true,
            key.public.clone(),
        )
        .await
        .map_err(|err| {
            error!(
                "Failed to create new VPN client session for device {} in location {}: {err}",
                ctx.device, ctx.location
            );
            FinishError::Internal
        })?;
        debug!(
            "Created new VPN client session with id {}",
            vpn_client_session.id
        );

        let gateway_network_info =
            build_authorized_gateway_network_info(network_device, key.public.clone());

        let gateway_command = GatewayCommand::VpnSessionAuthorized(
            ctx.location.id,
            ctx.device.clone(),
            gateway_network_info,
        );

        let event = BidiStreamEvent {
            context,
            event: BidiStreamEventType::DesktopClientMfa(Box::new(
                DesktopClientMfaEvent::Success {
                    location: ctx.location.clone(),
                    device: ctx.device.clone(),
                    attribution: MfaAttribution {
                        snapshot,
                        flow_name,
                    },
                    mobile_auth_device_name,
                },
            )),
        };

        // Delete the in-progress session atomically with the authorization.
        session.delete(&mut *transaction).await.map_err(|err| {
            error!("Failed to delete MFA session: {err}");
            FinishError::Internal
        })?;

        Ok(CompletedFlow {
            outcome: FinishOutcome::Completed {
                preshared_key: key.public.clone(),
            },
            gateway_command,
            event,
        })
    }

    /// Record a proof-verification failure, deleting the session once the per-step cap is reached
    /// so a subsequent finish fails closed.
    async fn record_failure(&self, session: VpnClientMfaSession<Id>) -> Result<(), FinishError> {
        let mut conn = self.pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            FinishError::Internal
        })?;
        let at_cap = session
            .increment_failed_attempts(&mut conn)
            .await
            .map_err(|err| {
                error!("Failed to record MFA failure: {err}");
                FinishError::Internal
            })?;
        if at_cap {
            warn!(
                "MFA session {} hit the failed-attempt cap of {MFA_FAILED_ATTEMPT_CAP}; deleting it",
                session.id
            );
            session.delete(&mut *conn).await.map_err(|err| {
                error!("Failed to delete MFA session: {err}");
                FinishError::Internal
            })?;
        }
        Ok(())
    }
}

/// Log an [`InitiateError`] with the context it needs, so the caller can then wrap it into a
/// typed error (`StartError` or `StepError`) without duplicating the logging.
fn log_initiate_error(err: &InitiateError, username: &str) {
    match err {
        InitiateError::EmailCode(e) => error!("Failed to generate email MFA code: {e}"),
        InitiateError::Database(e) => error!("Database error: {e}"),
        InitiateError::Mail(e) => {
            error!("Failed to send email MFA code for user {username}: {e}")
        }
        InitiateError::BiometricNotConfigured => {}
        InitiateError::InvalidPublicKey(e) => {
            error!("Start biometric MFA failed. Challenge creation failed. Reason: {e}")
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr},
        time::SystemTime,
    };

    use chrono::{TimeDelta, Utc};
    use defguard_common::{
        db::{
            Id,
            models::{
                Device, DeviceType, User, WireguardNetwork,
                biometric_auth::BiometricAuth,
                device::WireguardNetworkDevice,
                mfa_flow::{LocationMfaFlowAssignment, MfaFlow},
                settings::initialize_current_settings,
                user::{TOTP_CODE_DIGITS, TOTP_CODE_VALIDITY_PERIOD},
                vpn_client_mfa_session::{
                    MFA_FAILED_ATTEMPT_CAP, VPN_MFA_SESSION_TIMEOUT, VpnClientMfaSession,
                    hash_token,
                },
                vpn_client_session::{VpnClientMfaMethod, VpnClientSession},
                wireguard::ServiceLocationMode,
            },
            setup_pool,
        },
        testing::smtp::configure_working_smtp,
    };
    use defguard_proto::client_types::MfaMethod;
    use ipnetwork::IpNetwork;
    use sqlx::{
        PgPool,
        postgres::{PgConnectOptions, PgPoolOptions},
    };
    use tokio::sync::{broadcast, mpsc};
    use tonic::{Code, Status};
    use totp_lite::{Sha1, totp_custom};

    use super::MfaEngine;
    use crate::{
        enterprise::{
            license::{License, LicenseTier, SupportType, set_cached_license},
            limits::{Counts, set_counts},
        },
        events::{BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
        grpc::{GatewayCommand, proto::enterprise::license::LicenseLimits},
        mfa_engine::types::{FinishOutcome, Proof, StartRejectionReason, StartResult},
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
            LicenseTier::Business,
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
        let err = Status::from(err);
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

    /// A single-step plan using a non-TOTP/Email method must not be rejected by the MVP method
    /// boundary, which is multi-step only.
    #[sqlx::test]
    async fn test_start_multi_step_single_step_non_boundary_method_accepted(
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
            vec![vec![VpnClientMfaMethod::MobileApprove]],
        )
        .await;
        let user = create_user(&pool).await;
        let device = create_device(&pool, user.id).await;
        attach_device_to_location(&pool, location.id, device.id).await;
        BiometricAuth::new(device.id, "single-step-test-key".to_owned())
            .save(&pool)
            .await
            .expect("failed to register mobile-approve key");

        let (flow_id, step_methods) = resolve_flow(&pool, location.id, user.id).await;
        let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());

        let result = engine
            .start_multi_step(
                &location,
                &device,
                &user,
                flow_id,
                step_methods,
                vec![VpnClientMfaMethod::MobileApprove],
            )
            .await
            .expect("a single-step non-TOTP/Email plan must start");
        assert!(
            matches!(result, StartResult::Accepted(_)),
            "expected an accepted plan, got a rejection"
        );
    }

    async fn start_two_step_session(
        pool: &PgPool,
        user_id: Id,
    ) -> (VpnClientMfaSession<Id>, String) {
        let location = create_mfa_location(pool).await;
        let device = create_device(pool, user_id).await;
        attach_device_to_location(pool, location.id, device.id).await;
        let mut tx = pool.begin().await.unwrap();
        let (session, outcome) = VpnClientMfaSession::<Id>::start(
            &mut tx,
            location.id,
            device.id,
            user_id,
            1,
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
            VpnClientMfaMethod::Totp,
            None,
            VPN_MFA_SESSION_TIMEOUT,
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (session, outcome.token)
    }

    async fn advance_session(pool: &PgPool, session: &VpnClientMfaSession<Id>) {
        let mut conn = pool.acquire().await.unwrap();
        session
            .advance(
                &mut conn,
                session.current_step,
                None,
                VpnClientMfaMethod::Totp,
            )
            .await
            .unwrap()
            .expect("advance should match the current step");
    }

    #[sqlx::test]
    async fn test_step_start_mints_an_id(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let _smtp = configure_working_smtp(&pool).await;

        let mut user = create_user(&pool).await;
        user.new_email_secret(&pool)
            .await
            .expect("failed to generate email secret");
        user.enable_email_mfa(&pool)
            .await
            .expect("failed to enable email MFA");
        let (session, token) = start_two_step_session(&pool, user.id).await;
        advance_session(&pool, &session).await;

        let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
        let started = engine
            .step_start(token, VpnClientMfaMethod::Email)
            .await
            .expect("step start should succeed");
        assert!(!started.step_attempt_id.is_empty());
        assert!(started.challenge.is_none());
    }

    #[sqlx::test]
    async fn test_step_start_recall_mints_fresh_id_and_resends(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");
        let smtp = configure_working_smtp(&pool).await;

        let mut user = create_user(&pool).await;
        user.new_email_secret(&pool)
            .await
            .expect("failed to generate email secret");
        user.enable_email_mfa(&pool)
            .await
            .expect("failed to enable email MFA");
        let (session, token) = start_two_step_session(&pool, user.id).await;
        advance_session(&pool, &session).await;

        let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
        let first = engine
            .step_start(token.clone(), VpnClientMfaMethod::Email)
            .await
            .expect("first step start should succeed");
        let second = engine
            .step_start(token, VpnClientMfaMethod::Email)
            .await
            .expect("second step start should succeed");

        // Ticket 05 s4: a same-method re-call is a retry, not a no-op. It supersedes the prior
        // attempt and re-runs `initiate`, which is what makes "resend the code" work. Bounding
        // re-initiation is tracked separately in DefGuard/defguard#3585.
        assert_ne!(
            first.step_attempt_id, second.step_attempt_id,
            "a re-call must mint a fresh attempt id"
        );
        smtp.wait_for_count(2).await;
        assert_eq!(
            smtp.message_count(),
            2,
            "a re-call must re-send the email so the user can request a new code"
        );
    }

    #[sqlx::test]
    async fn test_step_start_rejects_method_not_in_step(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");

        let user = create_user(&pool).await;
        let (_session, token) = start_two_step_session(&pool, user.id).await;

        let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
        let err = engine
            .step_start(token, VpnClientMfaMethod::Email)
            .await
            .expect_err("a method outside the current step must be rejected");
        let err = Status::from(err);
        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), "MFA method is not in the current step");
    }

    #[sqlx::test]
    async fn test_step_start_rejects_unconfigured_method(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");

        // Email is not configured (email_mfa_enabled is false by default).
        let user = create_user(&pool).await;
        let (session, token) = start_two_step_session(&pool, user.id).await;
        advance_session(&pool, &session).await;

        let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
        let err = engine
            .step_start(token, VpnClientMfaMethod::Email)
            .await
            .expect_err("an unconfigured method must be rejected");
        let err = Status::from(err);
        assert_eq!(err.code(), Code::FailedPrecondition);
        assert_eq!(err.message(), "MFA method is not configured for this user");
    }

    async fn setup_user_totp_and_email(pool: &PgPool, user: &mut User<Id>) {
        user.new_totp_secret(pool).await.expect("new_totp_secret");
        user.enable_totp(pool).await.expect("enable_totp");
        user.new_email_secret(pool).await.expect("new_email_secret");
        user.enable_email_mfa(pool).await.expect("enable_email_mfa");
    }

    fn totp_code(user: &User<Id>) -> String {
        let secret = user.totp_secret.as_ref().expect("totp_secret must be set");
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system time before epoch")
            .as_secs();
        totp_custom::<Sha1>(TOTP_CODE_VALIDITY_PERIOD, TOTP_CODE_DIGITS, secret, ts)
    }

    fn email_code(user: &User<Id>) -> String {
        user.generate_email_mfa_code()
            .expect("email_mfa_secret must be set")
    }

    fn test_ip() -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 7))
    }

    #[sqlx::test]
    async fn test_finish_advanced_then_completed(_: PgPoolOptions, options: PgConnectOptions) {
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
        setup_user_totp_and_email(&pool, &mut user).await;
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
            .expect("start should succeed");
        let StartResult::Accepted(outcome) = result else {
            panic!("expected an accepted plan")
        };
        let token = outcome.token;

        // Step 0 (TOTP) is not final: finish returns Advanced without authorizing.
        let (outcome, _) = engine
            .finish(
                token.clone(),
                Proof {
                    code: Some(totp_code(&user)),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                test_ip(),
            )
            .await
            .expect("finish of step 0 should succeed");
        assert_eq!(outcome, FinishOutcome::Advanced { next_step: 1 });
        assert!(
            VpnClientSession::get_all_active_device_sessions_in_location(
                &pool,
                location.id,
                device.id
            )
            .await
            .unwrap()
            .is_empty(),
            "no session may be authorized before the final step"
        );
        assert!(event_rx.try_recv().is_err());

        // Initialize and finish step 1 (Email): this completes the flow.
        engine
            .step_start(token.clone(), VpnClientMfaMethod::Email)
            .await
            .expect("step_start should succeed");
        let (outcome, _) = engine
            .finish(
                token.clone(),
                Proof {
                    code: Some(email_code(&user)),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                test_ip(),
            )
            .await
            .expect("finish of step 1 should succeed");
        let FinishOutcome::Completed { preshared_key } = outcome else {
            panic!("expected a completed flow")
        };
        assert!(!preshared_key.is_empty());

        let sessions = VpnClientSession::get_all_active_device_sessions_in_location(
            &pool,
            location.id,
            device.id,
        )
        .await
        .unwrap();
        assert_eq!(sessions.len(), 1);
        assert!(sessions[0].is_mfa_session);
        assert!(
            VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
                .await
                .unwrap()
                .is_none(),
            "the in-progress session must be deleted on completion"
        );

        // A single Success event with the ordered satisfied methods.
        let event = event_rx.try_recv().expect("expected a success event");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::Success { attribution, .. } => {
                    assert_eq!(attribution.snapshot.steps.len(), 2);
                    assert_eq!(
                        attribution.snapshot.steps[0].satisfied,
                        Some(VpnClientMfaMethod::Totp)
                    );
                    assert_eq!(
                        attribution.snapshot.steps[1].satisfied,
                        Some(VpnClientMfaMethod::Email)
                    );
                }
                other => panic!("unexpected event: {other:?}"),
            },
            other => panic!("unexpected stream event: {other:?}"),
        }
    }

    /// Regression test for the step-skip vulnerability: a proof for step 0 must never be able to
    /// satisfy step 1 as well.
    ///
    /// The original attack was a `[TOTP, Email]` flow where the attacker held only the TOTP
    /// secret and replayed one valid TOTP code twice. Both calls verified against the same
    /// ephemeral state, both advanced the cursor, the second saw `current_step == total_steps`
    /// and authorized the peer - with the Email step never proved.
    #[sqlx::test]
    async fn test_finish_replayed_proof_cannot_skip_a_step(
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
        setup_user_totp_and_email(&pool, &mut user).await;
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
            .expect("start should succeed");
        let StartResult::Accepted(outcome) = result else {
            panic!("expected an accepted plan")
        };
        let token = outcome.token;

        let code = totp_code(&user);
        let (outcome, _) = engine
            .finish(
                token.clone(),
                Proof {
                    code: Some(code.clone()),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                test_ip(),
            )
            .await
            .expect("finish of step 0 should succeed");
        assert_eq!(outcome, FinishOutcome::Advanced { next_step: 1 });

        // Replay the very same proof. It must not complete the flow.
        let err = engine
            .finish(
                token.clone(),
                Proof {
                    code: Some(code),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                test_ip(),
            )
            .await
            .expect_err("a replayed step-0 proof must not satisfy step 1");
        let err = Status::from(err);
        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), "no MFA attempt in progress");

        // The security property the status code alone does not prove: no peer was authorized.
        assert!(
            VpnClientSession::get_all_active_device_sessions_in_location(
                &pool,
                location.id,
                device.id
            )
            .await
            .unwrap()
            .is_empty(),
            "a replayed proof must not authorize a peer"
        );
        assert!(
            event_rx.try_recv().is_err(),
            "a replayed proof must not emit a success event"
        );

        // The flow is still waiting on step 1, not completed.
        let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
            .await
            .unwrap()
            .expect("the MFA session must survive a rejected replay");
        assert_eq!(session.current_step, 1);
        assert_eq!(
            session.steps_snapshot.0.steps[1].satisfied, None,
            "the Email step must remain unsatisfied"
        );
    }

    /// A proof carrying a superseded `step_attempt_id` must be rejected. Re-calling `step_start`
    /// mints a fresh attempt, and the previous one stops being spendable at that moment.
    #[sqlx::test]
    async fn test_finish_rejects_superseded_attempt_id(
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
        setup_user_totp_and_email(&pool, &mut user).await;
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
        let token = outcome.token;

        engine
            .finish(
                token.clone(),
                Proof {
                    code: Some(totp_code(&user)),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                test_ip(),
            )
            .await
            .expect("finish of step 0 should succeed");

        let first = engine
            .step_start(token.clone(), VpnClientMfaMethod::Email)
            .await
            .expect("first step_start should succeed");
        let second = engine
            .step_start(token.clone(), VpnClientMfaMethod::Email)
            .await
            .expect("second step_start should succeed");
        assert_ne!(first.step_attempt_id, second.step_attempt_id);

        // Spend the superseded attempt id: rejected even though the code itself is valid.
        let err = engine
            .finish(
                token.clone(),
                Proof {
                    code: Some(email_code(&user)),
                    auth_pub_key: None,
                    step_attempt_id: Some(first.step_attempt_id),
                },
                test_ip(),
            )
            .await
            .expect_err("a superseded attempt id must be rejected");
        let err = Status::from(err);
        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), "stale MFA attempt");

        // The current attempt still works, so the guard rejects staleness, not the method.
        let (outcome, _) = engine
            .finish(
                token,
                Proof {
                    code: Some(email_code(&user)),
                    auth_pub_key: None,
                    step_attempt_id: Some(second.step_attempt_id),
                },
                test_ip(),
            )
            .await
            .expect("the current attempt must still complete the flow");
        assert!(matches!(outcome, FinishOutcome::Completed { .. }));
    }

    #[sqlx::test]
    async fn test_finish_cap_deletes_session_and_emits_failed(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");

        let user = create_user(&pool).await;
        let (_session, token) = start_two_step_session(&pool, user.id).await;

        let (engine, mut event_rx, _gateway_rx) = make_engine(pool.clone());
        for _ in 0..MFA_FAILED_ATTEMPT_CAP {
            let err = engine
                .finish(
                    token.clone(),
                    Proof {
                        code: Some("000000".to_owned()),
                        auth_pub_key: None,
                        step_attempt_id: None,
                    },
                    test_ip(),
                )
                .await
                .expect_err("a wrong code must be rejected");
            let err = Status::from(err);
            assert_eq!(err.code(), Code::Unauthenticated);
            assert_eq!(err.message(), "unauthorized");
        }

        assert!(
            VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
                .await
                .unwrap()
                .is_none(),
            "the session must be deleted at the attempt cap"
        );

        for _ in 0..MFA_FAILED_ATTEMPT_CAP {
            let event = event_rx.try_recv().expect("expected a failed event");
            match event.event {
                BidiStreamEventType::DesktopClientMfa(event) => match *event {
                    DesktopClientMfaEvent::Failed {
                        method, message, ..
                    } => {
                        assert_eq!(method, MfaMethod::Totp);
                        assert_eq!(message, "invalid TOTP code");
                    }
                    other => panic!("unexpected event: {other:?}"),
                },
                other => panic!("unexpected stream event: {other:?}"),
            }
        }
    }

    #[sqlx::test]
    async fn test_finish_on_uninitialized_step(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool)
            .await
            .expect("failed to init settings");

        let user = create_user(&pool).await;
        let (session, token) = start_two_step_session(&pool, user.id).await;
        advance_session(&pool, &session).await;

        let (engine, _event_rx, _gateway_rx) = make_engine(pool.clone());
        let err = engine
            .finish(
                token,
                Proof {
                    code: Some("000000".to_owned()),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                test_ip(),
            )
            .await
            .expect_err("finish on an uninitialized step must be rejected");
        let err = Status::from(err);
        assert_eq!(err.code(), Code::InvalidArgument);
        assert_eq!(err.message(), "no MFA attempt in progress");
    }
}
