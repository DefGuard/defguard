use std::fmt;

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use serde::Deserialize;
use sqlx::{PgExecutor, query, query_as};

use crate::db::{Id, NoId};

#[derive(Deserialize, Model)]
pub struct Gateway<I = NoId> {
    pub id: I,
    pub network_id: Id,
    pub url: String,
    pub hostname: Option<String>,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    pub certificate: Option<String>,
    pub certificate_expiry: Option<NaiveDateTime>,
    pub version: Option<String>,
    pub name: String,
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
    pub fn new<S: Into<String>>(network_id: Id, url: S, name: S) -> Self {
        Self {
            id: NoId,
            network_id,
            url: url.into(),
            hostname: None,
            connected_at: None,
            disconnected_at: None,
            certificate: None,
            certificate_expiry: None,
            version: None,
            name: name.into(),
        }
    }
}

impl Gateway<Id> {
    pub async fn find_by_network_id<'e, E>(
        executor: E,
        network_id: Id,
    ) -> Result<Vec<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT * FROM gateway WHERE network_id = $1 ORDER BY id",
            network_id
        )
        .fetch_all(executor)
        .await
    }

    /// Update `hostname` and set `connected_at` to the current time and save it to the database.
    pub async fn touch_connected<'e, E>(
        &mut self,
        executor: E,
        hostname: String,
    ) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        self.hostname = Some(hostname);
        self.connected_at = Some(Utc::now().naive_utc());
        query!(
            "UPDATE gateway SET hostname = $2, connected_at = $3 WHERE id = $1",
            self.id,
            self.hostname,
            self.connected_at
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Set `disconnected_at` to the current time and save it to the database.
    pub async fn touch_disconnected<'e, E>(&mut self, executor: E) -> Result<(), sqlx::Error>
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

    pub async fn delete_by_id<'e, E>(executor: E, id: Id, network_id: Id) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query!(
            "DELETE FROM \"gateway\" WHERE id = $1 AND network_id = $2",
            id,
            network_id
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    // TODO: Split the URL into address and port fields just like in proxy
    pub async fn find_by_url<'e, E>(executor: E, url: &str) -> Result<Option<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let record = query_as!(Self, "SELECT * FROM gateway WHERE url = $1", url)
            .fetch_optional(executor)
            .await?;

        Ok(record)
    }
}

impl fmt::Display for Gateway<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gateway(ID {}; URL {})", self.id, self.url)
    }
}
