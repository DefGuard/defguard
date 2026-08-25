//! Connect-time multi-step MFA engine.
//!
//! The engine owns the step cursor and attempt lifecycle over the durable
//! [`VpnClientMfaSession`](defguard_common::db::models::vpn_client_mfa_session::VpnClientMfaSession)
//! store. The gRPC handlers in `grpc::proxy::client_mfa` are thin adapters converting proto
//! messages to and from the domain types here; the engine never sees a proto message.

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
/// Mints the session, verifies proofs, advances the step cursor, and - at the final step -
/// authorizes the peer, sends the gateway command, and emits the audit events.
///
/// License gating happens only at `start`, which freezes the license-filtered step snapshot;
/// `step_start` and `finish` carry no license gate, so an in-flight flow runs to completion even if
/// the license lapses mid-flow.
pub struct MfaEngine {
    pool: PgPool,
    channels: EventChannels,
}

/// The side effects of a completed flow, built inside the transaction but dispatched by the
/// caller **after** it commits.
///
/// Neither the gateway authorization nor the success event can be rolled back. Dispatched inside
/// the transaction, a failed commit would leave the gateway holding an authorized peer with no
/// session row to show for it. Dispatched after, a failure surfaces as an error rather than as
/// silent access.
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
        // OIDC needs no license check here - the caller's first-step filter drops it when
        // unlicensed.
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
            } else if !chosen
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
    /// OpenID provider. Must stay in step with the descriptor builder's source for
    /// `oidc_configured`.
    async fn oidc_configured(&self) -> sqlx::Result<bool> {
        if !is_business_license_active() {
            return Ok(false);
        }
        Ok(OpenIdProvider::get_current(&self.pool).await?.is_some())
    }

    /// Initiate the current step: send the email code or mint the challenge and bind it to a
    /// fresh attempt.
    ///
    /// There is no branch for an already-initialized step: a re-call is a legal switch to a
    /// different method or a retry of the same one, and either way it re-runs `initiate` and mints
    /// a fresh attempt id, which is what makes "resend the code" work. The abandoned attempt's
    /// side effects are not cancelled; stale callbacks no-op on the superseded attempt id.
    ///
    /// A re-call does not touch `failed_attempts` - that counter bounds wrong proofs, not
    /// initialization. Bounding re-initiation is tracked in DefGuard/defguard#3585.
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
        // for, so a stale or duplicate advance matches zero rows. A non-final advance returns
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
    /// Everything here is transactional. The gateway command and the success event are not, so
    /// they are returned in [`CompletedFlow`] for the caller to dispatch after the commit.
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

/// Log an [`InitiateError`] with the context it needs, so `start` and `step_start` can each wrap it
/// into their own error type without duplicating the logging.
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
mod tests;
