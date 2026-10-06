use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use ctap_hid_fido2::{
    fidokey::get_assertion::get_assertion_params::Assertion, verifier::verify_assertion,
};
use defguard_common::db::{
    Id,
    models::{
        Settings, ThrottleScope, WebAuthn,
        biometric_auth::{BiometricAuth, BiometricAuthError, BiometricChallenge},
        user::UserError,
        vpn_client_mfa_session::{EphemeralState, MfaSessionContext},
        vpn_client_session::VpnClientMfaMethod,
        webauthn::to_ctap_public_key,
    },
};
use sqlx::PgPool;
use thiserror::Error;

use super::types::VerificationProof;
use crate::mail::templates::{TemplateError, mfa_code_mail};

/// Outcome of proof verification.
#[derive(Debug, PartialEq)]
pub enum Verdict {
    /// The proof is valid; the step may advance.
    Proved,
    /// An out-of-band step has not resolved yet (OIDC consent not yet granted). Never counted
    /// against the attempt cap.
    NotYet,
    /// The proof was rejected. `message` is the audit message; the caller maps this to
    /// `unauthenticated` and records the failure.
    Failed { message: &'static str },
}

/// Credential submitted to a legacy finish operation.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum LegacyCredential {
    Code(String),
    BiometricSignature(String),
}

/// Errors produced by method verifiers shared by the legacy and multi-step contracts.
#[derive(Debug, Error)]
pub(super) enum CommonVerifyError {
    /// A required credential is absent or has the wrong shape. `event` is the audit message to emit.
    #[error("{message}")]
    MalformedProof {
        message: &'static str,
        event: Option<&'static str>,
    },
    /// The session's ephemeral state holds no challenge for a method that requires one.
    #[error("session holds no challenge for this method")]
    MissingChallenge,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

/// An error surfaced by [`verify`] that is not a proof rejection.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// A required credential is absent or has the wrong shape. Maps to `invalid_argument` and skips
    /// the counter. `event` is the audit message to emit, `None` when the method does not audit this
    /// case.
    #[error("{message}")]
    MalformedProof {
        message: &'static str,
        event: Option<&'static str>,
    },
    /// The session's ephemeral state holds no challenge for a method that requires one.
    #[error("session holds no challenge for this method")]
    MissingChallenge,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error("Can't build RP ID - incorrect Defguard URL")]
    MissingRPID,
    #[error("MFA method requires a contract-specific verifier")]
    UnsupportedMethod,
}

impl From<CommonVerifyError> for VerifyError {
    fn from(error: CommonVerifyError) -> Self {
        match error {
            CommonVerifyError::MalformedProof { message, event } => {
                Self::MalformedProof { message, event }
            }
            CommonVerifyError::MissingChallenge => Self::MissingChallenge,
            CommonVerifyError::Db(error) => Self::Db(error),
        }
    }
}

/// An error surfaced by [`initiate`].
#[derive(Debug, Error)]
pub enum InitiateError {
    #[error("failed to generate email MFA code")]
    EmailCode(#[from] UserError),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("failed to send email MFA code")]
    Mail(#[from] TemplateError),
    #[error("biometric auth is not configured for this device")]
    BiometricNotConfigured,
    #[error("invalid biometric public key")]
    InvalidPublicKey(#[from] BiometricAuthError),
    #[error("MFA method is not supported")]
    UnsupportedMethod,
    #[error("Too many MFA requests. Try again later.")]
    TooManyRequests,
}

/// Initiate a step: send the email code or mint the biometric / mobile-approve challenge.
///
/// Returns `None` for methods that need no challenge. The caller binds the returned challenge to a
/// fresh attempt.
pub async fn initiate(
    pool: &PgPool,
    ctx: &MfaSessionContext,
    method: VpnClientMfaMethod,
) -> Result<Option<BiometricChallenge>, InitiateError> {
    if !ThrottleScope::VpnMfaInitiate
        .hit(pool, &super::throttle_key(ctx.location.id, ctx.device.id))
        .await?
    {
        return Err(InitiateError::TooManyRequests);
    }
    match method {
        VpnClientMfaMethod::Totp | VpnClientMfaMethod::Oidc => Ok(None),
        VpnClientMfaMethod::Email => {
            let code = ctx.user.generate_email_mfa_code()?;
            let mut transaction = pool.begin().await?;
            mfa_code_mail(
                &ctx.user.email,
                &mut transaction,
                &ctx.user.first_name,
                &code,
                None,
                true,
            )
            .await?;
            Ok(None)
        }
        VpnClientMfaMethod::Biometric => {
            let Some(auth) = BiometricAuth::find_by_device_id(pool, ctx.device.id).await? else {
                return Err(InitiateError::BiometricNotConfigured);
            };
            Ok(Some(BiometricChallenge::with_pubkey(auth.pub_key())?))
        }
        VpnClientMfaMethod::MobileApprove | VpnClientMfaMethod::Fido2 => {
            Ok(Some(BiometricChallenge::new()))
        }
    }
}

fn credential_method_mismatch() -> CommonVerifyError {
    CommonVerifyError::MalformedProof {
        message: "MFA credential does not match the selected method",
        event: None,
    }
}

fn verify_totp(ctx: &MfaSessionContext, code: Option<&str>) -> Result<Verdict, CommonVerifyError> {
    let Some(code) = code else {
        return Err(CommonVerifyError::MalformedProof {
            message: "TOTP code not provided",
            event: Some("TOTP code not provided in request"),
        });
    };
    Ok(if ctx.user.verify_totp_code(code) {
        Verdict::Proved
    } else {
        Verdict::Failed {
            message: "invalid TOTP code",
        }
    })
}

fn verify_email(ctx: &MfaSessionContext, code: Option<&str>) -> Result<Verdict, CommonVerifyError> {
    let Some(code) = code else {
        return Err(CommonVerifyError::MalformedProof {
            message: "email MFA code not provided",
            event: Some("email MFA code not provided in request"),
        });
    };
    Ok(if ctx.user.verify_email_mfa_code(code) {
        Verdict::Proved
    } else {
        Verdict::Failed {
            message: "invalid email MFA code",
        }
    })
}

fn verify_biometric(
    ephemeral: &EphemeralState,
    signature: Option<&str>,
) -> Result<Verdict, CommonVerifyError> {
    let challenge = ephemeral
        .biometric_challenge
        .as_ref()
        .ok_or(CommonVerifyError::MissingChallenge)?;
    let Some(signature) = signature else {
        return Err(CommonVerifyError::MalformedProof {
            message: "Challenge not found in request",
            event: None,
        });
    };
    Ok(match challenge.verify(signature) {
        Ok(()) => Verdict::Proved,
        Err(_) => Verdict::Failed {
            message: "Signed challenge rejected",
        },
    })
}

fn verify_oidc(ephemeral: &EphemeralState) -> Verdict {
    if ephemeral.openid_auth_completed {
        Verdict::Proved
    } else {
        Verdict::NotYet
    }
}

/// Verify a proof against the current step's selected method.
///
/// Read-only: never mutates the session. The caller owns every mutation (failure accounting,
/// advance, delete).
pub(super) async fn verify(
    pool: &PgPool,
    ctx: &MfaSessionContext,
    ephemeral: &EphemeralState,
    proof: Option<&VerificationProof>,
) -> Result<Verdict, VerifyError> {
    match (ephemeral.selected_method, proof) {
        (VpnClientMfaMethod::Totp, Some(VerificationProof::Code(code))) => {
            verify_totp(ctx, Some(code)).map_err(Into::into)
        }
        (VpnClientMfaMethod::Totp, None) => verify_totp(ctx, None).map_err(Into::into),
        (VpnClientMfaMethod::Totp, Some(_)) => Err(credential_method_mismatch().into()),
        (VpnClientMfaMethod::Email, Some(VerificationProof::Code(code))) => {
            verify_email(ctx, Some(code)).map_err(Into::into)
        }
        (VpnClientMfaMethod::Email, None) => verify_email(ctx, None).map_err(Into::into),
        (VpnClientMfaMethod::Email, Some(_)) => Err(credential_method_mismatch().into()),
        (VpnClientMfaMethod::Biometric, Some(VerificationProof::BiometricSignature(signature))) => {
            verify_biometric(ephemeral, Some(signature)).map_err(Into::into)
        }
        (VpnClientMfaMethod::Biometric, None) => {
            verify_biometric(ephemeral, None).map_err(Into::into)
        }
        (VpnClientMfaMethod::Biometric, Some(_)) => Err(credential_method_mismatch().into()),
        (VpnClientMfaMethod::Oidc, None) => Ok(verify_oidc(ephemeral)),
        (VpnClientMfaMethod::Oidc, Some(_)) => Err(credential_method_mismatch().into()),
        (VpnClientMfaMethod::MobileApprove, _) => Err(VerifyError::UnsupportedMethod),
        (VpnClientMfaMethod::Fido2, proof) => {
            verify_vpn_fido2_step(pool, ctx, ephemeral, proof).await
        }
    }
}

/// Verify a legacy credential using only the method selected in the persisted session.
pub(super) fn verify_legacy(
    ctx: &MfaSessionContext,
    ephemeral: &EphemeralState,
    proof: Option<&LegacyCredential>,
) -> Result<Verdict, CommonVerifyError> {
    match (ephemeral.selected_method, proof) {
        (VpnClientMfaMethod::Totp, Some(LegacyCredential::Code(code))) => {
            verify_totp(ctx, Some(code))
        }
        (VpnClientMfaMethod::Totp, None) => verify_totp(ctx, None),
        (VpnClientMfaMethod::Totp, Some(_)) => Err(credential_method_mismatch()),
        (VpnClientMfaMethod::Email, Some(LegacyCredential::Code(code))) => {
            verify_email(ctx, Some(code))
        }
        (VpnClientMfaMethod::Email, None) => verify_email(ctx, None),
        (VpnClientMfaMethod::Email, Some(_)) => Err(credential_method_mismatch()),
        (VpnClientMfaMethod::Biometric, Some(LegacyCredential::BiometricSignature(signature))) => {
            verify_biometric(ephemeral, Some(signature))
        }
        (VpnClientMfaMethod::Biometric, None) => verify_biometric(ephemeral, None),
        (VpnClientMfaMethod::Biometric, Some(_)) => Err(credential_method_mismatch()),
        (VpnClientMfaMethod::Oidc, None) => Ok(verify_oidc(ephemeral)),
        (VpnClientMfaMethod::Oidc, Some(_)) => Err(credential_method_mismatch()),
        (VpnClientMfaMethod::MobileApprove | VpnClientMfaMethod::Fido2, _) => {
            Err(CommonVerifyError::MalformedProof {
                message: "MFA method requires a contract-specific verifier",
                event: None,
            })
        }
    }
}

async fn verify_vpn_fido2_step(
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
    if verify_registered_fido2_assertion(
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

async fn verify_registered_fido2_assertion(
    pool: &PgPool,
    user_id: Id,
    rp_id: &str,
    challenge: &str,
    assertion: &Assertion,
    credential_id: Option<&[u8]>,
) -> Result<bool, VerifyError> {
    let passkeys = WebAuthn::passkeys_for_user(pool, user_id).await?;
    for passkey in &passkeys {
        if credential_id.is_some_and(|credential_id| passkey.cred_id().as_ref() != credential_id) {
            continue;
        }
        let Some(public_key) = to_ctap_public_key(passkey) else {
            continue;
        };
        if verify_assertion(rp_id, &public_key, challenge.as_bytes(), assertion) {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Verify a FIDO2 assertion used to authorize MFA configuration.
///
/// Returns `Ok(false)` when no registered key verifies the assertion.
pub async fn verify_mfa_config_fido2_assertion(
    pool: &PgPool,
    user_id: Id,
    challenge: &str,
    signature: Option<&[u8]>,
    auth_data: Option<&[u8]>,
    credential_id: Option<&[u8]>,
) -> Result<bool, VerifyError> {
    const RP_ID_HASH_LEN: usize = 32;

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
    verify_registered_fido2_assertion(pool, user_id, &rp_id, challenge, &assertion, credential_id)
        .await
}

/// Verify a signed MobileApprove challenge.
///
/// This entry point requires both signature fields and never reads the durable approval mark.
pub(super) async fn verify_mobile_signature(
    pool: &PgPool,
    ctx: &MfaSessionContext,
    ephemeral: &EphemeralState,
    signature: &str,
    auth_device_pub_key: &str,
) -> Result<Verdict, CommonVerifyError> {
    let challenge = ephemeral
        .biometric_challenge
        .as_ref()
        .ok_or(CommonVerifyError::MissingChallenge)?;
    if !BiometricAuth::verify_owner(pool, ctx.user.id, auth_device_pub_key).await? {
        // A signing device not owned by the user is indistinguishable from a wrong signature.
        return Ok(Verdict::Failed {
            message: "Signed challenge rejected",
        });
    }
    match challenge.verify_for_owner(signature, auth_device_pub_key) {
        Ok(()) => Ok(Verdict::Proved),
        Err(_) => Ok(Verdict::Failed {
            message: "Signed challenge rejected",
        }),
    }
}

/// Read the durable MobileApprove mark for the attempt-bound polling path.
#[must_use]
pub fn check_mobile_approval(ephemeral: &EphemeralState) -> Verdict {
    if ephemeral.mobile_approved {
        Verdict::Proved
    } else {
        Verdict::NotYet
    }
}

/// Empty unless `method` is FIDO2.
pub async fn offered_credential_ids(
    pool: &PgPool,
    ctx: &MfaSessionContext,
    method: VpnClientMfaMethod,
) -> Result<Vec<String>, sqlx::Error> {
    if method != VpnClientMfaMethod::Fido2 {
        return Ok(Vec::new());
    }
    fido2_credential_ids(pool, ctx.user.id).await
}

/// Every security key credential the user has registered, base64url as webauthn-rs serializes them.
pub async fn fido2_credential_ids(pool: &PgPool, user_id: Id) -> Result<Vec<String>, sqlx::Error> {
    Ok(WebAuthn::passkeys_for_user(pool, user_id)
        .await?
        .iter()
        .map(|passkey| BASE64_URL_SAFE_NO_PAD.encode(passkey.cred_id()))
        .collect())
}
