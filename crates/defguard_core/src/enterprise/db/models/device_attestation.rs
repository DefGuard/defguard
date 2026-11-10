use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Stores a pending device attestation challenge for an enrollment session.
#[derive(Clone, PartialEq, FromRow)]
pub struct DeviceAttestationState {
    pub challenge_id: String,
    pub token_id: String,
    pub wireguard_key: String,
    pub created_at: NaiveDateTime,
    /// The enrollment session deadline supplied by the caller.
    pub expires_at: NaiveDateTime,
    #[sqlx(try_from = "Vec<u8>")]
    pub payload: DeviceAttestationPayload,
}

/// CBOR payload containing the exact challenge bytes and backend verification state.
#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct DeviceAttestationPayload {
    pub data: Vec<u8>,
    pub backend: DeviceAttestationChallenge,
}

impl TryFrom<Vec<u8>> for DeviceAttestationPayload {
    type Error = serde_cbor::Error;

    fn try_from(encoded: Vec<u8>) -> Result<Self, Self::Error> {
        serde_cbor::from_slice(&encoded)
    }
}

impl DeviceAttestationState {
    /// Stores the session's pending challenge, replacing its previous state atomically.
    pub async fn save<'e, E: sqlx::PgExecutor<'e>>(&self, executor: E) -> sqlx::Result<()> {
        let payload = serde_cbor::to_vec(&self.payload)
            .map_err(|error| sqlx::Error::Encode(Box::new(error)))?;
        sqlx::query(
            "INSERT INTO device_attestation_state \
             (challenge_id, token_id, wireguard_key, created_at, expires_at, payload) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (token_id) DO UPDATE SET \
             challenge_id = EXCLUDED.challenge_id, wireguard_key = EXCLUDED.wireguard_key, \
             created_at = EXCLUDED.created_at, expires_at = EXCLUDED.expires_at, \
             payload = EXCLUDED.payload",
        )
        .bind(&self.challenge_id)
        .bind(&self.token_id)
        .bind(&self.wireguard_key)
        .bind(self.created_at)
        .bind(self.expires_at)
        .bind(payload)
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn find_by_token_id<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        token_id: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM device_attestation_state WHERE token_id = $1")
            .bind(token_id)
            .fetch_optional(executor)
            .await
    }

    /// Deletes this unexpired snapshot after verification, returning whether it was consumed.
    /// Use the device-write transaction so a rollback restores the challenge as well.
    pub async fn consume<'e, E: sqlx::PgExecutor<'e>>(&self, executor: E) -> sqlx::Result<bool> {
        let payload = serde_cbor::to_vec(&self.payload)
            .map_err(|error| sqlx::Error::Encode(Box::new(error)))?;
        let result = sqlx::query(
            "DELETE FROM device_attestation_state \
             WHERE challenge_id = $1 AND token_id = $2 AND wireguard_key = $3 \
             AND created_at = $4 AND expires_at = $5 AND payload = $6 \
             AND expires_at > (now() AT TIME ZONE 'UTC')",
        )
        .bind(&self.challenge_id)
        .bind(&self.token_id)
        .bind(&self.wireguard_key)
        .bind(self.created_at)
        .bind(self.expires_at)
        .bind(payload)
        .execute(executor)
        .await?;
        Ok(result.rows_affected() == 1)
    }
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

#[cfg(test)]
mod tests;
