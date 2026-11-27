use std::{fmt, time::SystemTime};

use chrono::{NaiveDateTime, TimeDelta, Utc};
use model_derive::Model;
use sqlx::{PgConnection, PgExecutor, Type, query, query_as};
use totp_lite::{Sha1, totp_custom};

use crate::{
    db::{Id, NoId},
    random::gen_totp_secret,
};

#[cfg(test)]
mod tests;

pub const DEVICE_VERIFICATION_TIMEOUT: TimeDelta = TimeDelta::hours(1);
pub const DEVICE_VERIFICATION_CODE_PERIOD: u64 = 60;
pub const DEVICE_VERIFICATION_CODE_DIGITS: u32 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Type)]
#[sqlx(type_name = "device_verification_outcome", rename_all = "snake_case")]
pub enum DeviceVerificationOutcome {
    Verified,
    Blocked,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceVerificationState {
    Pending,
    Expired,
    Resolved(DeviceVerificationOutcome),
}

#[derive(Clone, Model)]
#[table(device_verification)]
pub struct DeviceVerification<I = NoId> {
    pub id: I,
    pub device_id: Id,
    pub started_by: Option<Id>,
    pub secret: Vec<u8>,
    pub created_at: NaiveDateTime,
    pub expires_at: NaiveDateTime,
    #[model(option)]
    pub outcome: Option<DeviceVerificationOutcome>,
    pub resolved_at: Option<NaiveDateTime>,
}

impl<I: fmt::Debug> fmt::Debug for DeviceVerification<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceVerification")
            .field("id", &self.id)
            .field("device_id", &self.device_id)
            .field("started_by", &self.started_by)
            .field("secret", &"***")
            .field("created_at", &self.created_at)
            .field("expires_at", &self.expires_at)
            .field("outcome", &self.outcome)
            .field("resolved_at", &self.resolved_at)
            .finish()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct DeviceVerificationCode {
    pub code: String,
    pub expires_in: u64,
}

impl DeviceVerification {
    #[must_use]
    pub fn new(device_id: Id, started_by: Option<Id>) -> Self {
        let created_at = Utc::now().naive_utc();
        Self {
            id: NoId,
            device_id,
            started_by,
            secret: gen_totp_secret(),
            created_at,
            expires_at: created_at + DEVICE_VERIFICATION_TIMEOUT,
            outcome: None,
            resolved_at: None,
        }
    }

    /// Stores this verification unless the device already has an active one, which is
    /// returned instead. The flag tells whether a new verification was stored.
    pub async fn start_or_get(
        self,
        conn: &mut PgConnection,
    ) -> sqlx::Result<(DeviceVerification<Id>, bool)> {
        // a finished session is replaced by a new row, so a stale session id never reads its status
        query!(
            "DELETE FROM device_verification WHERE device_id = $1 \
            AND (expires_at <= (now() AT TIME ZONE 'UTC') OR outcome IS NOT NULL)",
            self.device_id
        )
        .execute(&mut *conn)
        .await?;

        let started = query_as!(
            DeviceVerification::<Id>,
            "INSERT INTO device_verification (device_id, started_by, secret, created_at, expires_at) \
            VALUES ($1, $2, $3, $4, $5) \
            ON CONFLICT (device_id) DO NOTHING \
            RETURNING id, device_id, started_by, secret, created_at, expires_at, \
            outcome \"outcome?: DeviceVerificationOutcome\", resolved_at",
            self.device_id,
            self.started_by,
            self.secret,
            self.created_at,
            self.expires_at,
        )
        .fetch_optional(&mut *conn)
        .await?;
        if let Some(started) = started {
            return Ok((started, true));
        }

        // the row may expire between the two queries, so it is fetched without the expiry filter
        let existing = query_as!(
            DeviceVerification::<Id>,
            "SELECT id, device_id, started_by, secret, created_at, expires_at, \
            outcome \"outcome?: DeviceVerificationOutcome\", resolved_at \
            FROM device_verification WHERE device_id = $1",
            self.device_id
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok((existing, false))
    }
}

impl DeviceVerification<Id> {
    pub async fn find_active_for_device<'e, E>(
        executor: E,
        device_id: Id,
    ) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, device_id, started_by, secret, created_at, expires_at, \
            outcome \"outcome?: DeviceVerificationOutcome\", resolved_at \
            FROM device_verification \
            WHERE device_id = $1 AND outcome IS NULL AND expires_at > (now() AT TIME ZONE 'UTC')",
            device_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn find_for_device<'e, E>(
        executor: E,
        id: Id,
        device_id: Id,
    ) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, device_id, started_by, secret, created_at, expires_at, \
            outcome \"outcome?: DeviceVerificationOutcome\", resolved_at \
            FROM device_verification WHERE id = $1 AND device_id = $2",
            id,
            device_id
        )
        .fetch_optional(executor)
        .await
    }

    /// Does nothing when the session already expired or was resolved.
    pub async fn resolve_active_for_device<'e, E>(
        executor: E,
        device_id: Id,
        outcome: DeviceVerificationOutcome,
    ) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "UPDATE device_verification \
            SET outcome = $2, resolved_at = (now() AT TIME ZONE 'UTC') \
            WHERE device_id = $1 AND outcome IS NULL AND expires_at > (now() AT TIME ZONE 'UTC') \
            RETURNING id, device_id, started_by, secret, created_at, expires_at, \
            outcome \"outcome?: DeviceVerificationOutcome\", resolved_at",
            device_id,
            outcome as DeviceVerificationOutcome,
        )
        .fetch_optional(executor)
        .await
    }

    #[must_use]
    pub fn state_at(&self, now: NaiveDateTime) -> DeviceVerificationState {
        match self.outcome {
            Some(outcome) => DeviceVerificationState::Resolved(outcome),
            None if self.expires_at <= now => DeviceVerificationState::Expired,
            None => DeviceVerificationState::Pending,
        }
    }

    #[must_use]
    pub fn code_at(&self, unix_secs: u64) -> DeviceVerificationCode {
        DeviceVerificationCode {
            code: totp_custom::<Sha1>(
                DEVICE_VERIFICATION_CODE_PERIOD,
                DEVICE_VERIFICATION_CODE_DIGITS,
                &self.secret,
                unix_secs,
            ),
            expires_in: DEVICE_VERIFICATION_CODE_PERIOD
                - unix_secs % DEVICE_VERIFICATION_CODE_PERIOD,
        }
    }

    #[must_use]
    pub fn current_code(&self) -> DeviceVerificationCode {
        let unix_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.code_at(unix_secs)
    }
}
