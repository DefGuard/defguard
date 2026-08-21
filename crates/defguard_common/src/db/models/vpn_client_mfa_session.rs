use std::time::Duration;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{NaiveDateTime, TimeDelta, Utc};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{
    Connection, PgConnection, PgExecutor, PgPool, query, query_as, query_scalar, types::Json,
};
use tracing::debug;

use crate::{
    db::{
        Id, NoId,
        models::{
            biometric_auth::BiometricChallenge, device::Device, user::User,
            vpn_client_session::VpnClientMfaMethod, wireguard::WireguardNetwork,
        },
    },
    random::gen_alphanumeric,
};

/// Fixed wall-clock window for the whole in-progress MFA flow, including collection.
///
/// Supersedes the in-memory `ClientLoginSession`, whose map entries lived for
/// `CLIENT_SESSION_TIMEOUT` (5 minutes). The window is deliberately doubled: a VPN MFA login
/// may require a remote mobile approval or an OIDC redirect that outlives the old in-memory
/// entry, and the durable row is reaped by a background job instead of a per-entry expiry.
pub const VPN_MFA_SESSION_TIMEOUT: Duration = Duration::from_mins(10);

/// Per-step cap on proof-verification failures. A sanity/abuse limit, not a lockout.
pub const MFA_FAILED_ATTEMPT_CAP: i32 = 5;

/// Point-in-time snapshot of the resolved MFA flow, frozen at `start`.
///
/// `flow_id` is attribution-only: written once and recorded in the immutable
/// authorization activity-log entry at delivery, never re-read to drive the flow.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StepsSnapshot {
    pub flow_id: Id,
    pub steps: Vec<Step>,
}

/// Attribution for a completed MFA session: the frozen step snapshot plus the governing flow's
/// title. `flow_name` is resolved at collection and is `None` when the flow was deleted
/// mid-session, which is a display concern rather than an error.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MfaAttribution {
    pub snapshot: StepsSnapshot,
    pub flow_name: Option<String>,
}

/// A single step within a frozen flow snapshot.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Step {
    pub methods: Vec<VpnClientMfaMethod>,
    /// The method that satisfied this step, recorded by `advance` from the method its caller
    /// verified.
    #[serde(default)]
    pub satisfied: Option<VpnClientMfaMethod>,
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

/// Result of `start`, carrying the raw token (returned exactly once), the id of the first
/// attempt minted alongside the row, and the hash of any session that was superseded.
pub struct StartOutcome {
    pub token: String,
    pub step_attempt_id: String,
    pub superseded_token_hash: Option<String>,
}

/// The location, device, and user a VPN MFA session references, loaded together for event
/// context and authorization.
pub struct MfaSessionContext {
    pub location: WireguardNetwork<Id>,
    pub device: Device<Id>,
    pub user: User<Id>,
}

/// Outcome of advancing to the next step.
#[derive(Clone, Debug, PartialEq)]
pub enum StepOutcome {
    /// The session advanced to `next_step` (0-indexed).
    Advanced { next_step: usize },
    /// The final step completed; collection is the next action.
    Complete,
}

/// Row shape returned by `advance`: the new step index and the updated snapshot.
struct AdvanceRow {
    current_step: i32,
    steps_snapshot: Json<StepsSnapshot>,
}

/// A durable in-progress VPN MFA session.
#[derive(Clone, Debug, Model)]
#[table(vpn_client_mfa_session)]
pub struct VpnClientMfaSession<I = NoId> {
    pub id: I,
    pub token_hash: String,
    pub location_id: Id,
    pub device_id: Id,
    pub user_id: Id,
    /// `#[model(json)]` makes the derive emit a `"steps_snapshot: _"` type override so the
    /// generated queries decode the `jsonb` column into `Json<StepsSnapshot>`, and binds it by
    /// reference with a no-op cast so sqlx's compile-time type check (which maps `jsonb` to
    /// `serde_json::Value`) is skipped.
    #[model(json)]
    pub steps_snapshot: Json<StepsSnapshot>,
    pub current_step: i32,
    /// Same `#[model(json)]` treatment as `steps_snapshot`, for a nullable `jsonb` column.
    #[model(json)]
    pub ephemeral_state: Option<Json<EphemeralState>>,
    pub failed_attempts: i32,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
}

/// Hash an opaque token for storage and lookup: base64url-nopad SHA-256.
#[must_use]
pub fn hash_token(token: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(token.as_bytes()))
}

/// Mint a fresh attempt on the current step, returning its `step_attempt_id` and the JSON the
/// `ephemeral_state` column stores. Shared by `start` (which writes it in the same statement
/// that mints the row) and `begin_attempt` (which overwrites a prior attempt in place).
fn new_attempt_state(
    method: VpnClientMfaMethod,
    challenge: Option<BiometricChallenge>,
) -> sqlx::Result<(String, serde_json::Value)> {
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

    Ok((step_attempt_id, state_json))
}

impl VpnClientMfaSession<Id> {
    /// Begin a new in-progress MFA session, superseding any existing session for the same
    /// `(location_id, device_id)`.
    ///
    /// The first attempt is minted here rather than by a follow-up `begin_attempt`: `method`
    /// and `challenge` are written by the same statement that mints the row, so the session is
    /// born with its attempt. Splitting the two writes left a window in which a concurrent
    /// `start` could take the `ON CONFLICT DO UPDATE` branch (which preserves the row id) and
    /// have the losing caller's `begin_attempt` land on the winner's row, handing that client a
    /// token whose attempt carries a method it never selected.
    ///
    /// `ttl` is a parameter (rather than a read of `VPN_MFA_SESSION_TIMEOUT`) so expiry can be
    /// exercised in tests without a 10-minute wait.
    #[allow(clippy::too_many_arguments)]
    pub async fn start(
        conn: &mut PgConnection,
        location_id: Id,
        device_id: Id,
        user_id: Id,
        flow_id: Id,
        steps: Vec<Vec<VpnClientMfaMethod>>,
        method: VpnClientMfaMethod,
        challenge: Option<BiometricChallenge>,
        ttl: Duration,
    ) -> sqlx::Result<(Self, StartOutcome)> {
        let token = gen_alphanumeric(32);
        let hash = hash_token(&token);
        let snapshot = StepsSnapshot {
            flow_id,
            steps: steps
                .into_iter()
                .map(|methods| Step {
                    methods,
                    satisfied: None,
                })
                .collect(),
        };
        let snapshot_json =
            serde_json::to_value(&snapshot).map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
        let (step_attempt_id, state_json) = new_attempt_state(method, challenge)?;
        let created_at = Utc::now().naive_utc();
        let expires_at = created_at + TimeDelta::seconds(ttl.as_secs() as i64);

        // Supersede any existing session for this (location, device), capturing its token hash so
        // the caller can cancel its waiter. The DELETE and the upsert run in one transaction so a
        // concurrent reader never observes the gap between them. The unique index plus the
        // `ON CONFLICT` upsert below closes the concurrent double-`Start` race (last-writer-wins),
        // and because the upsert also writes `ephemeral_state`, the winner's attempt is committed
        // with its row rather than by a second write a loser could interleave with.
        //
        // This is deliberately a transaction rather than one data-modifying CTE: in
        // `WITH d AS (DELETE ...) INSERT ...` the index insert is checked before the delete
        // becomes visible, so the upsert can still raise a unique violation.
        let mut tx = conn.begin().await?;

        let superseded_token_hash = query_scalar!(
            "DELETE FROM vpn_client_mfa_session \
             WHERE location_id = $1 AND device_id = $2 \
             RETURNING token_hash",
            location_id,
            device_id,
        )
        .fetch_optional(&mut *tx)
        .await?;

        let session = query_as!(
            Self,
            "INSERT INTO vpn_client_mfa_session \
                (token_hash, location_id, device_id, user_id, steps_snapshot, current_step, ephemeral_state, failed_attempts, created_at, expires_at) \
             VALUES ($1, $2, $3, $4, $5, 0, $6, 0, $7, $8) \
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
            state_json,
            created_at,
            expires_at,
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok((
            session,
            StartOutcome {
                token,
                step_attempt_id,
                superseded_token_hash,
            },
        ))
    }

    /// Look up an active session by raw token, hashing internally.
    ///
    /// Returns `Ok(None)` for an unknown token and an expired session. A row whose snapshot
    /// fails to deserialize surfaces as an error, as do other database errors; the caller owns
    /// the decision of how to surface them.
    pub async fn find_active_by_token<'e, E: PgExecutor<'e>>(
        executor: E,
        token: &str,
    ) -> sqlx::Result<Option<Self>> {
        let hash = hash_token(token);
        query_as!(
            Self,
            "SELECT id, token_hash, location_id, device_id, user_id, \
             steps_snapshot \"steps_snapshot: Json<StepsSnapshot>\", current_step, \
             ephemeral_state \"ephemeral_state: Json<EphemeralState>\", failed_attempts, \
             created_at, expires_at \
             FROM vpn_client_mfa_session \
             WHERE token_hash = $1 AND expires_at > (now() AT TIME ZONE 'UTC')",
            hash,
        )
        .fetch_optional(executor)
        .await
    }

    /// Load the location, device, and user this session references.
    ///
    /// Returns `Ok(None)` if any referenced entity no longer exists (deleted after the session
    /// was started); callers map the `None` to their own status.
    pub async fn load_context(&self, pool: &PgPool) -> sqlx::Result<Option<MfaSessionContext>> {
        let Some(location) = WireguardNetwork::find_by_id(pool, self.location_id).await? else {
            return Ok(None);
        };
        let Some(device) = Device::find_by_id(pool, self.device_id).await? else {
            return Ok(None);
        };
        let Some(user) = User::find_by_id(pool, self.user_id).await? else {
            return Ok(None);
        };
        Ok(Some(MfaSessionContext {
            location,
            device,
            user,
        }))
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
    ///
    /// The *first* attempt of a session is not minted here: `start` writes it inline, so this is
    /// for re-issuing an attempt on the current step and for initializing each subsequent step.
    pub async fn begin_attempt(
        &self,
        conn: &mut PgConnection,
        method: VpnClientMfaMethod,
        challenge: Option<BiometricChallenge>,
    ) -> sqlx::Result<String> {
        let (step_attempt_id, state_json) = new_attempt_state(method, challenge)?;

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
        self.mark_flag(conn, step_attempt_id, "openid_auth_completed")
            .await
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
        self.mark_flag(conn, step_attempt_id, "mobile_approved")
            .await
    }

    /// Set a named completion flag on the current attempt, gated on a matching
    /// `step_attempt_id`. Returns `true` if the flag was set (0 rows otherwise).
    async fn mark_flag(
        &self,
        conn: &mut PgConnection,
        step_attempt_id: &str,
        flag: &str,
    ) -> sqlx::Result<bool> {
        let result = query!(
            "UPDATE vpn_client_mfa_session \
             SET ephemeral_state = jsonb_set(ephemeral_state, ARRAY[$2]::text[], 'true'::jsonb) \
             WHERE id = $1 AND ephemeral_state IS NOT NULL AND ephemeral_state->>'step_attempt_id' = $3",
            self.id,
            flag,
            step_attempt_id,
        )
        .execute(&mut *conn)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Advance to the next step, clearing `ephemeral_state` and resetting `failed_attempts`.
    ///
    /// Records the closing step's proof into the snapshot first:
    /// `steps[current_step].satisfied = ephemeral_state.selected_method`. The write and the
    /// clear land in one statement so the proof cannot be lost between them. A NULL
    /// `ephemeral_state` leaves `satisfied` unset rather than erroring.
    ///
    /// The write is guarded by `current_step`, and by `step_attempt_id` when the client supplied
    /// one, so a stale or duplicate advance matches zero rows rather than skipping a step and
    /// returns `Ok(None)`. Does not extend `expires_at` (fixed window).
    pub async fn advance(
        &self,
        conn: &mut PgConnection,
        current_step: i32,
        step_attempt_id: Option<&str>,
        satisfied_method: VpnClientMfaMethod,
    ) -> sqlx::Result<Option<(StepOutcome, StepsSnapshot)>> {
        // Record the method the caller actually verified rather than re-reading
        // `ephemeral_state->'selected_method'`: a concurrent `begin_attempt` can overwrite that
        // field between the verification and this statement, attributing the step to a method the
        // user never proved.
        let satisfied = serde_json::to_value(satisfied_method)
            .map_err(|err| sqlx::Error::Decode(Box::new(err)))?;
        // The `WHERE` clause is the guard that stops a proof advancing a step it was not minted
        // for. `$3` is NULL for a pre-2.2 client that cannot send an attempt id, which degrades to
        // the step-only binding.
        let row = query_as!(
            AdvanceRow,
            "UPDATE vpn_client_mfa_session \
             SET steps_snapshot = CASE \
                 WHEN ephemeral_state IS NOT NULL THEN jsonb_set( \
                     steps_snapshot, \
                     ARRAY['steps', current_step::text, 'satisfied'], \
                     $4 \
                 ) \
                 ELSE steps_snapshot \
             END, \
             ephemeral_state = NULL, \
             current_step = current_step + 1, \
             failed_attempts = 0 \
             WHERE id = $1 AND current_step = $2 \
               AND ($3::text IS NULL OR ephemeral_state->>'step_attempt_id' = $3) \
             RETURNING current_step, steps_snapshot \"steps_snapshot: Json<StepsSnapshot>\"",
            self.id,
            current_step,
            step_attempt_id,
            satisfied,
        )
        .fetch_optional(&mut *conn)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let total_steps = self.steps_snapshot.0.steps.len() as i32;
        let outcome = if row.current_step >= total_steps {
            StepOutcome::Complete
        } else {
            StepOutcome::Advanced {
                next_step: row.current_step as usize,
            }
        };

        Ok(Some((outcome, row.steps_snapshot.0)))
    }

    /// Increment the per-step proof-failure counter.
    ///
    /// Returns `true` once [`MFA_FAILED_ATTEMPT_CAP`] is reached. Does not delete the session;
    /// the orchestrator owns deletion and the terminal event.
    pub async fn increment_failed_attempts(&self, conn: &mut PgConnection) -> sqlx::Result<bool> {
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
    let result =
        query!("DELETE FROM vpn_client_mfa_session WHERE expires_at < (now() AT TIME ZONE 'UTC')")
            .execute(pool)
            .await?;
    let count = result.rows_affected();
    debug!("Reaped {count} expired MFA session(s)");
    Ok(count)
}

#[cfg(test)]
mod tests;
