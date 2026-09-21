use std::{collections::HashSet, net::IpAddr};

use defguard_common::db::{
    Id,
    models::{
        Device, Settings, User, WireguardNetwork, biometric_auth::BiometricAuth,
        vpn_client_mfa_session::VpnMfaFlowKind, vpn_client_session::VpnClientMfaMethod,
    },
};
use thiserror::Error;

use super::{
    LoadedFinishContext, MfaEngine,
    authorize::ClientMfaServerError,
    error::{FinishCoreError, StartError},
    method::{Verdict, VerifyError, verify, verify_mobile_signature},
    types::{FinishOutcome, StartOutcome, VerificationProof},
};
use crate::events::{BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent};

/// Proof fields accepted by the legacy finish contract.
#[derive(Debug, Eq, PartialEq)]
pub struct LegacyProof {
    pub code: Option<String>,
    pub auth_pub_key: Option<String>,
}

/// Errors returned by the legacy finish operation.
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

impl From<LegacyProof> for VerificationProof {
    fn from(proof: LegacyProof) -> Self {
        Self {
            code: proof.code,
            auth_pub_key: proof.auth_pub_key,
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
        // Email initiation sends asynchronously, so require SMTP configuration before starting.
        // The adapter filters unlicensed OIDC before calling the legacy path.
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
            // Keep the device-specific biometric message; other methods use the generic message.
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
        let proof: VerificationProof = proof.into();
        let loaded = self
            .load_finish_context(&token, VpnMfaFlowKind::Legacy, ip)
            .await
            .map_err(map_finish_core_error)?;
        let LoadedFinishContext {
            session,
            ctx,
            ephemeral,
            context,
        } = loaded;

        let method = ephemeral.selected_method;

        // Legacy MobileApprove requires a signature; an empty proof is not a polling request.
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

        let verdict = if method == VpnClientMfaMethod::MobileApprove {
            let signature = proof.code.as_deref().ok_or(FinishError::MalformedProof {
                message: "Signature not found in request",
            })?;
            let auth_pub_key =
                proof
                    .auth_pub_key
                    .as_deref()
                    .ok_or(FinishError::MalformedProof {
                        message: "Authorization device key missing in request",
                    })?;
            verify_mobile_signature(&self.pool, &ctx, &ephemeral, signature, auth_pub_key).await
        } else {
            verify(&self.pool, &ctx, &ephemeral, &proof).await
        };

        let mut mobile_auth_device_name = None;
        match verdict {
            Ok(Verdict::Proved) => {
                if method == VpnClientMfaMethod::MobileApprove {
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
                tracing::debug!(
                    "User {} polled MFA finish for location {} before completing OIDC authentication",
                    ctx.user.username,
                    ctx.location
                );
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
                self.record_failure(session, &ctx, ip)
                    .await
                    .map_err(map_finish_core_error)?;
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
            Err(VerifyError::UnsupportedMethod) => {
                tracing::error!("MFA method requires a contract-specific verifier");
                return Err(FinishError::Internal);
            }
        }

        let outcome = self
            .advance_and_complete(
                session,
                &ctx,
                context,
                None,
                method,
                mobile_auth_device_name
                    .as_deref()
                    .or(ephemeral.mobile_auth_device_name.as_deref()),
            )
            .await
            .map_err(map_finish_core_error)?;
        Ok((outcome, method))
    }
}

fn map_finish_core_error(error: FinishCoreError) -> FinishError {
    match error {
        FinishCoreError::SessionNotFound => FinishError::SessionNotFound,
        FinishCoreError::UninitializedStep => FinishError::UninitializedStep,
        FinishCoreError::StaleAttempt => FinishError::StaleAttempt,
        FinishCoreError::Internal => FinishError::Internal,
        FinishCoreError::Event(error) => FinishError::Event(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_proof_converts_to_verification_proof() {
        assert_eq!(
            VerificationProof::from(LegacyProof {
                code: Some("code".to_owned()),
                auth_pub_key: Some("key".to_owned()),
            }),
            VerificationProof {
                code: Some("code".to_owned()),
                auth_pub_key: Some("key".to_owned()),
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
