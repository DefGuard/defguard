use std::{collections::HashSet, net::IpAddr};

use defguard_common::db::{
    Id,
    models::{
        Device, Settings, ThrottleScope, User, WireguardNetwork, biometric_auth::BiometricAuth,
        vpn_client_mfa_session::VpnMfaFlowKind, vpn_client_session::VpnClientMfaMethod,
    },
};
use thiserror::Error;
use tracing::{debug, error, warn};

use super::{
    LoadedFinishContext, MfaEngine,
    authorize::ClientMfaServerError,
    error::{FinishCoreError, StartError},
    filter_unlicensed_mfa_methods, is_code_method,
    method::{
        CommonVerifyError, InitiateError, LegacyCredential, Verdict, verify_legacy,
        verify_mobile_signature,
    },
    poll_allowed, throttle_key,
    types::{FinishOutcome, LegacyFinishOutcome, LegacyStartOutcome},
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

impl MfaEngine {
    /// Begin a single-step login through the frozen legacy contract.
    pub async fn start_legacy(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user: &User<Id>,
        flow_id: Id,
        step: HashSet<VpnClientMfaMethod>,
        selected_method: VpnClientMfaMethod,
    ) -> Result<LegacyStartOutcome, StartError> {
        // The legacy contract carries no assertion or credential list, so a FIDO2 session could be
        // started but never finished. Reject it here rather than relying on the caller to.
        if selected_method == VpnClientMfaMethod::Fido2 {
            error!("FIDO2 is not available through the legacy MFA contract");
            return Err(StartError::Initiate(InitiateError::UnsupportedMethod));
        }

        let step = filter_unlicensed_mfa_methods(&step);
        if !step.contains(&selected_method) {
            error!(
                "Selected MFA method ({selected_method:?}) is not supported by location \
                {location}"
            );
            return Err(StartError::MethodNotInStep);
        }

        // Email initiation sends asynchronously, so require SMTP configuration before starting.
        let smtp_configured = Settings::get_current_settings().smtp_configured();
        let oidc_configured = self.oidc_available().await.map_err(|err| {
            error!("Failed to get current OpenID provider: {err}");
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
                error!("Failed to check MFA method configuration: {err}");
                StartError::Internal
            })?
        {
            // Keep the device-specific biometric message; other methods use the generic message.
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

        let started = self
            .start_session(
                location,
                device,
                user,
                flow_id,
                vec![step],
                VpnMfaFlowKind::Legacy,
                selected_method,
            )
            .await?;

        Ok(LegacyStartOutcome {
            token: started.token,
            challenge: started.challenge,
            superseded_token_hash: started.superseded_token_hash,
        })
    }

    /// Verify and complete a single-step login through the frozen legacy contract.
    pub async fn finish_legacy(
        &self,
        token: String,
        proof: LegacyProof,
        ip: IpAddr,
    ) -> Result<(LegacyFinishOutcome, VpnClientMfaMethod), FinishError> {
        let LegacyProof { code, auth_pub_key } = proof;
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
        if method == VpnClientMfaMethod::Oidc
            && code.is_none()
            && auth_pub_key.is_none()
            && !poll_allowed(&session.token_hash)
        {
            debug!("Throttled a poll of MFA session {}", session.id);
            return Err(FinishError::OidcNotCompleted);
        }

        let charge_key = (is_code_method(method) && code.is_some())
            .then(|| throttle_key(session.location_id, session.device_id));
        if let Some(key) = &charge_key
            && !ThrottleScope::VpnMfaCode
                .hit(&self.pool, key)
                .await
                .map_err(|err| {
                    error!("Failed to charge an MFA code attempt: {err}");
                    FinishError::Internal
                })?
        {
            warn!(
                "User {} has no MFA code attempts left for device {} at location {}",
                ctx.user.username, ctx.device.id, ctx.location.name
            );
            return Err(FinishError::Unauthorized);
        }

        let credential = match method {
            VpnClientMfaMethod::Totp | VpnClientMfaMethod::Email => {
                code.clone().map(LegacyCredential::Code)
            }
            VpnClientMfaMethod::Biometric => code.clone().map(LegacyCredential::BiometricSignature),
            VpnClientMfaMethod::Oidc
            | VpnClientMfaMethod::MobileApprove
            | VpnClientMfaMethod::Fido2 => None,
        };

        // Legacy MobileApprove requires a signature; an empty proof is not a polling request.
        if method == VpnClientMfaMethod::MobileApprove && code.is_none() && auth_pub_key.is_none() {
            if ephemeral.biometric_challenge.is_none() {
                return Err(FinishError::MissingChallenge);
            }
            return Err(FinishError::MalformedProof {
                message: "Signature not found in request",
            });
        }

        let verdict = if method == VpnClientMfaMethod::MobileApprove {
            let signature = code.as_deref().ok_or(FinishError::MalformedProof {
                message: "Signature not found in request",
            })?;
            let mobile_pub_key = auth_pub_key.as_deref().ok_or(FinishError::MalformedProof {
                message: "Authorization device key missing in request",
            })?;
            verify_mobile_signature(&self.pool, &ctx, &ephemeral, signature, mobile_pub_key).await
        } else {
            verify_legacy(&ctx, &ephemeral, credential.as_ref())
        };
        if let Some(key) = &charge_key
            && matches!(&verdict, Ok(Verdict::Proved))
        {
            ThrottleScope::VpnMfaCode
                .refund(&self.pool, key)
                .await
                .map_err(|err| {
                    error!("Failed to refund an MFA code attempt: {err}");
                    FinishError::Internal
                })?;
        }

        let mut mobile_auth_device_name = None;
        match verdict {
            Ok(Verdict::Proved) => {
                if method == VpnClientMfaMethod::MobileApprove {
                    let mobile_pub_key = auth_pub_key.as_deref().ok_or_else(|| {
                        error!("Mobile approve auth pub key missing after successful verification");
                        FinishError::Internal
                    })?;
                    mobile_auth_device_name =
                        BiometricAuth::find_device_name(&self.pool, ctx.user.id, mobile_pub_key)
                            .await
                            .map_err(|err| {
                                error!(
                                    "Failed to find mobile approve device for user {}: {err}",
                                    ctx.user.id
                                );
                                FinishError::Internal
                            })?;
                }
            }
            Ok(Verdict::NotYet) => {
                debug!(
                    "User {} polled MFA finish for location {} before completing OIDC authentication",
                    ctx.user.username, ctx.location
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
            Err(CommonVerifyError::MalformedProof { message, event }) => {
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
            Err(CommonVerifyError::MissingChallenge) => {
                if method == VpnClientMfaMethod::Biometric {
                    return Err(FinishError::MissingBiometricChallenge);
                }
                return Err(FinishError::MissingChallenge);
            }
            Err(CommonVerifyError::Db(err)) => {
                error!("Failed to verify MFA proof: {err}");
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
        let outcome = match outcome {
            FinishOutcome::Completed { preshared_key } => {
                LegacyFinishOutcome::Completed { preshared_key }
            }
            FinishOutcome::AwaitingExternal => LegacyFinishOutcome::AwaitingExternal,
            FinishOutcome::Advanced { next_step } => {
                error!("Legacy MFA finish advanced unexpectedly to step {next_step}");
                return Err(FinishError::Internal);
            }
        };
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
    fn test_legacy_finish_error_messages_are_frozen() {
        let cases = [
            (FinishError::SessionNotFound, "login session not found"),
            (FinishError::UninitializedStep, "no MFA attempt in progress"),
            (
                FinishError::OidcNotCompleted,
                "OIDC authentication not completed yet",
            ),
            (FinishError::Unauthorized, "unauthorized"),
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
