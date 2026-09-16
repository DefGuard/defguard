use std::{collections::HashSet, net::IpAddr};

use defguard_common::db::{
    Id,
    models::{
        Device, Settings, User, WireguardNetwork,
        biometric_auth::BiometricAuth,
        vpn_client_mfa_session::{VpnClientMfaSession, VpnMfaFlowKind},
        vpn_client_session::VpnClientMfaMethod,
    },
};
use thiserror::Error;

use super::{
    MfaEngine,
    authorize::ClientMfaServerError,
    error::{FinishCoreError, StartError},
    method::{Verdict, VerifyError, verify},
    types::{FinishOutcome, Proof, StartOutcome},
};
use crate::events::{
    BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent,
};

/// Proof fields accepted by the frozen legacy finish contract.
#[derive(Debug, Eq, PartialEq)]
pub struct LegacyProof {
    pub code: Option<String>,
    pub auth_pub_key: Option<String>,
}

/// Error surfaced by [`MfaEngine::finish_legacy`].
#[derive(Debug, Error)]
pub enum FinishError {
    #[error("login session not found")]
    SessionNotFound,
    #[error("no MFA attempt in progress")]
    UninitializedStep,
    #[error("OIDC authentication not completed yet")]
    OidcNotCompleted,
    #[error("unauthorized")]
    Unauthorized,
    #[error("Too many failed MFA attempts. Please try connecting again.")]
    AttemptLimit,
    #[error("stale MFA attempt")]
    StaleAttempt,
    #[error("Challenge not found in session")]
    MissingChallenge,
    #[error("Challenge not found in MFA session")]
    MissingBiometricChallenge,
    #[error("{message}")]
    MalformedProof { message: &'static str },
    #[error("unexpected error")]
    Internal,
    #[error(transparent)]
    Event(#[from] ClientMfaServerError),
}

impl From<LegacyProof> for Proof {
    fn from(proof: LegacyProof) -> Self {
        Self {
            code: proof.code,
            auth_pub_key: proof.auth_pub_key,
            step_attempt_id: None,
            auth_data: None,
            credential_id: None,
        }
    }
}

impl MfaEngine {
    /// Begin a single-step login through the frozen legacy contract.
    pub async fn start_legacy(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user: &User<Id>,
        flow_id: Id,
        steps: Vec<HashSet<VpnClientMfaMethod>>,
        selected_method: VpnClientMfaMethod,
    ) -> Result<StartOutcome, StartError> {
        // Reject a selected method the user has not set up. `is_configured` is shared with
        // `start_multi_step` and `step_start` so the paths cannot disagree.
        //
        // Email needs `smtp_configured`: `initiate` hands the code to `send_and_forget`, so a
        // send failure has no way back to the client and an unusable mailer must be caught here.
        // OIDC needs no license check here - the caller's first-step filter drops it when
        // unlicensed.
        let smtp_configured = Settings::get_current_settings().smtp_configured();
        let oidc_configured = self.oidc_available().await.map_err(|err| {
            tracing::error!("Failed to get current OpenID provider: {err}");
            StartError::Internal
        })?;
        if !selected_method
            .is_configured(
                &self.pool,
                user,
                Some(device.id),
                smtp_configured,
                oidc_configured,
            )
            .await
            .map_err(|err| {
                tracing::error!("Failed to check MFA method configuration: {err}");
                StartError::Internal
            })?
        {
            // Biometric reports a device-scoped message, the rest a generic one. Which method
            // gets which string is client-visible.
            if selected_method == VpnClientMfaMethod::Biometric {
                tracing::error!("Biometric MFA is not configured for device {}", device.id);
                return Err(StartError::BiometricNotConfigured);
            }
            tracing::error!(
                "MFA method {selected_method:?} is not configured for user {}",
                user.username
            );
            return Err(StartError::MethodNotAvailable);
        }

        self.start_session(
            location,
            device,
            user,
            flow_id,
            steps,
            VpnMfaFlowKind::Legacy,
            selected_method,
        )
        .await
    }

    /// Verify and complete a single-step login through the frozen legacy contract.
    pub async fn finish_legacy(
        &self,
        token: String,
        proof: LegacyProof,
        ip: IpAddr,
    ) -> Result<(FinishOutcome, VpnClientMfaMethod), FinishError> {
        let proof: Proof = proof.into();
        let Some(session) = VpnClientMfaSession::<Id>::find_active_by_token(&self.pool, &token)
            .await
            .map_err(|err| {
                tracing::error!("Failed to find MFA session: {err}");
                FinishError::Internal
            })?
        else {
            tracing::error!("Client login session not found");
            return Err(FinishError::SessionNotFound);
        };

        let Some(ctx) = session.load_context(&self.pool).await.map_err(|err| {
            tracing::error!("Failed to load MFA session context: {err}");
            FinishError::Internal
        })?
        else {
            tracing::error!("MFA session references a missing location, device, or user");
            return Err(FinishError::Internal);
        };

        let Some(ephemeral_state) = session.ephemeral_state.as_ref() else {
            tracing::error!("No MFA attempt in progress");
            return Err(FinishError::UninitializedStep);
        };
        let ephemeral = ephemeral_state.0.clone();
        let method = ephemeral.selected_method;

        // Legacy clients cannot poll MobileApprove with an empty proof. Preserve the existing
        // malformed-proof and missing-challenge responses.
        if method == VpnClientMfaMethod::MobileApprove
            && proof.code.is_none()
            && proof.auth_pub_key.is_none()
        {
            if ephemeral.biometric_challenge.is_none() {
                return Err(FinishError::MissingChallenge);
            }
            return Err(FinishError::MalformedProof {
                message: "Signature not found in request",
            });
        }

        let is_mobile_signature =
            super::is_mobile_approve_request(method, proof.auth_pub_key.as_deref());
        let context = BidiRequestContext::new(
            ctx.user.id,
            ctx.user.username.clone(),
            ip,
            format!("{}", ctx.device),
        );
        let verdict = verify(&self.pool, &ctx, &ephemeral, &proof).await;

        let mut mobile_auth_device_name = None;
        match verdict {
            Ok(Verdict::Proved) => {
                if is_mobile_signature {
                    let auth_pub_key = proof.auth_pub_key.as_deref().ok_or_else(|| {
                        tracing::error!(
                            "Mobile approve auth pub key missing after successful verification"
                        );
                        FinishError::Internal
                    })?;
                    mobile_auth_device_name =
                        BiometricAuth::find_device_name(&self.pool, ctx.user.id, auth_pub_key)
                            .await
                            .map_err(|err| {
                                tracing::error!(
                                    "Failed to find mobile approve device for user {}: {err}",
                                    ctx.user.id
                                );
                                FinishError::Internal
                            })?;
                }
            }
            Ok(Verdict::NotYet) => {
                // Preserve pre-2.2 OIDC behavior.
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
                let at_cap = self
                    .record_failure(session, &ctx, ip)
                    .await
                    .map_err(map_finish_core_error)?;
                if at_cap {
                    return Err(FinishError::Unauthorized);
                }
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
                tracing::error!("Failed to verify MFA proof: {err}");
                return Err(FinishError::Internal);
            }
            Err(VerifyError::MissingRPID) => {
                tracing::error!("Failed to verify FIDO2: missing RP ID");
                return Err(FinishError::Internal);
            }
        }

        let mobile_auth_device_name = mobile_auth_device_name.or(ephemeral.mobile_auth_device_name);
        let mut transaction = self.pool.begin().await.map_err(|_| {
            tracing::error!("Failed to begin transaction");
            FinishError::Internal
        })?;

        let Some((advance, snapshot)) = session
            .advance(
                &mut transaction,
                session.current_step,
                None,
                method,
                mobile_auth_device_name.as_deref(),
            )
            .await
            .map_err(|err| {
                tracing::error!("Failed to advance MFA session: {err}");
                FinishError::Internal
            })?
        else {
            tracing::error!("MFA session could not be advanced");
            return Err(FinishError::StaleAttempt);
        };
        if let defguard_common::db::models::vpn_client_mfa_session::StepOutcome::Advanced {
            next_step,
        } = advance
        {
            transaction.commit().await.map_err(|_| {
                tracing::error!("Failed to commit transaction while advancing MFA flow.");
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
            .complete_flow(&mut transaction, session, snapshot, &ctx, context)
            .await
            .map_err(map_finish_core_error)?;
        transaction.commit().await.map_err(|_| {
            tracing::error!("Failed to commit transaction while finishing desktop client login.");
            FinishError::Internal
        })?;

        tracing::debug!("Sending `peer_create` message to gateway");
        self.channels
            .gateway_tx
            .send(completed.gateway_command)
            .map_err(|err| {
                tracing::error!("Error sending WireGuard event: {err}");
                FinishError::Internal
            })?;

        tracing::info!(
            "Desktop client login finished for {} at location {} with method {method:?}",
            ctx.user.username,
            ctx.location.name
        );
        self.channels.emit_event(completed.event)?;

        Ok((completed.outcome, method))
    }
}

fn map_finish_core_error(error: FinishCoreError) -> FinishError {
    match error {
        FinishCoreError::Internal => FinishError::Internal,
        FinishCoreError::Event(error) => FinishError::Event(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_proof_converts_to_fused_proof() {
        assert_eq!(
            Proof::from(LegacyProof {
                code: Some("code".to_owned()),
                auth_pub_key: Some("key".to_owned()),
            }),
            Proof {
                code: Some("code".to_owned()),
                auth_pub_key: Some("key".to_owned()),
                step_attempt_id: None,
                auth_data: None,
                credential_id: None,
            }
        );
    }

    #[test]
    fn legacy_finish_error_messages_are_frozen() {
        let cases = [
            (FinishError::SessionNotFound, "login session not found"),
            (FinishError::UninitializedStep, "no MFA attempt in progress"),
            (
                FinishError::OidcNotCompleted,
                "OIDC authentication not completed yet",
            ),
            (FinishError::Unauthorized, "unauthorized"),
            (
                FinishError::AttemptLimit,
                "Too many failed MFA attempts. Please try connecting again.",
            ),
            (FinishError::StaleAttempt, "stale MFA attempt"),
            (
                FinishError::MissingChallenge,
                "Challenge not found in session",
            ),
            (
                FinishError::MissingBiometricChallenge,
                "Challenge not found in MFA session",
            ),
            (
                FinishError::MalformedProof {
                    message: "malformed",
                },
                "malformed",
            ),
            (FinishError::Internal, "unexpected error"),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }
}
