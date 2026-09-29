use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
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
pub(super) async fn verify(
    pool: &PgPool,
    ctx: &MfaSessionContext,
    ephemeral: &EphemeralState,
    proof: Option<&VerificationProof>,
) -> Result<Verdict, VerifyError> {
    // Key this match on the session-selected method so the client cannot choose the verifier.
    match (ephemeral.selected_method, proof) {
        (VpnClientMfaMethod::Totp, Some(VerificationProof::Code(code))) => {
            if ctx.user.verify_totp_code(code) {
                Ok(Verdict::Proved)
            } else {
                Ok(Verdict::Failed {
                    message: "invalid TOTP code",
                })
            }
        }
        (VpnClientMfaMethod::Totp, None) => Err(VerifyError::MalformedProof {
            message: "TOTP code not provided",
            event: Some("TOTP code not provided in request"),
        }),
        (VpnClientMfaMethod::Totp, Some(_)) => Err(VerifyError::MalformedProof {
            message: "MFA credential does not match the selected method",
            event: None,
        }),
        (VpnClientMfaMethod::Email, Some(VerificationProof::Code(code))) => {
            if ctx.user.verify_email_mfa_code(code) {
                Ok(Verdict::Proved)
            } else {
                Ok(Verdict::Failed {
                    message: "invalid email MFA code",
                })
            }
        }
        (VpnClientMfaMethod::Email, None) => Err(VerifyError::MalformedProof {
            message: "email MFA code not provided",
            event: Some("email MFA code not provided in request"),
        }),
        (VpnClientMfaMethod::Email, Some(_)) => Err(VerifyError::MalformedProof {
            message: "MFA credential does not match the selected method",
            event: None,
        }),
        (VpnClientMfaMethod::Biometric, Some(VerificationProof::BiometricSignature(signature))) => {
            let challenge = ephemeral
                .biometric_challenge
                .as_ref()
                .ok_or(VerifyError::MissingChallenge)?;
            match challenge.verify(signature) {
                Ok(()) => Ok(Verdict::Proved),
                Err(_) => Ok(Verdict::Failed {
                    message: "Signed challenge rejected",
                }),
            }
        }
        (VpnClientMfaMethod::Biometric, None) => {
            ephemeral
                .biometric_challenge
                .as_ref()
                .ok_or(VerifyError::MissingChallenge)?;
            Err(VerifyError::MalformedProof {
                message: "Challenge not found in request",
                event: None,
            })
        }
        (VpnClientMfaMethod::Biometric, Some(_)) => Err(VerifyError::MalformedProof {
            message: "MFA credential does not match the selected method",
            event: None,
        }),
        (VpnClientMfaMethod::Oidc, None) => {
            if ephemeral.openid_auth_completed {
                Ok(Verdict::Proved)
            } else {
                Ok(Verdict::NotYet)
            }
        }
        (VpnClientMfaMethod::Oidc, Some(_)) => Err(VerifyError::MalformedProof {
            message: "MFA credential does not match the selected method",
            event: None,
        }),
        (VpnClientMfaMethod::MobileApprove, _) => Err(VerifyError::UnsupportedMethod),
        (
            VpnClientMfaMethod::Fido2,
            Some(VerificationProof::Fido2 {
                signature,
                authenticator_data,
                credential_id,
            }),
        ) => {
            const RP_ID_HASH_LEN: usize = 32;

            let settings = Settings::get_current_settings();
            let rp_id = settings
                .webauthn_rp_id()
                .map_err(|_| VerifyError::MissingRPID)?;
            let challenge = ephemeral
                .biometric_challenge
                .as_ref()
                .ok_or(VerifyError::MissingChallenge)?;
            if authenticator_data.len() < RP_ID_HASH_LEN {
                return Err(VerifyError::MalformedProof {
                    message: "Auth data too small",
                    event: None,
                });
            }
            let rpid_hash = authenticator_data[..RP_ID_HASH_LEN].to_vec();

            let passkeys = WebAuthn::passkeys_for_user(pool, ctx.user.id).await?;

            let assertion = Assertion {
                rpid_hash,
                signature: signature.clone(),
                auth_data: authenticator_data.clone(),
                ..Default::default()
            };
            for passkey in &passkeys {
                if passkey.cred_id().as_ref() != credential_id.as_slice() {
                    continue;
                }
                let Some(public_key) = to_ctap_public_key(passkey) else {
                    continue;
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

            Ok(Verdict::Failed {
                message: "FIDO2 challenge failed",
            })
        }
        (VpnClientMfaMethod::Fido2, None) => {
            let settings = Settings::get_current_settings();
            settings
                .webauthn_rp_id()
                .map_err(|_| VerifyError::MissingRPID)?;
            ephemeral
                .biometric_challenge
                .as_ref()
                .ok_or(VerifyError::MissingChallenge)?;
            Err(VerifyError::MalformedProof {
                message: "Signature",
                event: None,
            })
        }
        (VpnClientMfaMethod::Fido2, Some(_)) => Err(VerifyError::MalformedProof {
            message: "MFA credential does not match the selected method",
            event: None,
        }),
    }
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
) -> Result<Verdict, VerifyError> {
    let challenge = ephemeral
        .biometric_challenge
        .as_ref()
        .ok_or(VerifyError::MissingChallenge)?;
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

/// Return this user's registered FIDO2 credential IDs in webauthn-rs base64url form.
pub async fn offered_credential_ids(
    pool: &PgPool,
    ctx: &MfaSessionContext,
    method: VpnClientMfaMethod,
) -> Result<Vec<String>, sqlx::Error> {
    if method != VpnClientMfaMethod::Fido2 {
        return Ok(Vec::new());
    }
    Ok(WebAuthn::passkeys_for_user(pool, ctx.user.id)
        .await?
        .iter()
        .map(|passkey| BASE64_URL_SAFE_NO_PAD.encode(passkey.cred_id()))
        .collect())
}
