use ctap_hid_fido2::{
    fidokey::get_assertion::get_assertion_params::Assertion, verifier::verify_assertion,
};
use defguard_common::db::models::{
    Settings, WebAuthn,
    biometric_auth::{BiometricAuth, BiometricAuthError, BiometricChallenge},
    user::UserError,
    vpn_client_mfa_session::{EphemeralState, MfaSessionContext},
    vpn_client_session::VpnClientMfaMethod,
    webauthn::to_ctap_public_key,
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
    #[error("Can't build RP ID - incorrect Defguard URL")]
    MissingRPID,
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
        VpnClientMfaMethod::Biometric => {
            let challenge = ephemeral
                .biometric_challenge
                .as_ref()
                .ok_or(VerifyError::MissingChallenge)?;
            let signed_challenge = proof.code.as_ref().ok_or(VerifyError::MalformedProof {
                message: "Challenge not found in request",
                event: None,
            })?;
            match challenge.verify(signed_challenge) {
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
            match challenge.verify_for_owner(signature, auth_device_pub_key) {
                Ok(()) => Ok(Verdict::Proved),
                Err(_) => Ok(Verdict::Failed {
                    message: "Signed challenge rejected",
                }),
            }
        }
        VpnClientMfaMethod::Fido2 => {
            let settings = Settings::get_current_settings();
            let rp_id = settings
                .webauthn_rp_id()
                .map_err(|_| VerifyError::MissingRPID)?;
            let challenge = ephemeral
                .biometric_challenge
                .as_ref()
                .ok_or(VerifyError::MissingChallenge)?;
            let code = proof.code.as_ref().ok_or(VerifyError::MalformedProof {
                message: "RP ID hash not found in request",
                event: None,
            })?;
            let auth_pub_key = proof
                .auth_pub_key
                .as_ref()
                .ok_or(VerifyError::MalformedProof {
                    message: "Signature not found in request",
                    event: None,
                })?;
            let auth_data = proof
                .auth_data
                .as_ref()
                .ok_or(VerifyError::MalformedProof {
                    message: "Auth data not found in request",
                    event: None,
                })?;

            // Fetch WebAuthN passkeys and try to verify FIDO2 with them.
            let passkeys = WebAuthn::passkeys_for_user(pool, ctx.user.id).await?;
            for passkey in passkeys {
                if let Some(public_key) = to_ctap_public_key(&passkey) {
                    let assertion = Assertion {
                        rpid_hash: code.as_bytes().to_vec(),
                        signature: auth_pub_key.as_bytes().to_vec(),
                        auth_data: auth_data.clone(),
                        ..Default::default()
                    };
                    if verify_assertion(
                        &rp_id,
                        &public_key,
                        challenge.challenge.as_bytes(),
                        &assertion,
                    ) {
                        return Ok(Verdict::Proved);
                    }
                }
            }

            Ok(Verdict::Failed {
                message: "FIDO2 challenge failed",
            })
        }
    }
}
