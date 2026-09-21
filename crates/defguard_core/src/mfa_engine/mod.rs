//! Connect-time MFA engine for legacy and multi-step flows.
//!
//! The engine owns the step cursor and attempt lifecycle over the durable
//! [`VpnClientMfaSession`](defguard_common::db::models::vpn_client_mfa_session::VpnClientMfaSession)
//! store. The gRPC handlers in `grpc::proxy::client_mfa` are thin adapters converting proto
//! messages to and from the domain types here; the engine never sees a proto message.

use std::{collections::HashSet, net::IpAddr};

use defguard_common::db::{
    Id,
    models::{
        Device, User, WireguardNetwork,
        device::WireguardNetworkDevice,
        mfa_flow::MfaFlow,
        vpn_client_mfa_session::{
            EphemeralState, MFA_FAILED_ATTEMPT_CAP, MfaAttribution, MfaSessionContext, StepOutcome,
            StepsSnapshot, VPN_MFA_SESSION_TIMEOUT, VpnClientMfaSession, VpnMfaFlowKind,
        },
        vpn_client_session::VpnClientMfaMethod,
    },
    wireguard_key::WireguardKey,
};
use sqlx::{PgConnection, PgPool};
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};
use tracing::{debug, error, warn};

use crate::{
    enterprise::{db::models::openid_provider::OpenIdProvider, is_oidc_mfa_available},
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::GatewayCommand,
    mfa_engine::{
        authorize::{EventChannels, build_authorized_gateway_network_info, create_new_session},
        error::{FinishCoreError, StartError},
        method::{InitiateError, initiate, offered_credential_ids},
        types::{FinishOutcome, StartOutcome},
    },
};

pub mod authorize;
pub mod error;
pub mod legacy;
pub mod method;
pub mod multi_step;
pub mod types;

/// Owns connect-time MFA session state and final authorization.
///
/// It mints sessions, verifies proofs, advances the step cursor, and authorizes the peer at the
/// final step.
#[derive(Clone)]
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

pub(in crate::mfa_engine) struct LoadedFinishContext {
    pub(in crate::mfa_engine) session: VpnClientMfaSession<Id>,
    pub(in crate::mfa_engine) ctx: MfaSessionContext,
    pub(in crate::mfa_engine) ephemeral: EphemeralState,
    pub(in crate::mfa_engine) context: BidiRequestContext,
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

    /// Create and persist the initial MFA session and attempt.
    async fn start_session(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user: &User<Id>,
        flow_id: Id,
        steps: Vec<HashSet<VpnClientMfaMethod>>,
        flow_kind: VpnMfaFlowKind,
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
        let credential_ids = offered_credential_ids(&self.pool, &ctx, method)
            .await
            .map_err(|err| {
                error!("Failed to load FIDO2 credentials: {err}");
                StartError::Internal
            })?;

        let mut conn = self.pool.acquire().await.map_err(|_| {
            error!("Failed to acquire DB connection");
            StartError::Internal
        })?;
        let step_methods = steps.iter().map(VpnClientMfaMethod::ordered_set).collect();
        let (_session, outcome) = VpnClientMfaSession::<Id>::start(
            &mut conn,
            location.id,
            device.id,
            user.id,
            flow_id,
            step_methods,
            flow_kind,
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
            step_attempt_id: outcome.step_attempt_id,
            challenge: response_challenge,
            credential_ids,
            superseded_token_hash: outcome.superseded_token_hash,
        })
    }

    /// Resolve and snapshot OIDC availability for a new session.
    async fn oidc_available(&self) -> sqlx::Result<bool> {
        Ok(is_oidc_mfa_available(
            self.oidc_provider_configured().await?,
        ))
    }

    /// Check whether the OIDC provider still exists. Started sessions remain valid after a license
    /// lapse.
    async fn oidc_provider_configured(&self) -> sqlx::Result<bool> {
        Ok(OpenIdProvider::get_current(&self.pool).await?.is_some())
    }

    /// Load an active session only when it belongs to the contract selected by the caller.
    ///
    /// A mismatched flow is indistinguishable from an expired or unknown token. Keeping this
    /// check here protects engine callers that do not have an adapter-level route boundary.
    pub(crate) async fn find_active_session_for_flow(
        &self,
        token: &str,
        expected_flow_kind: VpnMfaFlowKind,
    ) -> sqlx::Result<Option<VpnClientMfaSession<Id>>> {
        Ok(
            VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, token)
                .await?
                .filter(|session| session.flow_kind == expected_flow_kind),
        )
    }

    pub(in crate::mfa_engine) async fn load_finish_context(
        &self,
        token: &str,
        expected_flow_kind: VpnMfaFlowKind,
        ip: IpAddr,
    ) -> Result<LoadedFinishContext, FinishCoreError> {
        let Some(session) = self
            .find_active_session_for_flow(token, expected_flow_kind)
            .await
            .map_err(|err| {
                error!("Failed to find MFA session: {err}");
                FinishCoreError::Internal
            })?
        else {
            error!("Client login session not found");
            return Err(FinishCoreError::SessionNotFound);
        };

        let Some(ctx) = session.load_context(&self.pool).await.map_err(|err| {
            error!("Failed to load MFA session context: {err}");
            FinishCoreError::Internal
        })?
        else {
            error!("MFA session references a missing location, device, or user");
            return Err(FinishCoreError::Internal);
        };

        let Some(ephemeral_state) = session.ephemeral_state.as_ref() else {
            error!("No MFA attempt in progress");
            return Err(FinishCoreError::UninitializedStep);
        };

        let ephemeral = ephemeral_state.0.clone();
        let context = BidiRequestContext::new(
            ctx.user.id,
            ctx.user.username.clone(),
            ip,
            format!("{}", ctx.device),
        );
        Ok(LoadedFinishContext {
            session,
            ctx,
            ephemeral,
            context,
        })
    }

    pub(in crate::mfa_engine) async fn advance_and_complete(
        &self,
        session: VpnClientMfaSession<Id>,
        ctx: &MfaSessionContext,
        context: BidiRequestContext,
        attempt_id: Option<&str>,
        method: VpnClientMfaMethod,
        mobile_auth_device_name: Option<&str>,
    ) -> Result<FinishOutcome, FinishCoreError> {
        let mut transaction = self.pool.begin().await.map_err(|err| {
            error!("Failed to begin transaction: {err}");
            FinishCoreError::Internal
        })?;

        let Some((advance, snapshot)) = session
            .advance(
                &mut transaction,
                session.current_step,
                attempt_id,
                method,
                mobile_auth_device_name,
            )
            .await
            .map_err(|err| {
                error!("Failed to advance MFA session: {err}");
                FinishCoreError::Internal
            })?
        else {
            error!("Stale MFA attempt: the step was already advanced or the attempt is superseded");
            return Err(FinishCoreError::StaleAttempt);
        };

        if let StepOutcome::Advanced { next_step } = advance {
            transaction.commit().await.map_err(|err| {
                error!("Failed to commit transaction while advancing MFA flow: {err}");
                FinishCoreError::Internal
            })?;
            return Ok(FinishOutcome::Advanced {
                next_step: next_step as u32,
            });
        }

        let completed = self
            .complete_flow(&mut transaction, session, snapshot, ctx, context)
            .await?;
        transaction.commit().await.map_err(|err| {
            error!("Failed to commit transaction while finishing desktop client login: {err}");
            FinishCoreError::Internal
        })?;

        debug!("Sending `peer_create` message to gateway");
        self.channels
            .gateway_tx
            .send(completed.gateway_command)
            .map_err(|err| {
                error!("Error sending WireGuard event: {err}");
                FinishCoreError::Internal
            })?;
        self.channels.emit_event(completed.event)?;
        Ok(completed.outcome)
    }

    /// Completes the flow by creating the preshared key and VPN session, then removing the MFA
    /// session. Database changes happen together; [`CompletedFlow`] carries events for dispatch
    /// after the commit.
    async fn complete_flow(
        &self,
        transaction: &mut PgConnection,
        session: VpnClientMfaSession<Id>,
        snapshot: StepsSnapshot,
        ctx: &MfaSessionContext,
        context: BidiRequestContext,
    ) -> Result<CompletedFlow, FinishCoreError> {
        let Ok(Some(network_device)) =
            WireguardNetworkDevice::find(&mut *transaction, ctx.device.id, ctx.location.id).await
        else {
            error!(
                "Failed to fetch network config for device {} and location {}",
                ctx.device, ctx.location
            );
            return Err(FinishCoreError::Internal);
        };

        let flow_name = MfaFlow::find_by_id(&mut *transaction, snapshot.flow_id)
            .await
            .map_err(|err| {
                error!("Failed to resolve MFA flow for attribution: {err}");
                FinishCoreError::Internal
            })?
            .map(|flow| flow.title);

        let key = WireguardKey::generate();

        let vpn_client_session = create_new_session(
            &self.channels,
            &mut *transaction,
            &ctx.location,
            &ctx.user,
            &ctx.device,
            true,
            key.public(),
        )
        .await
        .map_err(|err| {
            error!(
                "Failed to create new VPN client session for device {} in location {}: {err}",
                ctx.device, ctx.location
            );
            FinishCoreError::Internal
        })?;
        debug!(
            "Created new VPN client session with id {}",
            vpn_client_session.id
        );

        let gateway_network_info =
            build_authorized_gateway_network_info(network_device, key.public());

        let gateway_command = GatewayCommand::VpnSessionAuthorized(
            ctx.location.id,
            ctx.device.clone(),
            gateway_network_info,
        );

        // Use the last mobile approval's name; do not fall back to an earlier one.
        let mobile_auth_device_name = snapshot
            .steps
            .iter()
            .rev()
            .find(|step| step.satisfied == Some(VpnClientMfaMethod::MobileApprove))
            .and_then(|step| step.mobile_auth_device_name.clone());

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

        session.delete(&mut *transaction).await.map_err(|err| {
            error!("Failed to delete MFA session: {err}");
            FinishCoreError::Internal
        })?;

        Ok(CompletedFlow {
            outcome: FinishOutcome::Completed {
                preshared_key: key.public(),
            },
            gateway_command,
            event,
        })
    }

    /// Record a proof-verification failure, deleting the session once the per-step cap is reached
    /// so a subsequent finish fails closed.
    async fn record_failure(
        &self,
        session: VpnClientMfaSession<Id>,
        ctx: &MfaSessionContext,
        ip: IpAddr,
    ) -> Result<bool, FinishCoreError> {
        let mut transaction = self.pool.begin().await.map_err(|err| {
            error!("Failed to begin transaction while recording MFA failure: {err}");
            FinishCoreError::Internal
        })?;
        let at_cap = session
            .increment_failed_attempts(&mut transaction)
            .await
            .map_err(|err| {
                error!("Failed to record MFA failure: {err}");
                FinishCoreError::Internal
            })?;
        let abort_event = if at_cap {
            let flow_name = MfaFlow::find_by_id(&mut *transaction, session.steps_snapshot.flow_id)
                .await
                .map_err(|err| {
                    error!("Failed to resolve MFA flow for abort attribution: {err}");
                    FinishCoreError::Internal
                })?
                .map(|flow| flow.title);
            Some(BidiStreamEvent {
                context: BidiRequestContext::new(
                    ctx.user.id,
                    ctx.user.username.clone(),
                    ip,
                    format!("{}", ctx.device),
                ),
                event: BidiStreamEventType::DesktopClientMfa(Box::new(
                    DesktopClientMfaEvent::Aborted {
                        location: ctx.location.clone(),
                        device: ctx.device.clone(),
                        attribution: MfaAttribution {
                            snapshot: session.steps_snapshot.0.clone(),
                            flow_name,
                        },
                    },
                )),
            })
        } else {
            None
        };
        if at_cap {
            warn!(
                "MFA session {} hit the failed-attempt cap of {MFA_FAILED_ATTEMPT_CAP}; deleting it",
                session.id
            );
            session.delete(&mut *transaction).await.map_err(|err| {
                error!("Failed to delete MFA session: {err}");
                FinishCoreError::Internal
            })?;
        }
        transaction.commit().await.map_err(|err| {
            error!("Failed to commit MFA failure record: {err}");
            FinishCoreError::Internal
        })?;
        if let Some(event) = abort_event {
            self.channels.emit_event(event)?;
        }
        Ok(at_cap)
    }
}

/// Log an initiation error with the username needed by the email case.
fn log_initiate_error(err: &InitiateError, username: &str) {
    match err {
        InitiateError::EmailCode(e) => error!("Failed to generate email MFA code: {e}"),
        InitiateError::Database(e) => error!("Database error: {e}"),
        InitiateError::Mail(e) => {
            error!("Failed to send email MFA code for user {username}: {e}");
        }
        InitiateError::BiometricNotConfigured => {}
        InitiateError::InvalidPublicKey(e) => {
            error!("Start biometric MFA failed. Challenge creation failed. Reason: {e}");
        }
        InitiateError::UnsupportedMethod => {
            error!("MFA start for user {username} selected a method Core does not support");
        }
    }
}

#[cfg(test)]
mod tests;
