use base64::{
    Engine, alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    prelude::BASE64_URL_SAFE_NO_PAD,
};
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
            // The client sends binary as base64url, matching how webauthn-rs
            // writes the credential ids it was offered.
            let rpid_hash = decode_proof_field(proof.code.as_ref(), "RP ID hash")?;
            let signature = decode_proof_field(proof.auth_pub_key.as_ref(), "Signature")?;
            let auth_data = proof
                .auth_data
                .as_ref()
                .ok_or(VerifyError::MalformedProof {
                    message: "Auth data not found in request",
                    event: None,
                })?;

            // The key names the credential it signed with, so verification goes
            // straight to that public key. A client that names none - a pre-FIDO2
            // build - falls back to trying every registered key; one that names a
            // credential this user does not own matches nothing and fails.
            let passkeys = WebAuthn::passkeys_for_user(pool, ctx.user.id).await?;
            let named = proof
                .credential_id
                .as_deref()
                .and_then(|credential_id| decode_base64(credential_id).ok());

            let assertion = Assertion {
                rpid_hash,
                signature,
                auth_data: auth_data.clone(),
                ..Default::default()
            };
            for passkey in &passkeys {
                // Skip the keys the client did not name, if it named one.
                if named.as_ref().is_some_and(|credential_id| {
                    passkey.cred_id().as_ref() != credential_id.as_slice()
                }) {
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
    }
}

/// Decode a base64 value the client sent as part of a FIDO2 proof.
///
/// webauthn-rs writes binary as URL-safe base64 without padding and reads
/// either alphabet, padded or not; be equally forgiving rather than assuming
/// one.
fn decode_base64(value: &str) -> Result<Vec<u8>, base64::DecodeError> {
    /// Padding is accepted but not required, so one engine covers both the
    /// padded and unpadded spelling of its alphabet.
    fn engine(alphabet: alphabet::Alphabet) -> GeneralPurpose {
        GeneralPurpose::new(
            &alphabet,
            GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
        )
    }

    engine(alphabet::URL_SAFE)
        .decode(value)
        .or_else(|err| engine(alphabet::STANDARD).decode(value).map_err(|_| err))
}

/// Decode a required base64 field of a FIDO2 proof, naming it if it is absent
/// or malformed.
fn decode_proof_field(value: Option<&String>, field: &'static str) -> Result<Vec<u8>, VerifyError> {
    let value = value.ok_or(VerifyError::MalformedProof {
        message: field,
        event: None,
    })?;
    decode_base64(value).map_err(|_| VerifyError::MalformedProof {
        message: field,
        event: None,
    })
}

/// The credentials to offer the security key: every one this user has
/// registered, base64url as webauthn-rs serializes them. Only FIDO2 needs them.
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

#[cfg(test)]
mod tests {
    use base64::prelude::{BASE64_STANDARD, BASE64_STANDARD_NO_PAD, BASE64_URL_SAFE};

    use super::*;

    #[test]
    fn test_decode_base64_accepts_every_alphabet() {
        // Bytes whose url-safe encoding (`_-`) differs from the standard one
        // (`/+`), so a decoder locked to one alphabet fails the other.
        let raw = vec![0xff_u8, 0xfe, 0xfd, 0x00];

        for encoded in [
            // What the desktop client sends, matching webauthn-rs.
            BASE64_URL_SAFE_NO_PAD.encode(&raw),
            BASE64_URL_SAFE.encode(&raw),
            BASE64_STANDARD.encode(&raw),
            BASE64_STANDARD_NO_PAD.encode(&raw),
        ] {
            assert_eq!(
                decode_base64(&encoded).expect("should decode"),
                raw,
                "failed to decode {encoded}"
            );
        }
    }

    #[test]
    fn test_decode_proof_field_names_a_missing_or_malformed_value() {
        assert!(matches!(
            decode_proof_field(None, "RP ID hash"),
            Err(VerifyError::MalformedProof {
                message: "RP ID hash",
                ..
            })
        ));
        assert!(matches!(
            decode_proof_field(Some(&"not base64!!".to_string()), "Signature"),
            Err(VerifyError::MalformedProof {
                message: "Signature",
                ..
            })
        ));
    }
}
