use chrono::{NaiveDateTime, Utc};
use defguard_common::db::{Id, NoId, models::vpn_client_mfa_session::VpnClientMfaSession};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{Connection, FromRow, PgConnection, Type};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Type)]
#[sqlx(type_name = "challenge_purpose", rename_all = "snake_case")]
pub enum DeviceIdentityChallengePurpose {
    Polling,
    PostureConnection,
    MfaConnection,
}

#[derive(Clone, PartialEq, FromRow, Model)]
#[table(device_identity_challenge)]
pub struct DeviceIdentityChallenge<I = NoId> {
    pub id: I,
    #[model(enum)]
    pub purpose: DeviceIdentityChallengePurpose,
    pub device_id: Id,
    pub location_id: Option<Id>,
    pub mfa_session_id: Option<Id>,
    /// Captures the session generation, concurrent Start can replace a session while keeping its ID.
    pub mfa_session_token_hash: Option<String>,
    /// Exact Core-encoded bytes include the nonce, key fingerprint and operation binding.
    pub data: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    pub consumed_at: Option<NaiveDateTime>,
}

impl DeviceIdentityChallenge<NoId> {
    /// Binds a fresh MFA challenge to the session context, token generation and deadline.
    pub fn for_mfa(session: &VpnClientMfaSession<Id>, data: Vec<u8>) -> Self {
        Self {
            id: NoId,
            purpose: DeviceIdentityChallengePurpose::MfaConnection,
            device_id: session.device_id,
            location_id: Some(session.location_id),
            mfa_session_id: Some(session.id),
            mfa_session_token_hash: Some(session.token_hash.clone()),
            data,
            created_at: Utc::now().naive_utc(),
            expires_at: session.expires_at,
            consumed_at: None,
        }
    }
}

impl DeviceIdentityChallenge<Id> {
    /// Atomically consumes a live challenge bound to the device, purpose, location and MFA session.
    /// Wall-clock expiry checks also apply inside long-running transactions.
    pub async fn consume<'e, E: sqlx::PgExecutor<'e>>(
        executor: E,
        id: Id,
        device_id: Id,
        purpose: DeviceIdentityChallengePurpose,
        location_id: Option<Id>,
        mfa_session_id: Option<Id>,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            "UPDATE device_identity_challenge AS challenge SET consumed_at = (clock_timestamp() AT TIME ZONE 'UTC') \
             WHERE id = $1 AND device_id = $2 AND purpose = $3 \
             AND location_id IS NOT DISTINCT FROM $4 AND mfa_session_id IS NOT DISTINCT FROM $5 \
             AND consumed_at IS NULL AND expires_at > (clock_timestamp() AT TIME ZONE 'UTC') \
             AND (mfa_session_id IS NULL OR EXISTS ( \
                 SELECT 1 FROM vpn_client_mfa_session AS session \
                 WHERE session.id = challenge.mfa_session_id \
                 AND session.token_hash = challenge.mfa_session_token_hash \
                 AND session.expires_at > (clock_timestamp() AT TIME ZONE 'UTC') \
                 AND challenge.expires_at <= session.expires_at \
             )) RETURNING challenge.*",
        )
        .bind(id)
        .bind(device_id)
        .bind(purpose)
        .bind(location_id)
        .bind(mfa_session_id)
        .fetch_optional(executor)
        .await
    }

    /// Records an already verified MFA proof and consumes its challenge in one transaction.
    /// Call after verifying the proof with the device's registered identity key.
    pub async fn record_verified_mfa_possession(
        connection: &mut PgConnection,
        challenge_id: Id,
        session: &VpnClientMfaSession<Id>,
    ) -> sqlx::Result<bool> {
        let mut transaction = connection.begin().await?;
        let updated = sqlx::query(
            "UPDATE vpn_client_mfa_session SET possession_verified = true \
             WHERE id = $1 AND device_id = $2 AND location_id = $3 AND token_hash = $4 \
             AND expires_at > (clock_timestamp() AT TIME ZONE 'UTC') \
             AND possession_verified = false",
        )
        .bind(session.id)
        .bind(session.device_id)
        .bind(session.location_id)
        .bind(&session.token_hash)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            transaction.rollback().await?;
            return Ok(false);
        }
        let consumed = Self::consume(
            &mut *transaction,
            challenge_id,
            session.device_id,
            DeviceIdentityChallengePurpose::MfaConnection,
            Some(session.location_id),
            Some(session.id),
        )
        .await?;
        if consumed.is_none() {
            transaction.rollback().await?;
            return Ok(false);
        }
        transaction.commit().await?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
