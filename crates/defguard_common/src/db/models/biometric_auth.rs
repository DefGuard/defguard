use base64::{Engine, engine::general_purpose::STANDARD};
use ed25519_dalek::{PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH, Signature, Verifier, VerifyingKey};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, query_as, query_scalar};
use thiserror::Error;

use crate::{
    db::{Id, NoId},
    random::gen_alphanumeric,
};

#[derive(Error, Debug)]
pub enum BiometricAuthError {
    #[error("Public key is not valid ed25519")]
    InvalidPublicKey,
    #[error("Signature invalid")]
    InvalidSignature,
    #[error("Verification of submitted challenge failed. {0}")]
    ChallengeFailed(String),
    #[error("Base64 decoding failed. {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    #[error("Challenge had no owner")]
    ChallengeNotOwned,
}

type PubKeyBytes = [u8; PUBLIC_KEY_LENGTH];
type SignatureBytes = [u8; SIGNATURE_LENGTH];

#[derive(Model)]
#[table(biometric_auth)]
pub struct BiometricAuth<I = NoId> {
    id: I,
    /// `ed25519_dalek::VerifyingKey` encoded in base64.
    pub_key: String,
    pub device_id: Id,
}

impl BiometricAuth {
    #[must_use]
    pub fn new(device_id: Id, pub_key: String) -> Self {
        Self {
            id: NoId,
            device_id,
            pub_key,
        }
    }
}

impl BiometricAuth<Id> {
    #[must_use]
    pub fn pub_key(&self) -> &str {
        self.pub_key.as_str()
    }

    pub async fn find_by_device_id<'e, E>(executor: E, device_id: Id) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, pub_key, device_id FROM biometric_auth WHERE device_id = $1",
            &device_id
        )
        .fetch_optional(executor)
        .await
    }

    /// Returns the name of device owning the given biometric auth public key, scoped to
    /// the provided user. `None` if no such device exists.
    pub async fn find_device_name<'e, E>(
        executor: E,
        user_id: Id,
        pub_key: &str,
    ) -> sqlx::Result<Option<String>>
    where
        E: PgExecutor<'e>,
    {
        query_scalar!(
            "SELECT d.name \
            FROM biometric_auth b JOIN device d ON b.device_id = d.id \
            WHERE d.user_id = $1 AND b.pub_key = $2",
            user_id,
            pub_key
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_by_user_id<'e, E>(executor: E, user_id: Id) -> sqlx::Result<Vec<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT b.id, b.pub_key, b.device_id FROM biometric_auth b \
            JOIN device d ON b.device_id = d.id WHERE d.user_id = $1",
            &user_id
        )
        .fetch_all(executor)
        .await
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct BiometricChallenge {
    pub auth_pub_key: Option<VerifyingKey>,
    pub challenge: String,
}

fn decode_pub_key(public_key: &str) -> Result<VerifyingKey, BiometricAuthError> {
    let pub_bytes: PubKeyBytes = STANDARD
        .decode(public_key)
        .map_err(|_| BiometricAuthError::InvalidPublicKey)?
        .try_into()
        .map_err(|_| BiometricAuthError::InvalidPublicKey)?;

    VerifyingKey::from_bytes(&pub_bytes).map_err(|_| BiometricAuthError::InvalidPublicKey)
}

impl Default for BiometricChallenge {
    fn default() -> Self {
        Self::new()
    }
}

impl BiometricChallenge {
    pub fn with_pubkey(pub_key: &str) -> Result<Self, BiometricAuthError> {
        let verifying_key = decode_pub_key(pub_key)?;
        let mut res = Self::new();
        res.auth_pub_key = Some(verifying_key);
        Ok(res)
    }

    #[must_use]
    pub fn new() -> Self {
        let challenge = gen_alphanumeric(44);
        Self {
            challenge,
            auth_pub_key: None,
        }
    }

    pub fn verify(
        &self,
        signed_challenge: &str,
        owner: Option<String>,
    ) -> Result<(), BiometricAuthError> {
        if let Some(auth_pub_key) = owner {
            let verifying_key = decode_pub_key(auth_pub_key.as_str())?;
            return verify(signed_challenge, &verifying_key, &self.challenge);
        }
        if let Some(verifying_key) = &self.auth_pub_key {
            return verify(signed_challenge, verifying_key, &self.challenge);
        }
        Err(BiometricAuthError::ChallengeNotOwned)
    }
}

fn verify(
    signature: &str,
    verifying_key: &VerifyingKey,
    original_challenge: &str,
) -> Result<(), BiometricAuthError> {
    let sig_bytes: SignatureBytes = STANDARD
        .decode(signature)
        .map_err(|_| BiometricAuthError::InvalidSignature)?
        .try_into()
        .map_err(|_| BiometricAuthError::InvalidSignature)?;
    let signature = Signature::from_bytes(&sig_bytes);
    verifying_key
        .verify(original_challenge.as_bytes(), &signature)
        .map_err(|_| BiometricAuthError::InvalidSignature)
}

#[cfg(test)]
mod test {
    use std::assert_matches;

    use ed25519_dalek::{Signer, SigningKey};
    use getrandom::{SysRng, rand_core::UnwrapErr};

    use super::*;

    const TEST_CHALLENGE: &str = "test-challenge";

    #[test]
    fn test_verify_valid_sig() {
        let mut csprng = UnwrapErr(SysRng);
        let signing_key = SigningKey::generate(&mut csprng);
        let signed = signing_key.sign(TEST_CHALLENGE.as_bytes());
        let serialized_signature = STANDARD.encode(signed.to_bytes());
        assert!(
            verify(
                &serialized_signature,
                &signing_key.verifying_key(),
                TEST_CHALLENGE
            )
            .is_ok()
        );
    }

    #[test]
    fn test_verify_invalid_signature() {
        let mut csprng = UnwrapErr(SysRng);
        let signing_key = SigningKey::generate(&mut csprng);
        let bad_signature = [0u8; SIGNATURE_LENGTH];
        let signature_b64 = STANDARD.encode(bad_signature);
        let result = verify(&signature_b64, &signing_key.verifying_key(), TEST_CHALLENGE);

        assert_matches!(result, Err(BiometricAuthError::InvalidSignature));
    }

    #[test]
    fn test_verify_invalid_public_key() {
        let bad_pub_key = STANDARD.encode([1, 2, 3]);

        assert_matches!(
            decode_pub_key(bad_pub_key.as_str()),
            Err(BiometricAuthError::InvalidPublicKey)
        );
    }
}
