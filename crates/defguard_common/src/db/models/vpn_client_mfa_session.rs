use std::time::Duration;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgExecutor, query, query_as, query_scalar, types::Json};
use tracing::debug;

use crate::{
    db::{
        Id,
        models::{biometric_auth::BiometricChallenge, vpn_client_session::VpnClientMfaMethod},
    },
    random::gen_alphanumeric,
};

/// Fixed wall-clock window for the whole in-progress MFA flow, including collection.
pub const VPN_MFA_SESSION_TIMEOUT: Duration = Duration::from_mins(10);

/// Per-step cap on proof-verification failures. A sanity/abuse limit, not a lockout.
pub const MFA_FAILED_ATTEMPT_CAP: i32 = 5;

/// Point-in-time snapshot of the resolved MFA flow, frozen at `start`.
///
/// `flow_id` is attribution-only: written once and copied to the authorized
/// `vpn_client_session` at delivery, never re-read to drive the flow.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StepsSnapshot {
    pub flow_id: Id,
    pub steps: Vec<Step>,
}

/// A single step within a frozen flow snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Step {
    pub methods: Vec<VpnClientMfaMethod>,
}

/// Per-step ephemeral attempt state, cleared to NULL on `advance`.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EphemeralState {
    pub step_attempt_id: String,
    pub selected_method: VpnClientMfaMethod,
    #[serde(default)]
    pub openid_auth_completed: bool,
    #[serde(default)]
    pub mobile_approved: bool,
    #[serde(default)]
    pub biometric_challenge: Option<BiometricChallenge>,
}

/// Result of `start`, carrying the raw token (returned exactly once) and the hash of any
/// session that was superseded.
pub struct StartOutcome {
    pub token: String,
    pub superseded_token_hash: Option<String>,
}

/// A durable in-progress VPN MFA session.
pub struct VpnClientMfaSession {
    pub id: Id,
    pub token_hash: String,
    pub location_id: Id,
    pub device_id: Id,
    pub user_id: Id,
    pub steps_snapshot: Json<StepsSnapshot>,
    pub current_step: i32,
    pub ephemeral_state: Option<Json<EphemeralState>>,
    pub failed_attempts: i32,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

/// Hash an opaque token for storage and lookup: base64url-nopad SHA-256.
#[must_use]
pub fn token_hash(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}

impl VpnClientMfaSession {
    /// Begin a new in-progress MFA session, superseding any existing session for the same
    /// `(location_id, device_id)`.
    ///
    /// `ttl` is a parameter (rather than a read of `VPN_MFA_SESSION_TIMEOUT`) so expiry can be
    /// exercised in tests without a 10-minute wait.
    pub async fn start(
        conn: &mut PgConnection,
        location_id: Id,
        device_id: Id,
        user_id: Id,
        flow_id: Id,
        steps: Vec<Vec<VpnClientMfaMethod>>,
        ttl: Duration,
    ) -> sqlx::Result<(Self, StartOutcome)> {
        let token = gen_alphanumeric(32);
        let hash = token_hash(&token);
        let snapshot = StepsSnapshot {
            flow_id,
            steps: steps.into_iter().map(|methods| Step { methods }).collect(),
        };
        let snapshot_json =
            serde_json::to_value(&snapshot).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
        let created_at = Utc::now().naive_utc();
        let expires_at = created_at + TimeDelta::seconds(ttl.as_secs() as i64);

        // Supersede any existing session for this (location, device), capturing its token hash so
        // the caller can cancel its waiter. The unique index plus the `ON CONFLICT` upsert below
        // closes the concurrent double-`Start` race (last-writer-wins, not an error).
        let superseded_token_hash = query_scalar!(
            "DELETE FROM vpn_client_mfa_session \
             WHERE location_id = $1 AND device_id = $2 \
             RETURNING token_hash",
            location_id,
            device_id,
        )
        .fetch_optional(&mut *conn)
        .await?;

        let session = query_as!(
            Self,
            "INSERT INTO vpn_client_mfa_session \
                (token_hash, location_id, device_id, user_id, steps_snapshot, current_step, ephemeral_state, failed_attempts, created_at, expires_at) \
             VALUES ($1, $2, $3, $4, $5, 0, NULL, 0, $6, $7) \
             ON CONFLICT (location_id, device_id) DO UPDATE SET \
                token_hash = EXCLUDED.token_hash, \
                user_id = EXCLUDED.user_id, \
                steps_snapshot = EXCLUDED.steps_snapshot, \
                current_step = EXCLUDED.current_step, \
                ephemeral_state = EXCLUDED.ephemeral_state, \
                failed_attempts = EXCLUDED.failed_attempts, \
                created_at = EXCLUDED.created_at, \
                expires_at = EXCLUDED.expires_at \
             RETURNING \
                id, token_hash, location_id, device_id, user_id, \
                steps_snapshot \"steps_snapshot: Json<StepsSnapshot>\", current_step, \
                ephemeral_state \"ephemeral_state: Json<EphemeralState>\", failed_attempts, \
                created_at, expires_at",
            hash,
            location_id,
            device_id,
            user_id,
            snapshot_json,
            created_at,
            expires_at,
        )
        .fetch_one(&mut *conn)
        .await?;

        Ok((
            session,
            StartOutcome {
                token,
                superseded_token_hash,
            },
        ))
    }

    /// Look up an active session by raw token, hashing internally.
    ///
    /// Returns `None` for an unknown token, an expired session, and a stale row whose snapshot
    /// fails to deserialize.
    pub async fn find_active_by_token<'e, E: PgExecutor<'e>>(
        executor: E,
        token: &str,
    ) -> Option<Self> {
        let hash = token_hash(token);
        let result = query_as!(
            Self,
            "SELECT id, token_hash, location_id, device_id, user_id, \
             steps_snapshot \"steps_snapshot: Json<StepsSnapshot>\", current_step, \
             ephemeral_state \"ephemeral_state: Json<EphemeralState>\", failed_attempts, \
             created_at, expires_at \
             FROM vpn_client_mfa_session \
             WHERE token_hash = $1 AND expires_at > now()",
            hash,
        )
        .fetch_optional(executor)
        .await;

        match result {
            Ok(session) => session,
            Err(err) => {
                debug!("Failed to find active MFA session: {err}");
                None
            }
        }
    }

    /// Remove this session row (authorize-time, abort-time, supersede-time).
    pub async fn delete<'e, E: PgExecutor<'e>>(&self, executor: E) -> sqlx::Result<()> {
        query!("DELETE FROM vpn_client_mfa_session WHERE id = $1", self.id)
            .execute(executor)
            .await?;
        Ok(())
    }

    /// The methods available on the current step.
    #[must_use]
    pub fn current_step_methods(&self) -> &[VpnClientMfaMethod] {
        self.steps_snapshot
            .0
            .steps
            .get(self.current_step as usize)
            .map_or(&[], |step| step.methods.as_slice())
    }
}

#[cfg(test)]
mod tests;
