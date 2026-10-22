use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use ctap_hid_fido2::{
    fidokey::get_assertion::get_assertion_params::Assertion, verifier::verify_assertion,
};
use defguard_common::db::{
    Id,
    models::{
        Settings, WebAuthn,
        vpn_client_mfa_session::{EphemeralState, MfaSessionContext},
        webauthn::{AuthenticatorKind, to_ctap_public_key},
    },
};
use sqlx::PgPool;
use thiserror::Error;

use crate::{
    fido2::auth_flags::AuthFlags,
    mfa_engine::{
        method::{Verdict, VerifyError, credential_method_mismatch},
        multi_step::StepFinishError,
        types::VerificationProof,
    },
};

mod setup;

const RP_ID_HASH_LEN: usize = 32;

/// Binary FIDO2 assertion submitted to a multi-step finish.
#[derive(Debug, Eq, PartialEq)]
pub struct Fido2Assertion {
    pub rp_id_hash: Vec<u8>,
    pub authenticator_data: Vec<u8>,
    pub signature: Vec<u8>,
    pub credential_id: Vec<u8>,
}

/// Error converting a typed credential to the verifier representation.
#[derive(Debug, Eq, Error, PartialEq)]
pub enum ProofConversionError {
    #[error("FIDO2 authenticator data is too short")]
    Fido2AuthenticatorDataTooShort,
    #[error("FIDO2 RP ID hash does not match authenticator data")]
    Fido2RpIdHashMismatch,
    #[error("FIDO2 credential ID is empty")]
    Fido2CredentialIdEmpty,
    #[error("FIDO2 signature is empty")]
    Fido2SignatureEmpty,
}

impl TryFrom<Fido2Assertion> for VerificationProof {
    type Error = ProofConversionError;

    fn try_from(assertion: Fido2Assertion) -> Result<Self, Self::Error> {
        let Fido2Assertion {
            rp_id_hash,
            authenticator_data,
            signature,
            credential_id,
        } = assertion;

        if authenticator_data.len() < RP_ID_HASH_LEN {
            return Err(ProofConversionError::Fido2AuthenticatorDataTooShort);
        }
        if authenticator_data[..RP_ID_HASH_LEN] != rp_id_hash {
            return Err(ProofConversionError::Fido2RpIdHashMismatch);
        }
        if credential_id.is_empty() {
            // Unknown non-empty IDs must fail verification to prevent credential enumeration.
            return Err(ProofConversionError::Fido2CredentialIdEmpty);
        }
        if signature.is_empty() {
            return Err(ProofConversionError::Fido2SignatureEmpty);
        }

        Ok(Self::Fido2 {
            rp_id_hash,
            signature,
            authenticator_data,
            credential_id,
        })
    }
}

pub(crate) fn map_proof_conversion_error(error: ProofConversionError) -> StepFinishError {
    StepFinishError::MalformedProof {
        message: match error {
            ProofConversionError::Fido2AuthenticatorDataTooShort => {
                "FIDO2 authenticator data is too short"
            }
            ProofConversionError::Fido2RpIdHashMismatch => {
                "FIDO2 RP ID hash does not match authenticator data"
            }
            ProofConversionError::Fido2CredentialIdEmpty => "FIDO2 credential ID is empty",
            ProofConversionError::Fido2SignatureEmpty => "FIDO2 signature is empty",
        },
    }
}

pub(crate) async fn verify_vpn_fido2_step(
    pool: &PgPool,
    ctx: &MfaSessionContext,
    ephemeral: &EphemeralState,
    proof: Option<&VerificationProof>,
) -> Result<Verdict, VerifyError> {
    let assertion = match proof {
        Some(VerificationProof::Fido2 {
            rp_id_hash,
            signature,
            authenticator_data,
            credential_id,
        }) => Some((rp_id_hash, signature, authenticator_data, credential_id)),
        Some(_) => return Err(credential_method_mismatch().into()),
        None => None,
    };
    let settings = Settings::get_current_settings();
    let rp_id = settings
        .webauthn_rp_id()
        .map_err(|_| VerifyError::MissingRPID)?;
    let challenge = ephemeral
        .biometric_challenge
        .as_ref()
        .ok_or(VerifyError::MissingChallenge)?;
    let Some((rp_id_hash, signature, authenticator_data, credential_id)) = assertion else {
        return Err(VerifyError::MalformedProof {
            message: "Signature",
            event: None,
        });
    };
    let assertion = Assertion {
        rpid_hash: rp_id_hash.clone(),
        signature: signature.clone(),
        auth_data: authenticator_data.clone(),
        ..Default::default()
    };
    if verify_client_fido2_assertion(
        pool,
        ctx.user.id,
        &rp_id,
        &challenge.challenge,
        &assertion,
        Some(credential_id.as_slice()),
    )
    .await?
    {
        Ok(Verdict::Proved)
    } else {
        Ok(Verdict::Failed {
            message: "FIDO2 challenge failed",
        })
    }
}

/// Whether one of the user's hardware-attested keys signed the assertion with user presence
/// and verification, and the key is not backup eligible.
async fn verify_client_fido2_assertion(
    pool: &PgPool,
    user_id: Id,
    rp_id: &str,
    challenge: &str,
    assertion: &Assertion,
    credential_id: Option<&[u8]>,
) -> Result<bool, VerifyError> {
    let passkeys =
        WebAuthn::passkeys_for_user(pool, user_id, Some(AuthenticatorKind::Hardware)).await?;
    let signed = passkeys.iter().any(|passkey| {
        credential_id.is_none_or(|credential_id| passkey.cred_id().as_ref() == credential_id)
            && to_ctap_public_key(passkey).is_some_and(|public_key| {
                verify_assertion(rp_id, &public_key, challenge.as_bytes(), assertion)
            })
    });
    if !signed {
        return Ok(false);
    }

    // The flags are covered by the signature, so they are only trusted once it verified.
    let Some(flags) =
        AuthFlags::parse(&assertion.auth_data).filter(|flags| flags.present_and_verified())
    else {
        warn!(
            "Rejecting FIDO2 assertion for user {user_id}: user presence or verification missing"
        );
        return Ok(false);
    };
    // Registration rejects backup eligible keys, the same rule holds for every client assertion.
    if flags.backup_eligible {
        warn!("Rejecting FIDO2 assertion for user {user_id}: credential is backup eligible");
        return Ok(false);
    }
    Ok(true)
}

/// Verify a FIDO2 assertion used to authorize MFA configuration.
///
/// Returns `Ok(false)` unless a hardware-attested, non backup eligible key verifies the assertion.
pub async fn verify_mfa_config_fido2_assertion(
    pool: &PgPool,
    user_id: Id,
    challenge: &str,
    signature: Option<&[u8]>,
    auth_data: Option<&[u8]>,
    credential_id: Option<&[u8]>,
) -> Result<bool, VerifyError> {
    let settings = Settings::get_current_settings();
    let rp_id = settings
        .webauthn_rp_id()
        .map_err(|_| VerifyError::MissingRPID)?;
    let signature = signature.ok_or(VerifyError::MalformedProof {
        message: "Signature",
        event: None,
    })?;
    let auth_data = auth_data.ok_or(VerifyError::MalformedProof {
        message: "Auth data not found in request",
        event: None,
    })?;
    let rpid_hash = auth_data
        .get(..RP_ID_HASH_LEN)
        .ok_or(VerifyError::MalformedProof {
            message: "Auth data too small",
            event: None,
        })?
        .to_vec();

    let assertion = Assertion {
        rpid_hash,
        signature: signature.to_vec(),
        auth_data: auth_data.to_vec(),
        ..Default::default()
    };
    verify_client_fido2_assertion(pool, user_id, &rp_id, challenge, &assertion, credential_id).await
}

/// The user's hardware-attested security key credentials, base64url as webauthn-rs serializes
/// them. Only these keys are accepted from the client.
pub async fn fido2_credential_ids(pool: &PgPool, user_id: Id) -> Result<Vec<String>, sqlx::Error> {
    Ok(
        WebAuthn::passkeys_for_user(pool, user_id, Some(AuthenticatorKind::Hardware))
            .await?
            .iter()
            .map(|passkey| BASE64_URL_SAFE_NO_PAD.encode(passkey.cred_id()))
            .collect(),
    )
}
