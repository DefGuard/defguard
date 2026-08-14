use std::time::Duration;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgExecutor, PgPool, query, query_as, query_scalar, types::Json};
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

/// Outcome of advancing to the next step.
#[derive(Clone, Debug, PartialEq)]
pub enum StepOutcome {
    /// The session advanced to `next_step` (0-indexed).
    Advanced { next_step: usize },
    /// The final step completed; collection is the next action.
    Complete,
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

    /// Begin (or re-issue) an attempt on the current step, overwriting any prior attempt.
    ///
    /// Returns the fresh `step_attempt_id`, which every async completion (OIDC callback,
    /// mobile approve) must carry and match.
    pub async fn begin_attempt(
        &self,
        conn: &mut PgConnection,
        method: VpnClientMfaMethod,
        challenge: Option<BiometricChallenge>,
    ) -> sqlx::Result<String> {
        let step_attempt_id = gen_alphanumeric(32);
        let state = EphemeralState {
            step_attempt_id: step_attempt_id.clone(),
            selected_method: method,
            openid_auth_completed: false,
            mobile_approved: false,
            biometric_challenge: challenge,
        };
        let state_json =
            serde_json::to_value(&state).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;

        query!(
            "UPDATE vpn_client_mfa_session SET ephemeral_state = $2 WHERE id = $1",
            self.id,
            state_json,
        )
        .execute(&mut *conn)
        .await?;

        Ok(step_attempt_id)
    }

    /// Mark the current attempt's OIDC verification complete.
    ///
    /// Returns `true` if the mark applied; a stale `step_attempt_id` is a no-op.
    pub async fn mark_oidc_completed(
        &self,
        conn: &mut PgConnection,
        step_attempt_id: &str,
    ) -> sqlx::Result<bool> {
        let result = query!(
            "UPDATE vpn_client_mfa_session \
             SET ephemeral_state = jsonb_set(ephemeral_state, '{openid_auth_completed}', 'true'::jsonb) \
             WHERE id = $1 AND ephemeral_state IS NOT NULL AND ephemeral_state->>'step_attempt_id' = $2",
            self.id,
            step_attempt_id,
        )
        .execute(&mut *conn)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Mark the current attempt's mobile approval complete.
    ///
    /// Symmetric to [`Self::mark_oidc_completed`]: returns `true` if the mark applied, and a
    /// stale `step_attempt_id` is a no-op. The caller verifies the approval signature first.
    pub async fn mark_mobile_approved(
        &self,
        conn: &mut PgConnection,
        step_attempt_id: &str,
    ) -> sqlx::Result<bool> {
        let result = query!(
            "UPDATE vpn_client_mfa_session \
             SET ephemeral_state = jsonb_set(ephemeral_state, '{mobile_approved}', 'true'::jsonb) \
             WHERE id = $1 AND ephemeral_state IS NOT NULL AND ephemeral_state->>'step_attempt_id' = $2",
            self.id,
            step_attempt_id,
        )
        .execute(&mut *conn)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Advance to the next step, clearing `ephemeral_state` and resetting `failed_attempts`.
    ///
    /// Does not extend `expires_at` (fixed window).
    pub async fn advance(&self, conn: &mut PgConnection) -> sqlx::Result<StepOutcome> {
        let next_step = query_scalar!(
            "UPDATE vpn_client_mfa_session \
             SET ephemeral_state = NULL, current_step = current_step + 1, failed_attempts = 0 \
             WHERE id = $1 \
             RETURNING current_step",
            self.id,
        )
        .fetch_one(&mut *conn)
        .await?;

        let total_steps = self.steps_snapshot.0.steps.len() as i32;
        let outcome = if next_step >= total_steps {
            StepOutcome::Complete
        } else {
            StepOutcome::Advanced {
                next_step: next_step as usize,
            }
        };

        Ok(outcome)
    }

    /// Record a proof-verification failure, incrementing the per-step counter.
    ///
    /// Returns `true` at [`MFA_FAILED_ATTEMPT_CAP`]. Does not delete the session; the
    /// orchestrator owns deletion and the terminal event.
    pub async fn record_failure(&self, conn: &mut PgConnection) -> sqlx::Result<bool> {
        let failed_attempts = query_scalar!(
            "UPDATE vpn_client_mfa_session \
             SET failed_attempts = failed_attempts + 1 \
             WHERE id = $1 \
             RETURNING failed_attempts",
            self.id,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(failed_attempts >= MFA_FAILED_ATTEMPT_CAP)
    }
}

/// Delete every session whose fixed window has elapsed. Silent hygiene, not correctness.
pub async fn reap_expired(pool: &PgPool) -> sqlx::Result<u64> {
    let result = query!("DELETE FROM vpn_client_mfa_session WHERE expires_at < now()")
        .execute(pool)
        .await?;
    let count = result.rows_affected();
    debug!("Reaped {count} expired MFA session(s)");
    Ok(count)
}

#[cfg(test)]
mod tests;
