use std::fmt;

use chrono::{NaiveDateTime, Timelike, Utc};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, query, query_as, query_scalar};

use crate::db::{Id, NoId};

#[derive(Clone, Debug, Deserialize, Model, Serialize, PartialEq)]
pub struct Gateway<I = NoId> {
    pub id: I,
    pub location_id: Id,
    pub name: String,
    pub address: String,
    pub port: i32,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    pub certificate: Option<String>,
    pub certificate_expiry: Option<NaiveDateTime>,
    pub version: Option<String>,
    pub enabled: bool,
    pub modified_at: NaiveDateTime,
    pub modified_by: String,
}

impl<I> Gateway<I> {
    pub fn is_connected(&self) -> bool {
        if let (Some(connected_at), Some(disconnected_at)) =
            (self.connected_at, self.disconnected_at)
        {
            disconnected_at <= connected_at
        } else {
            self.connected_at.is_some()
        }
    }
}

impl Gateway {
    #[must_use]
    pub fn new<S: Into<String>>(
        network_id: Id,
        name: S,
        address: S,
        port: i32,
        modified_by: S,
    ) -> Self {
        // FIXME: this is a workaround for reducing timestamp precision.
        // `chrono` has nanosecond precision by default, while Postgres only does microseconds.
        // It avoids issues when comparing to objects fetched from DB.
        let modified_at = Utc::now().naive_utc();
        let modified_at = modified_at
            .with_nanosecond((modified_at.nanosecond() / 1_000) * 1_000)
            .expect("failed to truncate timestamp precision");

        Self {
            id: NoId,
            location_id: network_id,
            name: name.into(),
            address: address.into(),
            port,
            connected_at: None,
            disconnected_at: None,
            certificate: None,
            certificate_expiry: None,
            version: None,
            enabled: true,
            modified_by: modified_by.into(),
            modified_at,
        }
    }
}

impl Gateway<Id> {
    /// Mark all gateways currently considered connected as disconnected.
    pub async fn mark_all_disconnected<'e, E>(executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query(
            "UPDATE gateway \
			 SET disconnected_at = NOW() \
			 WHERE connected_at IS NOT NULL \
			 AND (disconnected_at IS NULL OR disconnected_at <= connected_at)",
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn find_by_location_id<'e, E>(
        executor: E,
        location_id: Id,
    ) -> Result<Vec<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT * FROM gateway WHERE location_id = $1 ORDER BY id",
            location_id
        )
        .fetch_all(executor)
        .await
    }

    /// Update `connected_at` to the current time and save it to the database.
    pub async fn touch_connected<'e, E>(&mut self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        self.connected_at = Some(Utc::now().naive_utc());
        query!(
            "UPDATE gateway SET connected_at = $2 WHERE id = $1",
            self.id,
            self.connected_at
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Set `disconnected_at` to the current time and save it to the database.
    pub async fn touch_disconnected<'e, E>(&mut self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        self.disconnected_at = Some(Utc::now().naive_utc());
        query!(
            "UPDATE gateway SET disconnected_at = $2 WHERE id = $1",
            self.id,
            self.disconnected_at
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn delete_by_id<'e, E>(executor: E, id: Id) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM \"gateway\" WHERE id = $1", id,)
            .execute(executor)
            .await?;

        Ok(())
    }

    // TODO: Split the URL into address and port fields just like in proxy
    pub async fn find_by_url<'e, E>(
        executor: E,
        address: &str,
        port: u16,
    ) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        let record = query_as!(
            Self,
            "SELECT * FROM gateway WHERE address = $1 AND port = $2",
            address,
            i32::from(port)
        )
        .fetch_optional(executor)
        .await?;

        Ok(record)
    }

    /// Return address and port as URL with HTTP scheme.
    #[must_use]
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.address, self.port)
    }

    /// Disable all Gateways except one. Used for expired licence.
    pub async fn leave_one_enabled<'e, E>(executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        let result = query_scalar!(
            "UPDATE gateway SET enabled = false WHERE enabled AND id NOT IN (\
                SELECT id FROM gateway WHERE enabled LIMIT 1
            )"
        )
        .execute(executor)
        .await?;

        tracing::debug!("Disabled {} Gateways", result.rows_affected());

        Ok(())
    }
}

impl fmt::Display for Gateway<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Gateway(ID {}; URL {}:{})",
            self.id, self.address, self.port
        )
    }
}
