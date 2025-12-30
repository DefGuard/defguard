use std::fmt;

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use serde::{
    Deserialize,
    ser::{Serialize, SerializeStruct, Serializer},
};
use sqlx::{PgExecutor, query, query_as};
use utoipa::ToSchema;

use crate::db::{Id, NoId};

#[derive(Deserialize, Model, ToSchema)]
pub struct Gateway<I = NoId> {
    pub id: I,
    pub network_id: Id,
    pub url: String,
    pub hostname: Option<String>,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
}

/// Implement `Serialize` to accomodate `connected`.
impl<I> Serialize for Gateway<I> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Gateway", 6)?;

        state.serialize_field("network_id", &self.network_id)?;
        state.serialize_field("url", &self.url)?;
        state.serialize_field("hostname", &self.hostname)?;
        state.serialize_field("connected_at", &self.connected_at)?;
        state.serialize_field("disconnected_at", &self.disconnected_at)?;
        // Transient
        state.serialize_field("connected", &self.is_connected())?;

        state.end()
    }
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
    pub fn new<S: Into<String>>(network_id: Id, url: S) -> Self {
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
}

impl fmt::Display for Gateway<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Gateway(ID {}; URL {})", self.id, self.url)
    }
}
