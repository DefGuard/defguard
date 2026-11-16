use std::time::Duration;

use base64::{
    Engine,
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD},
};
use chrono::{NaiveDateTime, TimeDelta, Timelike};
use defguard_common::db::{Id, models::Device};
use rand::{RngCore, rngs::OsRng};
use serde::{Serialize, Serializer};
use sha2::{Digest, Sha256};
use tonic::Status;

use crate::db::models::enrollment::Token;

/// Keeps the issued CBOR bytes immutable for storage, transport and verification.
pub struct AttestationData {
    challenge_id: String,
    expires_at: NaiveDateTime,
    bytes: Vec<u8>,
}

#[derive(Serialize)]
struct AttestationDataV1<'a> {
    version: u8,
    purpose: &'static str,
    challenge_id: &'a str,
    #[serde(serialize_with = "serialize_bytes")]
    session_binding: &'a [u8],
    device_id: Id,
    #[serde(serialize_with = "serialize_bytes")]
    wireguard_public_key: &'a [u8],
    #[serde(serialize_with = "serialize_bytes")]
    nonce: &'a [u8],
    expires_at: i64,
}

fn serialize_bytes<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(bytes)
}

impl AttestationData {
    /// Builds challenge data bound to an existing enrollment session and device.
    /// Uses UTC timestamps and rounds expiry down to the protobuf deadline's whole seconds.
    pub fn new(
        enrollment: &Token,
        device: &Device<Id>,
        session_timeout: Duration,
        now: NaiveDateTime,
    ) -> Result<Self, Status> {
        let used_at = enrollment
            .used_at
            .ok_or_else(|| Status::unauthenticated("enrollment session has not started"))?;
        let timeout = TimeDelta::from_std(session_timeout)
            .map_err(|_| Status::internal("invalid enrollment session timeout"))?;
        let expires_at = used_at
            .checked_add_signed(timeout)
            .and_then(|deadline| deadline.with_nanosecond(0))
            .ok_or_else(|| Status::internal("invalid enrollment session deadline"))?;
        if now >= expires_at {
            return Err(Status::unauthenticated("enrollment session has expired"));
        }
        let public_key = BASE64_STANDARD
            .decode(&device.wireguard_pubkey)
            .map_err(|_| Status::invalid_argument("invalid device WireGuard public key"))?;
        if public_key.len() != 32 {
            return Err(Status::invalid_argument(
                "invalid device WireGuard public key",
            ));
        }
        let mut randomness = [0u8; 64];
        OsRng
            .try_fill_bytes(&mut randomness)
            .map_err(|_| Status::internal("failed to generate attestation challenge randomness"))?;
        let (identifier, nonce) = randomness.split_at(32);
        let challenge_id = BASE64_URL_SAFE_NO_PAD.encode(identifier);
        let session_binding = Sha256::digest(enrollment.id.as_bytes());
        let bytes = serde_cbor::to_vec(&AttestationDataV1 {
            version: 1,
            purpose: "defguard.device-identity.attestation",
            challenge_id: &challenge_id,
            session_binding: &session_binding,
            device_id: device.id,
            wireguard_public_key: &public_key,
            nonce,
            expires_at: expires_at.and_utc().timestamp(),
        })
        .map_err(|_| Status::internal("failed to encode attestation challenge data"))?;
        Ok(Self {
            challenge_id,
            expires_at,
            bytes,
        })
    }

    pub fn challenge_id(&self) -> &str {
        &self.challenge_id
    }

    pub fn expires_at(&self) -> NaiveDateTime {
        self.expires_at
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Computes TPM qualifying data from the exact issued bytes without decoding or reserializing them.
pub fn qualifying_data(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}
