use chrono::NaiveDateTime;
use model_derive::Model;
use serde_json::Value;
use sqlx::{FromRow, Type};

use crate::db::{Id, NoId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Type)]
#[sqlx(type_name = "device_identity_backend", rename_all = "snake_case")]
pub enum DeviceIdentityBackend {
    Tpm,
    AppleAppAttest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Type)]
#[sqlx(type_name = "device_attestation_status", rename_all = "snake_case")]
pub enum DeviceAttestationStatus {
    NotAttested,
    Attested,
    Unrecognized,
    AttestedManually,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Type)]
#[sqlx(type_name = "device_identity_error_code", rename_all = "snake_case")]
pub enum DeviceIdentityErrorCode {
    Unavailable,
    TemporarilyUnavailable,
    Timeout,
    RateLimited,
    PermissionDenied,
    Unsupported,
    IdentityLost,
    InvalidChallenge,
    BackendFailure,
    InvalidEvidence,
    ChallengeExpired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Type)]
#[sqlx(type_name = "challenge_purpose", rename_all = "snake_case")]
pub enum DeviceIdentityChallengePurpose {
    Attestation,
    Possession,
}

#[derive(Clone, PartialEq, FromRow, Model)]
#[table(device_identity)]
pub struct DeviceIdentity<I = NoId> {
    pub id: I,
    pub device_id: Id,
    #[model(enum)]
    pub backend: DeviceIdentityBackend,
    /// Absent until Core has established the identity public key.
    pub public_key_der: Option<Vec<u8>>,
    /// SHA-256 of the SPKI DER identity key.
    pub key_fingerprint: Option<Vec<u8>>,
    /// SHA-256 of the SPKI DER endorsement key, populated only for TPM.
    pub ek_fingerprint: Option<Vec<u8>>,
    pub apple_key_id: Option<String>,
    /// Last accepted unsigned 32-bit assertion counter, stored as PostgreSQL bigint.
    pub apple_sign_count: Option<i64>,
    #[model(enum)]
    pub attestation_status: DeviceAttestationStatus,
    #[model(option)]
    pub error_code: Option<DeviceIdentityErrorCode>,
    /// Client-reported diagnostics, independent of cryptographically verified evidence.
    #[model(json)]
    pub capabilities: Option<Value>,
    /// Versioned evidence payload that the writer must encrypt before persistence.
    pub attestation_evidence: Option<Vec<u8>>,
    pub created_at: NaiveDateTime,
    pub attested_at: Option<NaiveDateTime>,
    pub last_possession_verified_at: Option<NaiveDateTime>,
}

#[derive(Clone, PartialEq, FromRow, Model)]
#[table(device_identity_challenge)]
pub struct DeviceIdentityChallenge<I = NoId> {
    pub id: I,
    #[model(enum)]
    pub purpose: DeviceIdentityChallengePurpose,
    pub device_id: Option<Id>,
    pub identity_id: Option<Id>,
    /// References the existing enrollment token, which is sensitive authentication material.
    pub enrollment_token_id: Option<String>,
    /// Exact Core-encoded bytes sent to the client.
    pub data: Vec<u8>,
    pub credential_blob: Option<Vec<u8>>,
    pub encrypted_secret: Option<Vec<u8>>,
    /// SHA-256 of the random activation secret, populated only with TPM credential fields.
    pub activation_secret_hash: Option<Vec<u8>>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub consumed_at: Option<NaiveDateTime>,
}

#[cfg(test)]
mod tests;
