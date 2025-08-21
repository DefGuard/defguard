use base64::{Engine, engine::general_purpose, prelude::BASE64_STANDARD};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use model_derive::Model;
use sqlx::{PgExecutor, query, query_as};
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

impl From<BiometricAuthError> for tonic::Status {
    fn from(value: BiometricAuthError) -> Self {
        Self::invalid_argument(value.to_string())
    }
}

#[derive(Model, Clone)]
#[table(biometric_auth)]
pub struct BiometricAuth<I = NoId> {
    pub id: I,
    pub pub_key: String,
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

    pub fn validate_pubkey(pub_key: &str) -> Result<(), BiometricAuthError> {
        let decoded = BASE64_STANDARD.decode(pub_key)?;
        if decoded.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
            return Err(BiometricAuthError::InvalidPublicKey);
        }
        Ok(())
    }
}

impl BiometricAuth<Id> {
    pub(crate) async fn find_by_device_id<'e, E>(
        executor: E,
        device_id: Id,
    ) -> Result<Option<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, pub_key, device_id FROM biometric_auth WHERE device_id=$1",
            &device_id
        )
        .fetch_optional(executor)
        .await
    }

    pub(crate) async fn verify_owner<'e, E>(
        executor: E,
        user_id: Id,
        pub_key: &str,
    ) -> Result<bool, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let q_result = query!(
            "SELECT b.id FROM biometric_auth as b JOIN device d ON b.device_id = d.id WHERE d.user_id = $1 AND b.pub_key = $2",
            user_id,
            pub_key
        )
        .fetch_optional(executor)
        .await?;
        Ok(q_result.is_some())
    }

    pub(crate) async fn find_by_user_id<'e, E>(
        executor: E,
        user_id: Id,
    ) -> Result<Vec<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT b.id, b.pub_key, b.device_id FROM biometric_auth as b JOIN device d ON b.device_id = d.id WHERE d.user_id = $1", &user_id
        )
        .fetch_all(executor)
        .await
    }
}

#[derive(Clone, Debug)]
pub struct BiometricChallenge {
    pub auth_pub_key: Option<String>,
    pub challenge: String,
}

fn decode_pub_key(public_key: &str) -> Result<VerifyingKey, BiometricAuthError> {
    let pub_bytes: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] = general_purpose::STANDARD
        .decode(public_key)
        .map_err(|_| BiometricAuthError::InvalidPublicKey)?
        .try_into()
        .map_err(|_| BiometricAuthError::InvalidPublicKey)?;
    let verifying_key =
        VerifyingKey::from_bytes(&pub_bytes).map_err(|_| BiometricAuthError::InvalidPublicKey)?;
    Ok(verifying_key)
}

impl BiometricChallenge {
    pub fn new_with_owner(pub_key: &str) -> Result<Self, BiometricAuthError> {
        let _ = decode_pub_key(pub_key)?;
        let mut res = Self::new();
        res.auth_pub_key = Some(pub_key.to_string());
        Ok(res)
    }

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
            return verify(signed_challenge, auth_pub_key.as_str(), &self.challenge);
        }
        if let Some(auth_pub_key) = &self.auth_pub_key {
            return verify(signed_challenge, auth_pub_key.as_str(), &self.challenge);
        }
        Err(BiometricAuthError::ChallengeNotOwned)
    }
}

fn verify(
    signature: &str,
    public_key: &str,
    original_challenge: &str,
) -> Result<(), BiometricAuthError> {
    let verifying_key = decode_pub_key(public_key)?;
    let sig_bytes: [u8; ed25519_dalek::SIGNATURE_LENGTH] = general_purpose::STANDARD
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
    use base64::engine::general_purpose;
    use ed25519_dalek::Signer;
    use matches::assert_matches;

    use super::*;

    #[test]
    fn test_verify_valid_sig() {
        let mut csprng = rand_core::OsRng;
        let signing_key = ed25519_dalek::SigningKey::generate(&mut csprng);
        let challenge = "test-challenge";
        let signed = signing_key.sign(challenge.as_bytes());
        let serialized_signature = BASE64_STANDARD.encode(signed.to_bytes());
        let serialized_pub_key = BASE64_STANDARD.encode(signing_key.verifying_key().as_bytes());

        assert_matches!(
            verify(&serialized_signature, &serialized_pub_key, challenge),
            Ok(())
        );
    }

    #[test]
    fn test_verify_invalid_signature() {
        let mut csprng = rand_core::OsRng;
        let signing_key = ed25519_dalek::SigningKey::generate(&mut csprng);
        let challenge = "test-challenge";

        let bad_signature = [0u8; ed25519_dalek::SIGNATURE_LENGTH];
        let signature_b64 = general_purpose::STANDARD.encode(bad_signature);
        let public_key_b64 =
            general_purpose::STANDARD.encode(signing_key.verifying_key().as_bytes());

        let result = verify(&signature_b64, &public_key_b64, challenge);

        assert_matches!(result, Err(BiometricAuthError::InvalidSignature));
    }

    #[test]
    fn test_verify_invalid_public_key() {
        let challenge = "test-challenge";
        let signature = [0u8; ed25519_dalek::SIGNATURE_LENGTH];
        let signature_b64 = general_purpose::STANDARD.encode(signature);

        let bad_pub_key = general_purpose::STANDARD.encode([1, 2, 3]);

        let result = verify(&signature_b64, &bad_pub_key, challenge);

        assert_matches!(result, Err(BiometricAuthError::InvalidPublicKey));
    }
}
