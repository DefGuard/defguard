use std::fmt;

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::{query, query_as, PgExecutor};

use crate::db::{Id, NoId};

#[derive(Clone, Debug, Deserialize, Model, PartialEq, Serialize)]
pub(crate) struct Gateway<I = NoId> {
    pub id: I,
    pub network_id: Id,
    pub url: String,
    pub hostname: Option<String>,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
}

impl Gateway {
    #[must_use]
    pub(crate) fn new<S: Into<String>>(network_id: Id, url: S) -> Self {
        Self {
            id: NoId,
            network_id,
            url: url.into(),
            hostname: None,
            connected_at: None,
            disconnected_at: None,
        }
    }
}

impl Gateway<Id> {
    pub(crate) async fn find_by_network_id<'e, E>(
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
    pub(crate) async fn touch_connected<'e, E>(
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
    pub(crate) async fn touch_disconnected<'e, E>(&mut self, executor: E) -> Result<(), sqlx::Error>
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

    pub(crate) fn is_connected(&self) -> bool {
        if let (Some(connected_at), Some(disconnected_at)) =
            (self.connected_at, self.disconnected_at)
        {
            disconnected_at <= connected_at
        } else {
            self.connected_at.is_some()
        }
    }
}

impl fmt::Display for Gateway<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gateway(ID {}; URL {})", self.id, self.url)
    }
}
