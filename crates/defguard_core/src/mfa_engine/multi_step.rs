use std::{collections::HashSet, net::IpAddr};

use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use defguard_common::db::{
    Id,
    models::{
        Device, Settings, User, WireguardNetwork, biometric_auth::BiometricAuth,
        vpn_client_mfa_session::VpnMfaFlowKind, vpn_client_session::VpnClientMfaMethod,
    },
};
use thiserror::Error;
use tracing::error;

use super::{
    LoadedFinishContext, MfaEngine,
    authorize::ClientMfaServerError,
    error::{FinishCoreError, StartError},
    filter_unlicensed_mfa_methods,
    method::{Verdict, VerifyError, check_mobile_approval, verify, verify_mobile_signature},
    types::{FinishOutcome, StartOutcome, VerificationProof},
};
use crate::{
    enterprise::is_business_license_active,
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
};

/// Proof fields accepted by an attempt-bound finish.
#[derive(Debug, Eq, PartialEq)]
pub struct StepProof {
    pub step_attempt_id: String,
    pub credential: Option<StepCredential>,
}

/// Credential submitted to a multi-step finish.
#[derive(Debug, Eq, PartialEq)]
pub enum StepCredential {
    Code(String),
    BiometricSignature(String),
    Fido2(Fido2Assertion),
}

/// Binary FIDO2 assertion submitted to a multi-step finish.
#[derive(Debug, Eq, PartialEq)]
pub struct Fido2Assertion {
    pub rp_id_hash: Vec<u8>,
    pub authenticator_data: Vec<u8>,
    pub signature: Vec<u8>,
    pub credential_id: Vec<u8>,
}

/// Error converting a typed proof to the compatibility representation.
#[derive(Debug, Eq, Error, PartialEq)]
pub enum ProofConversionError {
    #[error("FIDO2 authenticator data is too short")]
    Fido2AuthenticatorDataTooShort,
    #[error("FIDO2 RP ID hash does not match authenticator data")]
    Fido2RpIdHashMismatch,
}

/// Proof used by the mark-only mobile approval operation.
#[derive(Debug, Eq, PartialEq)]
pub struct MobileApprovalProof {
    pub signature: String,
    pub auth_pub_key: String,
    pub step_attempt_id: String,
}

/// Error surfaced by [`MfaEngine::step_start`].
#[derive(Debug, Error)]
pub enum StepError {
    #[error("login session not found")]
    SessionNotFound,
    #[error("MFA method is not in the current step")]
    MethodNotInStep,
    #[error("MFA method is not configured for this user")]
    MethodNotConfigured,
    #[error("unexpected error")]
    Internal,
    #[error(transparent)]
    Initiate(#[from] super::method::InitiateError),
}

/// Errors reachable from the multi-step finish and mobile-approval methods.
#[derive(Debug, Error)]
pub enum StepFinishError {
    #[error("login session not found")]
    SessionNotFound,
    #[error("no MFA attempt in progress")]
    UninitializedStep,
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

/// Why a submitted step is rejected.
#[derive(Debug, PartialEq, Eq)]
pub enum StartRejectionReason {
    /// The chosen method is not in this step's allowed set.
    MethodNotInStep,
    /// The step has no methods left once the license filter is applied.
    StepEmptyAfterLicense,
    /// The user cannot satisfy the step. Deliberately opaque.
    StepUnavailable,
}

/// A sparse per-step rejection: only failing steps are returned.
#[derive(Debug)]
pub struct StepRejection {
    pub step: u32,
    pub reason: StartRejectionReason,
}

/// Result of a multi-step start. Rejected plans create no session, token, or event.
#[derive(Debug)]
pub enum StartResult {
    Accepted(StartOutcome),
    Rejected(Vec<StepRejection>),
}

/// Result of `step_start`: the minted attempt id plus an optional biometric / mobile-approve
/// challenge.
#[derive(Debug)]
pub struct StepStarted {
    pub step_attempt_id: String,
    pub challenge: Option<String>,
    /// FIDO2 only: see [`StartOutcome::credential_ids`].
    pub credential_ids: Vec<String>,
}

impl TryFrom<StepProof> for VerificationProof {
    type Error = ProofConversionError;

    fn try_from(proof: StepProof) -> Result<Self, Self::Error> {
        let mut normalized = Self {
            code: None,
            auth_pub_key: None,
            auth_data: None,
            credential_id: None,
        };

        match proof.credential {
            None => {}
            Some(StepCredential::Code(code) | StepCredential::BiometricSignature(code)) => {
                normalized.code = Some(code);
            }
            Some(StepCredential::Fido2(assertion)) => {
                const RP_ID_HASH_LEN: usize = 32;

                if assertion.authenticator_data.len() < RP_ID_HASH_LEN {
                    return Err(ProofConversionError::Fido2AuthenticatorDataTooShort);
                }
                if assertion.authenticator_data[..RP_ID_HASH_LEN] != assertion.rp_id_hash {
                    return Err(ProofConversionError::Fido2RpIdHashMismatch);
                }

                // The verifier's normalized representation carries the FIDO2 signature as base64
                // in `auth_pub_key`; the binary fields remain unchanged.
                normalized.auth_pub_key = Some(BASE64_URL_SAFE_NO_PAD.encode(assertion.signature));
                normalized.auth_data = Some(assertion.authenticator_data);
                normalized.credential_id = Some(assertion.credential_id);
            }
        }

        Ok(normalized)
    }
}

impl MfaEngine {
    /// Validate and start a selected multi-step plan.
    ///
    /// Rejected plans return sparse reasons without creating a session, token, or event.
    pub async fn start_multi_step(
        &self,
        location: &WireguardNetwork<Id>,
        device: &Device<Id>,
        user: &User<Id>,
        flow_id: Id,
        steps: Vec<HashSet<VpnClientMfaMethod>>,
        selected_methods: Vec<VpnClientMfaMethod>,
    ) -> Result<StartResult, StartError> {
        let business = is_business_license_active();

        // A multi-step flow requires a Business license; fail closed.
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

        let filtered_steps = steps
            .iter()
            .map(filter_unlicensed_mfa_methods)
            .collect::<Vec<HashSet<_>>>();

        let smtp_configured = Settings::get_current_settings().smtp_configured();
        let oidc_configured = self.oidc_available().await.map_err(|err| {
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
                // License filtering removed every method from this step.
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
                VpnMfaFlowKind::MultiStep,
                selected_methods[0],
            )
            .await?;
        Ok(StartResult::Accepted(outcome))
    }

    /// Initiate or reissue the current step and bind it to a fresh attempt.
    ///
    /// Reissuing a step repeats initiation and supersedes the prior attempt, so a same-method
    /// call resends the code. Callbacks for the superseded attempt are ignored.
    ///
    /// Reissuing does not change `failed_attempts`; that counter tracks rejected proofs.
    pub async fn step_start(
        &self,
        token: String,
        method: VpnClientMfaMethod,
    ) -> Result<StepStarted, StepError> {
        let Some(session) = self
            .find_active_session_for_flow(&token, VpnMfaFlowKind::MultiStep)
            .await
            .map_err(|err| {
                error!("Failed to find MFA session: {err}");
                StepError::Internal
            })?
        else {
            error!("Client login session not found");
            return Err(StepError::SessionNotFound);
        };

        if !session
            .current_step_methods()
            .is_some_and(|methods| methods.contains(&method))
        {
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
        let oidc_configured = self.oidc_provider_configured().await.map_err(|err| {
            error!("Failed to get current OpenID provider: {err}");
            StepError::Internal
        })?;
        if !method
            .is_configured(
                &self.pool,
                &ctx.user,
                Some(ctx.device.id),
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

        let challenge = super::method::initiate(&self.pool, &ctx, method)
            .await
            .map_err(|err| {
                super::log_initiate_error(&err, &ctx.user.username);
                StepError::from(err)
            })?;
        let credential_ids = super::method::offered_credential_ids(&self.pool, &ctx, method)
            .await
            .map_err(|err| {
                error!("Failed to load FIDO2 credentials: {err}");
                StepError::Internal
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
            challenge: challenge.map(|challenge| challenge.challenge),
            credential_ids,
        })
    }

    /// Verify and apply one attempt-bound multi-step proof.
    pub async fn finish_step(
        &self,
        token: String,
        proof: StepProof,
        ip: IpAddr,
    ) -> Result<FinishOutcome, StepFinishError> {
        let attempt_id = proof.step_attempt_id.clone();
        let loaded = self
            .load_finish_context(&token, VpnMfaFlowKind::MultiStep, ip)
            .await
            .map_err(map_step_finish_core_error)?;
        let LoadedFinishContext {
            session,
            ctx,
            ephemeral,
            context,
        } = loaded;

        if attempt_id != ephemeral.step_attempt_id {
            error!("Stale MFA attempt: the attempt is superseded");
            return Err(StepFinishError::StaleAttempt);
        }

        let method = ephemeral.selected_method;
        let valid_credential = matches!(
            (&proof.credential, method),
            (
                None,
                VpnClientMfaMethod::Oidc | VpnClientMfaMethod::MobileApprove
            ) | (
                Some(StepCredential::Code(_)),
                VpnClientMfaMethod::Totp | VpnClientMfaMethod::Email,
            ) | (
                Some(StepCredential::BiometricSignature(_)),
                VpnClientMfaMethod::Biometric
            ) | (Some(StepCredential::Fido2(_)), VpnClientMfaMethod::Fido2)
        );
        if !valid_credential {
            return Err(StepFinishError::MalformedProof {
                message: "MFA credential does not match the selected method",
            });
        }

        let proof = VerificationProof::try_from(proof).map_err(map_proof_conversion_error)?;
        let verdict = if method == VpnClientMfaMethod::MobileApprove {
            if proof.code.is_some() || proof.auth_pub_key.is_some() {
                return Err(StepFinishError::MalformedProof {
                    message: "Mobile approval must use the approve operation",
                });
            }
            check_mobile_approval(&ephemeral)
        } else {
            match verify(&self.pool, &ctx, &ephemeral, &proof).await {
                Ok(verdict) => verdict,
                Err(VerifyError::MalformedProof { message, event }) => {
                    if let Some(event_message) = event {
                        self.channels.emit_event(BidiStreamEvent {
                            context: BidiRequestContext::new(
                                context.user_id,
                                context.username.clone(),
                                context.ip,
                                context.device_name.clone(),
                            ),
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
                    return Err(StepFinishError::MalformedProof { message });
                }
                Err(error) => return Err(map_verify_error(method, error)),
            }
        };

        match verdict {
            Verdict::Proved => {}
            Verdict::NotYet => return Ok(FinishOutcome::AwaitingExternal),
            Verdict::Failed { message } => {
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
                if self
                    .record_failure(session, &ctx, ip)
                    .await
                    .map_err(map_step_finish_core_error)?
                {
                    return Err(StepFinishError::AttemptLimit);
                }
                return Err(StepFinishError::Unauthorized);
            }
        }

        self.advance_and_complete(
            session,
            &ctx,
            context,
            Some(&attempt_id),
            method,
            ephemeral.mobile_auth_device_name.as_deref(),
        )
        .await
        .map_err(map_step_finish_core_error)
    }

    /// Verify and durably mark a mobile approval without advancing or authorizing the flow.
    pub async fn approve_mobile_step(
        &self,
        token: String,
        proof: MobileApprovalProof,
        ip: IpAddr,
    ) -> Result<(), StepFinishError> {
        let loaded = self
            .load_finish_context(&token, VpnMfaFlowKind::MultiStep, ip)
            .await
            .map_err(map_step_finish_core_error)?;
        let LoadedFinishContext {
            session,
            ctx,
            ephemeral,
            context,
        } = loaded;

        if ephemeral.selected_method != VpnClientMfaMethod::MobileApprove {
            return Err(StepFinishError::MalformedProof {
                message: "Mobile approval is not the current MFA method",
            });
        }
        if proof.step_attempt_id != ephemeral.step_attempt_id {
            error!("Stale MFA attempt: the attempt is superseded");
            return Err(StepFinishError::StaleAttempt);
        }
        if proof.signature.is_empty() {
            return Err(StepFinishError::MalformedProof {
                message: "Signature not found in request",
            });
        }
        if proof.auth_pub_key.is_empty() {
            return Err(StepFinishError::MalformedProof {
                message: "Authorization device key missing in request",
            });
        }

        match verify_mobile_signature(
            &self.pool,
            &ctx,
            &ephemeral,
            &proof.signature,
            &proof.auth_pub_key,
        )
        .await
        .map_err(|error| map_verify_error(VpnClientMfaMethod::MobileApprove, error))?
        {
            Verdict::Proved => {
                let mobile_auth_device_name =
                    BiometricAuth::find_device_name(&self.pool, ctx.user.id, &proof.auth_pub_key)
                        .await
                        .map_err(|err| {
                            error!("Failed to find mobile approval device: {err}");
                            StepFinishError::Internal
                        })?;
                let mut transaction = self.pool.begin().await.map_err(|err| {
                    error!("Failed to begin transaction while marking mobile approval: {err}");
                    StepFinishError::Internal
                })?;
                if !session
                    .mark_mobile_approved(
                        &mut transaction,
                        &proof.step_attempt_id,
                        mobile_auth_device_name.as_deref(),
                    )
                    .await
                    .map_err(|err| {
                        error!("Failed to mark mobile approval: {err}");
                        StepFinishError::Internal
                    })?
                {
                    return Err(StepFinishError::StaleAttempt);
                }
                transaction.commit().await.map_err(|err| {
                    error!("Failed to commit mobile approval mark: {err}");
                    StepFinishError::Internal
                })?;
                Ok(())
            }
            Verdict::Failed { message } => {
                self.channels.emit_event(BidiStreamEvent {
                    context,
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: ctx.location.clone(),
                            device: ctx.device.clone(),
                            method: VpnClientMfaMethod::MobileApprove.into(),
                            message: message.to_owned(),
                        },
                    )),
                })?;
                if self
                    .record_failure(session, &ctx, ip)
                    .await
                    .map_err(map_step_finish_core_error)?
                {
                    return Err(StepFinishError::AttemptLimit);
                }
                Err(StepFinishError::Unauthorized)
            }
            Verdict::NotYet => Err(StepFinishError::Internal),
        }
    }
}

fn map_proof_conversion_error(error: ProofConversionError) -> StepFinishError {
    StepFinishError::MalformedProof {
        message: match error {
            ProofConversionError::Fido2AuthenticatorDataTooShort => {
                "FIDO2 authenticator data is too short"
            }
            ProofConversionError::Fido2RpIdHashMismatch => {
                "FIDO2 RP ID hash does not match authenticator data"
            }
        },
    }
}

fn map_verify_error(method: VpnClientMfaMethod, error: VerifyError) -> StepFinishError {
    match error {
        VerifyError::MalformedProof { message, .. } => StepFinishError::MalformedProof { message },
        VerifyError::MissingChallenge => {
            if method == VpnClientMfaMethod::Biometric {
                StepFinishError::MissingBiometricChallenge
            } else {
                StepFinishError::MissingChallenge
            }
        }
        VerifyError::Db(error) => {
            error!("Failed to verify MFA proof: {error}");
            StepFinishError::Internal
        }
        VerifyError::MissingRPID => {
            error!("Failed to verify FIDO2: missing RP ID");
            StepFinishError::Internal
        }
        VerifyError::UnsupportedMethod => StepFinishError::Internal,
    }
}

fn map_step_finish_core_error(error: FinishCoreError) -> StepFinishError {
    match error {
        FinishCoreError::SessionNotFound => StepFinishError::SessionNotFound,
        FinishCoreError::UninitializedStep => StepFinishError::UninitializedStep,
        FinishCoreError::StaleAttempt => StepFinishError::StaleAttempt,
        FinishCoreError::Internal => StepFinishError::Internal,
        FinishCoreError::Event(error) => StepFinishError::Event(error),
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::BASE64_URL_SAFE_NO_PAD;

    use super::*;

    #[test]
    fn test_step_proof_converts_structured_fido2_assertion_without_loss() {
        let rp_id_hash = vec![1; 32];
        let mut authenticator_data = rp_id_hash.clone();
        authenticator_data.extend([2, 3, 4]);
        let signature = vec![5, 6, 7];
        let credential_id = vec![8, 9, 10];
        let proof = VerificationProof::try_from(StepProof {
            step_attempt_id: "attempt".to_owned(),
            credential: Some(StepCredential::Fido2(Fido2Assertion {
                rp_id_hash: rp_id_hash.clone(),
                authenticator_data: authenticator_data.clone(),
                signature: signature.clone(),
                credential_id: credential_id.clone(),
            })),
        })
        .expect("valid FIDO2 assertion should convert");
        assert_eq!(proof.auth_data, Some(authenticator_data));
        assert_eq!(proof.credential_id, Some(credential_id));
        assert_eq!(
            BASE64_URL_SAFE_NO_PAD
                .decode(proof.auth_pub_key.expect("signature should be present"))
                .expect("signature should remain decodable"),
            signature
        );
    }

    #[test]
    fn test_step_proof_rejects_mismatched_fido2_rp_id_hash() {
        let error = VerificationProof::try_from(StepProof {
            step_attempt_id: "attempt".to_owned(),
            credential: Some(StepCredential::Fido2(Fido2Assertion {
                rp_id_hash: vec![1; 32],
                authenticator_data: vec![2; 32],
                signature: vec![3],
                credential_id: vec![4],
            })),
        })
        .expect_err("mismatched RP ID hash must be rejected");

        assert_eq!(error, ProofConversionError::Fido2RpIdHashMismatch);
    }
}
