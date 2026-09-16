use std::{collections::HashSet, net::IpAddr};

use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use defguard_common::db::{
    Id,
    models::{
        Device, Settings, User, WireguardNetwork,
        biometric_auth::BiometricAuth,
        vpn_client_mfa_session::{StepOutcome, VpnClientMfaSession, VpnMfaFlowKind},
        vpn_client_session::VpnClientMfaMethod,
    },
};
use thiserror::Error;
use tracing::{debug, error, info};

use super::{
    MfaEngine,
    authorize::ClientMfaServerError,
    error::{FinishCoreError, StartError},
    legacy::FinishError,
    method::{Verdict, VerifyError, verify},
    types::{FinishOutcome, Proof, StartOutcome},
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

impl From<MobileApprovalProof> for Proof {
    fn from(proof: MobileApprovalProof) -> Self {
        Self {
            code: Some(proof.signature),
            auth_pub_key: Some(proof.auth_pub_key),
            step_attempt_id: Some(proof.step_attempt_id),
            auth_data: None,
            credential_id: None,
        }
    }
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

impl TryFrom<StepProof> for Proof {
    type Error = ProofConversionError;

    fn try_from(proof: StepProof) -> Result<Self, Self::Error> {
        let mut fused = Self {
            code: None,
            auth_pub_key: None,
            step_attempt_id: Some(proof.step_attempt_id),
            auth_data: None,
            credential_id: None,
        };

        match proof.credential {
            None => {}
            Some(StepCredential::Code(code) | StepCredential::BiometricSignature(code)) => {
                fused.code = Some(code);
            }
            Some(StepCredential::Fido2(assertion)) => {
                const RP_ID_HASH_LEN: usize = 32;

                if assertion.authenticator_data.len() < RP_ID_HASH_LEN {
                    return Err(ProofConversionError::Fido2AuthenticatorDataTooShort);
                }
                if assertion.authenticator_data[..RP_ID_HASH_LEN] != assertion.rp_id_hash {
                    return Err(ProofConversionError::Fido2RpIdHashMismatch);
                }

                // The fused representation carries the FIDO2 signature as base64 in
                // `auth_pub_key`; the binary fields remain unchanged.
                fused.auth_pub_key = Some(BASE64_URL_SAFE_NO_PAD.encode(assertion.signature));
                fused.auth_data = Some(assertion.authenticator_data);
                fused.credential_id = Some(assertion.credential_id);
            }
        }

        Ok(fused)
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

        // Freeze the license-filtered snapshot: OIDC is a business-tier method.
        let filtered_steps = steps
            .iter()
            .map(|step| {
                step.iter()
                    .copied()
                    .filter(|method| *method != VpnClientMfaMethod::Oidc || business)
                    .collect::<HashSet<_>>()
            })
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

    /// Finish a compatibility proof using cursor checks for omitted attempt IDs and attempt checks
    /// for supplied IDs.
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
        let method = ephemeral.selected_method;

        // A supplied attempt ID must match the current attempt; an omitted ID uses cursor checks.
        if let Some(attempt_id) = proof.step_attempt_id.as_deref()
            && attempt_id != ephemeral.step_attempt_id
        {
            error!("Stale MFA attempt: the attempt is superseded");
            return Err(FinishError::StaleAttempt);
        }

        if method == VpnClientMfaMethod::MobileApprove
            && proof.step_attempt_id.is_none()
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
                        error!("Mobile approve auth pub key missing after successful verification");
                        FinishError::Internal
                    })?;
                    mobile_auth_device_name =
                        BiometricAuth::find_device_name(&self.pool, ctx.user.id, auth_pub_key)
                            .await
                            .map_err(|err| {
                                error!(
                                    "Failed to find mobile approve device for user {}: {err}",
                                    ctx.user.id
                                );
                                FinishError::Internal
                            })?;
                }
                if is_mobile_signature && let Some(attempt_id) = proof.step_attempt_id.as_deref() {
                    let mut transaction = self.pool.begin().await.map_err(|err| {
                        error!("Failed to begin transaction while marking mobile approval: {err}");
                        FinishError::Internal
                    })?;
                    if !session
                        .mark_mobile_approved(
                            &mut transaction,
                            attempt_id,
                            mobile_auth_device_name.as_deref(),
                        )
                        .await
                        .map_err(|err| {
                            error!("Failed to mark mobile approval: {err}");
                            FinishError::Internal
                        })?
                    {
                        error!("Stale MFA attempt: the attempt is superseded");
                        return Err(FinishError::StaleAttempt);
                    }
                    transaction.commit().await.map_err(|err| {
                        error!("Failed to commit mobile approval mark: {err}");
                        FinishError::Internal
                    })?;
                    return Ok((FinishOutcome::AwaitingExternal, method));
                }
            }
            Ok(Verdict::NotYet) => {
                if proof.step_attempt_id.is_some() {
                    return Ok((FinishOutcome::AwaitingExternal, method));
                }
                // The omitted-attempt form keeps the legacy OIDC response.
                self.channels.emit_event(BidiStreamEvent {
                    context,
                    event: BidiStreamEventType::DesktopClientMfa(Box::new(
                        DesktopClientMfaEvent::Failed {
                            location: ctx.location.clone(),
                            device: ctx.device.clone(),
                            method: method.into(),
                            message: "tried to finish OIDC MFA login but they haven't completed OIDC authentication yet"
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
                if at_cap && proof.step_attempt_id.is_some() {
                    return Err(FinishError::AttemptLimit);
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
                error!("Failed to verify MFA proof: {err}");
                return Err(FinishError::Internal);
            }
            Err(VerifyError::MissingRPID) => {
                error!("Failed to verify FIDO2: missing RP ID");
                return Err(FinishError::Internal);
            }
        }

        let mobile_auth_device_name = mobile_auth_device_name.or(ephemeral.mobile_auth_device_name);
        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            FinishError::Internal
        })?;

        let Some((advance, snapshot)) = session
            .advance(
                &mut transaction,
                session.current_step,
                proof.step_attempt_id.as_deref(),
                method,
                mobile_auth_device_name.as_deref(),
            )
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
            .complete_flow(&mut transaction, session, snapshot, &ctx, context)
            .await
            .map_err(map_finish_core_error)?;
        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction while finishing desktop client login.");
            FinishError::Internal
        })?;

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
}

fn map_finish_core_error(error: FinishCoreError) -> FinishError {
    match error {
        FinishCoreError::Internal => FinishError::Internal,
        FinishCoreError::Event(error) => FinishError::Event(error),
    }
}

#[cfg(test)]
mod tests {
    use base64::prelude::BASE64_URL_SAFE_NO_PAD;

    use super::*;

    #[test]
    fn mobile_approval_proof_converts_to_fused_proof() {
        assert_eq!(
            Proof::from(MobileApprovalProof {
                signature: "signature".to_owned(),
                auth_pub_key: "key".to_owned(),
                step_attempt_id: "attempt".to_owned(),
            }),
            Proof {
                code: Some("signature".to_owned()),
                auth_pub_key: Some("key".to_owned()),
                step_attempt_id: Some("attempt".to_owned()),
                auth_data: None,
                credential_id: None,
            }
        );
    }

    #[test]
    fn step_proof_converts_structured_fido2_assertion_without_loss() {
        let rp_id_hash = vec![1; 32];
        let mut authenticator_data = rp_id_hash.clone();
        authenticator_data.extend([2, 3, 4]);
        let signature = vec![5, 6, 7];
        let credential_id = vec![8, 9, 10];
        let proof = Proof::try_from(StepProof {
            step_attempt_id: "attempt".to_owned(),
            credential: Some(StepCredential::Fido2(Fido2Assertion {
                rp_id_hash: rp_id_hash.clone(),
                authenticator_data: authenticator_data.clone(),
                signature: signature.clone(),
                credential_id: credential_id.clone(),
            })),
        })
        .expect("valid FIDO2 assertion should convert");

        assert_eq!(proof.step_attempt_id, Some("attempt".to_owned()));
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
    fn step_proof_rejects_mismatched_fido2_rp_id_hash() {
        let error = Proof::try_from(StepProof {
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
