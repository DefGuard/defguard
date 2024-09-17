use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError, PgExecutor};

use super::DbPool;
use crate::random::gen_alphanumeric;

// Token used for polling requests.
#[derive(Clone, Debug, Model)]
pub struct PollingToken {
    pub id: Option<i64>,
    pub token: String,
    pub device_id: i64,
    pub created_at: NaiveDateTime,
}

impl PollingToken {
    pub fn new(device_id: i64) -> Self {
        Self {
            id: None,
            device_id,
            token: gen_alphanumeric(32),
            created_at: Utc::now().naive_utc(),
        }
    }

    pub async fn find(pool: &DbPool, token: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, token, device_id, created_at
            FROM pollingtoken WHERE token = $1",
            token
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete_for_device_id<'e, E>(executor: E, device_id: i64) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query!("DELETE FROM pollingtoken WHERE device_id = $1", device_id)
            .execute(executor)
            .await?;
        Ok(())
    }
}
