use defguard_common::db::models::{
    biometric_auth::{BiometricAuth, BiometricAuthError, BiometricChallenge},
    user::UserError,
    vpn_client_mfa_session::{EphemeralState, MfaSessionContext},
    vpn_client_session::VpnClientMfaMethod,
};
use sqlx::PgPool;
use thiserror::Error;

use super::types::Proof;
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

/// An error surfaced by [`verify`] that is not a proof rejection.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// A required proof field was absent. Maps to `invalid_argument` and skips the counter.
    /// `event` is the audit message to emit, `None` when the method does not audit this case.
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
        VpnClientMfaMethod::Biometric | VpnClientMfaMethod::Fido2 => {
            let Some(auth) = BiometricAuth::find_by_device_id(pool, ctx.device.id).await? else {
                return Err(InitiateError::BiometricNotConfigured);
            };
            Ok(Some(BiometricChallenge::with_pubkey(auth.pub_key())?))
        }
        VpnClientMfaMethod::MobileApprove => Ok(Some(BiometricChallenge::new())),
    }
}

/// Verify a proof against the current step's selected method.
///
/// Read-only: never mutates the session. The caller owns every mutation (failure accounting,
/// advance, delete).
pub async fn verify(
    pool: &PgPool,
    ctx: &MfaSessionContext,
    ephemeral: &EphemeralState,
    proof: &Proof,
) -> Result<Verdict, VerifyError> {
    match ephemeral.selected_method {
        VpnClientMfaMethod::Totp => {
            let code = proof.code.as_ref().ok_or(VerifyError::MalformedProof {
                message: "TOTP code not provided",
                event: Some("TOTP code not provided in request"),
            })?;
            if ctx.user.verify_totp_code(code) {
                Ok(Verdict::Proved)
            } else {
                Ok(Verdict::Failed {
                    message: "invalid TOTP code",
                })
            }
        }
        VpnClientMfaMethod::Email => {
            let code = proof.code.as_ref().ok_or(VerifyError::MalformedProof {
                message: "email MFA code not provided",
                event: Some("email MFA code not provided in request"),
            })?;
            if ctx.user.verify_email_mfa_code(code) {
                Ok(Verdict::Proved)
            } else {
                Ok(Verdict::Failed {
                    message: "invalid email MFA code",
                })
            }
        }
        VpnClientMfaMethod::Biometric | VpnClientMfaMethod::Fido2 => {
            let challenge = ephemeral
                .biometric_challenge
                .as_ref()
                .ok_or(VerifyError::MissingChallenge)?;
            let signed_challenge = proof.code.as_ref().ok_or(VerifyError::MalformedProof {
                message: "Challenge not found in request",
                event: None,
            })?;
            match challenge.verify(signed_challenge.as_str(), None) {
                Ok(()) => Ok(Verdict::Proved),
                Err(_) => Ok(Verdict::Failed {
                    message: "Signed challenge rejected",
                }),
            }
        }
        VpnClientMfaMethod::Oidc => {
            if ephemeral.openid_auth_completed {
                Ok(Verdict::Proved)
            } else {
                Ok(Verdict::NotYet)
            }
        }
        VpnClientMfaMethod::MobileApprove => {
            let challenge = ephemeral
                .biometric_challenge
                .as_ref()
                .ok_or(VerifyError::MissingChallenge)?;
            let signature = proof.code.as_ref().ok_or(VerifyError::MalformedProof {
                message: "Signature not found in request",
                event: None,
            })?;
            let auth_device_pub_key =
                proof
                    .auth_pub_key
                    .as_ref()
                    .ok_or(VerifyError::MalformedProof {
                        message: "Authorization device key missing in request",
                        event: None,
                    })?;
            // FIXME: probably not needed
            if !BiometricAuth::verify_owner(pool, ctx.user.id, auth_device_pub_key).await? {
                // A signing device not owned by the user is indistinguishable from a wrong
                // signature, so the "does this pubkey belong to user X" oracle cannot be probed
                // and the attempt is still charged by the caller.
                return Ok(Verdict::Failed {
                    message: "Signed challenge rejected",
                });
            }
            match challenge.verify(signature.as_str(), Some(auth_device_pub_key.clone())) {
                Ok(()) => Ok(Verdict::Proved),
                Err(_) => Ok(Verdict::Failed {
                    message: "Signed challenge rejected",
                }),
            }
        }
    }
}
