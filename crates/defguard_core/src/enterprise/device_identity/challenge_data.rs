//! Wire encoding for data bound to an enrollment attestation challenge.
//!
//! Core stores the exact CBOR bytes it sends to the client so verification uses the same
//! encoding even if the serializer changes.

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

/// The challenge data bound to a device and its enrollment session.
pub struct AttestationData {
    challenge_id: String,
    expires_at: NaiveDateTime,
    /// The original CBOR encoding must survive storage and transport for proof verification.
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
    /// Rounds the existing UTC session deadline down so storage and protobuf use the same expiry.
    pub fn new(
        enrollment: &Token,
        device: &Device<Id>,
        session_timeout: Duration,
        now: NaiveDateTime,
    ) -> Result<Self, Status> {
        let expires_at = session_deadline(enrollment, session_timeout)?;
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

        // The identifier selects the pending attempt. The separate nonce makes the
        // signed context fresh even when the session and device are unchanged.
        let mut randomness = [0u8; 64];
        OsRng
            .try_fill_bytes(&mut randomness)
            .map_err(|_| Status::internal("failed to generate attestation challenge randomness"))?;
        let (identifier, nonce) = randomness.split_at(32);
        let challenge_id = BASE64_URL_SAFE_NO_PAD.encode(identifier);

        // Bind the proof to the enrollment session without putting the credential
        // itself into the bytes returned to the client.
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

pub(super) fn session_deadline(
    enrollment: &Token,
    session_timeout: Duration,
) -> Result<NaiveDateTime, Status> {
    let used_at = enrollment
        .used_at
        .ok_or_else(|| Status::unauthenticated("enrollment session has not started"))?;
    let timeout = TimeDelta::from_std(session_timeout)
        .map_err(|_| Status::internal("invalid enrollment session timeout"))?;
    used_at
        .checked_add_signed(timeout)
        .and_then(|deadline| deadline.with_nanosecond(0))
        .ok_or_else(|| Status::internal("invalid enrollment session deadline"))
}
