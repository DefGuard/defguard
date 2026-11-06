use chrono::NaiveDateTime;
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};

use crate::db::{Id, NoId};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[sqlx(type_name = "device_identity_backend", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DeviceIdentityBackend {
    Tpm,
    AppleAppAttest,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[sqlx(type_name = "device_attestation_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DeviceAttestationStatus {
    #[default]
    NotAttested,
    Attested,
    Unrecognized,
    AttestedManually,
    AttestationFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Type)]
#[sqlx(type_name = "challenge_purpose", rename_all = "snake_case")]
pub enum DeviceIdentityChallengePurpose {
    Polling,
    PostureConnection,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct DeviceAttestationState {
    pub challenge_id: String,
    pub data: Vec<u8>,
    pub wireguard_key: String,
    pub backend: DeviceAttestationChallenge,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub enum DeviceAttestationChallenge {
    Tpm {
        ak_name: Vec<u8>,
        ek_fingerprint: [u8; 32],
        activation_secret_hash: [u8; 32],
    },
    AppleAppAttest {
        key_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum DevicePossessionState {
    Pending { challenge_id: String, data: Vec<u8> },
    Verified { key_fingerprint: [u8; 32] },
}

#[derive(Clone, PartialEq, FromRow, Model)]
#[table(device_identity_challenge)]
pub struct DeviceIdentityChallenge<I = NoId> {
    pub id: I,
    #[model(enum)]
    pub purpose: DeviceIdentityChallengePurpose,
    pub device_id: Id,
    pub location_id: Option<Id>,
    /// Exact Core-encoded bytes include the nonce, key fingerprint and operation binding.
    pub data: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub consumed_at: Option<NaiveDateTime>,
}

impl DeviceIdentityChallenge<Id> {
    pub async fn consume<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        id: Id,
        device_id: Id,
        purpose: DeviceIdentityChallengePurpose,
        location_id: Option<Id>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            "UPDATE device_identity_challenge SET consumed_at = (now() AT TIME ZONE 'UTC') \
             WHERE id = $1 AND device_id = $2 AND purpose = $3 \
             AND location_id IS NOT DISTINCT FROM $4 AND consumed_at IS NULL \
             AND expires_at > (now() AT TIME ZONE 'UTC') RETURNING *",
        )
        .bind(id)
        .bind(device_id)
        .bind(purpose)
        .bind(location_id)
        .fetch_optional(executor)
        .await
    }
}

#[cfg(test)]
mod tests;
