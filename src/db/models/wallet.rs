use super::DbPool;
use crate::hex::hex_decode;
use chrono::{NaiveDateTime, Utc};
use ethers_core::types::transaction::eip712::{Eip712, TypedData};
use model_derive::Model;
use openidconnect::Nonce;
use rocket::serde::json::serde_json;
use secp256k1::{
    ecdsa::{RecoverableSignature, RecoveryId},
    Message, Secp256k1,
};
use sqlx::{query, query_as, Error as SqlxError};
use std::{
    error::Error,
    fmt::{Display, Formatter, Result as FmtResult},
};
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug)]
pub enum Web3Error {
    Decode,
    InvalidMessage,
    InvalidRecoveryId,
    InvalidSignature,
    ParseSignature,
    Recovery,
    VerifyAddress,
}

impl Error for Web3Error {}

impl Display for Web3Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Self::Decode => write!(f, "hex decoding error"),
            Self::InvalidMessage => write!(f, "invalid message"),
            Self::InvalidRecoveryId => write!(f, "invalid recovery id"),
            Self::InvalidSignature => write!(f, "invalid signature"),
            Self::ParseSignature => write!(f, "error parsing signature"),
            Self::Recovery => write!(f, "recovery error"),
            Self::VerifyAddress => write!(f, "error veryfing address"),
        }
    }
}

/// Compute the Keccak-256 hash of input bytes.
#[must_use]
pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    hasher.finalize(&mut output);
    output
}

pub fn hash_message<S: AsRef<[u8]>>(message: S) -> [u8; 32] {
    let message = message.as_ref();
    let mut eth_message = format!("\x19Ethereum Signed Message:\n{}", message.len()).into_bytes();
    eth_message.extend_from_slice(message);
    keccak256(&eth_message)
}

#[derive(Model)]
pub struct Wallet {
    pub(crate) id: Option<i64>,
    pub(crate) user_id: i64,
    pub address: String,
    pub name: String,
    pub chain_id: i64,
    pub challenge_message: String,
    pub challenge_signature: Option<String>,
    pub creation_timestamp: NaiveDateTime,
    pub validation_timestamp: Option<NaiveDateTime>,
    pub use_for_mfa: bool,
}

impl Wallet {
    #[must_use]
    pub fn new_for_user(
        user_id: i64,
        address: String,
        name: String,
        chain_id: i64,
        challenge_message: String,
    ) -> Self {
        Self {
            id: None,
            user_id,
            address,
            name,
            chain_id,
            challenge_message,
            challenge_signature: None,
            creation_timestamp: Utc::now().naive_utc(),
            validation_timestamp: None,
            use_for_mfa: false,
        }
    }

    pub fn verify_address(&self, message: &str, signature: &str) -> Result<bool, Web3Error> {
        let address_array = hex_decode(&self.address).map_err(|_| Web3Error::Decode)?;
        let signature_array = hex_decode(signature).map_err(|_| Web3Error::Decode)?;
        let typed_data: TypedData = serde_json::from_str(message).map_err(|_| Web3Error::Decode)?;
        let hash_msg = typed_data.encode_eip712().map_err(|_| Web3Error::Decode)?;
        let message = Message::from_slice(&hash_msg).map_err(|_| Web3Error::InvalidMessage)?;
        if signature_array.len() != 65 {
            return Err(Web3Error::InvalidMessage);
        }
        let id = match signature_array[64] {
            0 | 27 => 0,
            1 | 28 => 1,
            v if v >= 35 => i32::from((v - 1) & 1),
            _ => return Err(Web3Error::InvalidRecoveryId),
        };
        let recovery_id = RecoveryId::from_i32(id).map_err(|_| Web3Error::ParseSignature)?;
        let recoverable_signature =
            RecoverableSignature::from_compact(&signature_array[0..64], recovery_id)
                .map_err(|_| Web3Error::ParseSignature)?;
        let public_key = Secp256k1::new()
            .recover_ecdsa(&message, &recoverable_signature)
            .map_err(|_| Web3Error::Recovery)?;
        let public_key = public_key.serialize_uncompressed();
        let hash = keccak256(&public_key[1..]);

        Ok(hash[12..] == address_array)
    }

    pub fn validate_signature(&self, signature: &str) -> Result<(), Web3Error> {
        if self.verify_address(&self.challenge_message, signature)? {
            Ok(())
        } else {
            Err(Web3Error::VerifyAddress)
        }
    }

    pub async fn set_signature(&mut self, pool: &DbPool, signature: &str) -> Result<(), SqlxError> {
        self.challenge_signature = Some(signature.into());
        self.validation_timestamp = Some(Utc::now().naive_utc());
        if let Some(id) = self.id {
            query!(
                "UPDATE wallet SET challenge_signature = $1, validation_timestamp = $2 WHERE id = $3",
                self.challenge_signature, self.validation_timestamp, id
            )
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    /// Prepare challenge message using EIP-712 format
    #[must_use]
    pub fn format_challenge(address: &str, challenge_message: &str) -> String {
        let nonce = Nonce::new_random();

        format!(
            r#"{{
	"domain": {{ "name": "Defguard", "version": "1" }},
        "types": {{
		"EIP712Domain": [
                    {{ "name": "name", "type": "string" }},
                    {{ "name": "version", "type": "string" }}
		],
		"ProofOfOwnership": [
                    {{ "name": "wallet", "type": "address" }},
                    {{ "name": "content", "type": "string" }},
                    {{ "name": "nonce", "type": "string" }}
		]
	}},
	"primaryType": "ProofOfOwnership",
	"message": {{
		"wallet": "{}",
		"content": "{}",
                "nonce": "{}"
	}}}}
        "#,
            address,
            challenge_message,
            nonce.secret()
        )
        .chars()
        .filter(|c| c != &'\r' && c != &'\n' && c != &'\t')
        .collect()
    }

    pub async fn find_by_user_and_address(
        pool: &DbPool,
        user_id: i64,
        address: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, address, name, chain_id, challenge_message, challenge_signature, \
            creation_timestamp, validation_timestamp, use_for_mfa FROM wallet \
            WHERE user_id = $1 AND address = $2",
            user_id,
            address
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn disable_mfa_for_user(pool: &DbPool, user_id: i64) -> Result<(), SqlxError> {
        query!(
            "UPDATE wallet SET use_for_mfa = FALSE WHERE user_id = $1",
            user_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
