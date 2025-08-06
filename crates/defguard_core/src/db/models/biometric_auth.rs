use crate::{
    db::{Id, NoId},
    random::gen_alphanumeric,
};
use base64::Engine;
use base64::engine::general_purpose;
use ed25519_dalek::Verifier;
use ed25519_dalek::{Signature, VerifyingKey};
use model_derive::Model;
use sqlx::{PgExecutor, query_as};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BiometricAuthError {
    #[error("Public key is not valid ed25519")]
    InvalidPublicKey,
    #[error("Signature invalid")]
    InvalidSignature,
    #[error("Verification of submitted challenge failed. {0}")]
    ChallengeFailed(String),
}

#[derive(Model, Clone)]
#[table(biometric_auth)]
pub struct BiometricAuth<I = NoId> {
    pub id: I,
    pub pub_key: String,
    pub device_id: Id,
}

impl BiometricAuth {
    pub fn new(device_id: Id, pub_key: String) -> Self {
        Self {
            id: NoId,
            device_id,
            pub_key,
        }
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
    let pub_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(public_key)
        .map_err(|_| BiometricAuthError::InvalidPublicKey)?
        .try_into()
        .map_err(|_| BiometricAuthError::InvalidPublicKey)?;
    let verifying_key =
        VerifyingKey::from_bytes(&pub_bytes).map_err(|_| BiometricAuthError::InvalidPublicKey)?;
    Ok(verifying_key)
}

impl BiometricChallenge {
    pub fn new(auth_pub_key: Option<String>) -> Result<Self, BiometricAuthError> {
        if let Some(pub_key) = &auth_pub_key {
            let _ = decode_pub_key(pub_key.as_str())?;
        }
        let challenge = gen_alphanumeric(44);
        Ok(Self {
            challenge,
            auth_pub_key,
        })
    }

    #[must_use]
    pub fn verify(&self, signed_challenge: &str) -> bool {
        if let Some(auth_pub_key) = &self.auth_pub_key {
            return match verify(signed_challenge, auth_pub_key.as_str(), &self.challenge) {
                Ok(res) => res,
                Err(e) => {
                    error!("Biometric auth verification failed!\n Reason: {e}");
                    false
                }
            };
        }
        false
    }
}

fn verify(
    signed_challenge: &str,
    public_key: &str,
    original_challenge: &str,
) -> Result<bool, BiometricAuthError> {
    let verifying_key = decode_pub_key(public_key)?;
    let sig_bytes: [u8; 64] = general_purpose::STANDARD
        .decode(signed_challenge)
        .map_err(|_| BiometricAuthError::InvalidSignature)?
        .try_into()
        .map_err(|_| BiometricAuthError::InvalidSignature)?;
    let signature = Signature::from_bytes(&sig_bytes);
    match verifying_key.verify(original_challenge.as_bytes(), &signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}
