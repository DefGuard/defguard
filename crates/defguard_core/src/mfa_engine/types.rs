//! Domain types for the MFA engine.
//!
//! These are proto-free: the conversions to and from the proto messages live in the gRPC handler
//! (`grpc::proxy::client_mfa`), so the engine can be exercised without a transport.

use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use thiserror::Error;

/// Result of `start`. `token` is returned exactly once, `challenge` is `Some` only for a method
/// the client must sign against (biometric or mobile approve), and `superseded_token_hash` names
/// the session this start replaced so the handler can cancel its waiter.
#[derive(Debug)]
pub struct StartOutcome {
    pub token: String,
    pub challenge: Option<String>,
    /// FIDO2 only: the credentials registered for this user, base64url. The
    /// client offers them to the security key, which answers for the one it
    /// holds.
    pub credential_ids: Vec<String>,
    pub superseded_token_hash: Option<String>,
}

/// Proof fields accepted by the frozen legacy finish contract.
#[derive(Debug, Eq, PartialEq)]
pub struct LegacyProof {
    pub code: Option<String>,
    pub auth_pub_key: Option<String>,
}

/// Proof fields accepted by the multi-step finish contract.
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

/// Error converting a typed proof to the transitional fused representation.
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

/// Transitional fused proof used while `finish` still serves both contracts.
#[derive(Debug, Eq, PartialEq)]
pub struct Proof {
    pub code: Option<String>,
    /// Legacy mobile public key or base64-encoded FIDO2 signature.
    pub auth_pub_key: Option<String>,
    pub step_attempt_id: Option<String>,
    /// FIDO2 authenticator data in the transitional packed representation.
    pub auth_data: Option<Vec<u8>>,
    /// FIDO2 credential selected by the client. Names the security key in use, so verification goes
    /// straight to its public key.
    pub credential_id: Option<Vec<u8>>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_and_mobile_proofs_convert_to_fused_proof() {
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

/// Result of `step_start`: the minted attempt id plus an optional biometric / mobile-approve
/// challenge.
#[derive(Debug)]
pub struct StepStarted {
    pub step_attempt_id: String,
    pub challenge: Option<String>,
    /// FIDO2 only: see [`StartOutcome::credential_ids`].
    pub credential_ids: Vec<String>,
}

/// Outcome of `finish`.
#[derive(Debug, PartialEq)]
pub enum FinishOutcome {
    /// The step just submitted advanced the flow to `next_step` (0-indexed).
    Advanced { next_step: u32 },
    /// The final step completed and a preshared key was minted.
    Completed { preshared_key: String },
    /// Still waiting for external confirmation (OIDC or mobile auth) to be completed.
    AwaitingExternal,
}

/// Why a step of the submitted plan was refused at `start`.
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

/// Result of the multi-step `start`. A refused plan creates no session, token, or event.
#[derive(Debug)]
pub enum StartResult {
    Accepted(StartOutcome),
    Rejected(Vec<StepRejection>),
}
