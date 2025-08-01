use crate::{
    db::{Id, NoId},
    random::gen_alphanumeric,
};
use base64::Engine;
use base64::engine::general_purpose;
use ed25519_dalek::Verifier;
use ed25519_dalek::{Signature, VerifyingKey};
use model_derive::Model;

#[derive(Model, Clone)]
#[table(mobile_auth)]
pub struct MobileAuth<I = NoId> {
    pub id: I,
    pub pub_key: String,
    pub device_id: Id,
}

impl MobileAuth {
    #[must_use]
    pub fn new(device_id: Id, pub_key: String) -> Self {
        Self {
            id: NoId,
            device_id,
            pub_key,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MobileChallenge {
    pub auth_pub_key: Option<String>,
    pub challenge: String,
}

impl MobileChallenge {
    #[must_use]
    pub fn new(auth_pub_key: Option<String>) -> Self {
        let challenge = gen_alphanumeric(44);
        Self {
            challenge,
            auth_pub_key,
        }
    }

    #[must_use]
    pub fn verify(self: &Self, signed_challenge: &str) -> bool {
        if let Some(auth_pub_key) = &self.auth_pub_key {
            return verify(signed_challenge, auth_pub_key.as_str(), &self.challenge);
        }
        false
    }
}

// TODO: errors
fn verify(signed_challenge: &str, public_key: &str, original_challenge: &str) -> bool {
    let pub_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(public_key)
        .unwrap()
        .try_into()
        .unwrap();
    let verifying_key = VerifyingKey::from_bytes(&pub_bytes).unwrap();
    let sig_bytes: [u8; 64] = general_purpose::STANDARD
        .decode(signed_challenge)
        .unwrap()
        .try_into()
        .unwrap();
    let signature = Signature::from_bytes(&sig_bytes);
    match verifying_key.verify(original_challenge.as_bytes(), &signature) {
        Ok(()) => true,
        Err(_) => false,
    }
}
