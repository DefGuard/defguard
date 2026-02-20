use std::fmt;

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, query, query_as};

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
    pub modified_at: NaiveDateTime,
    pub modified_by: Id,
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
        modified_by: Id,
    ) -> Self {
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
            modified_by,
            modified_at: Utc::now().naive_utc(),
        }
    }
}

impl Gateway<Id> {
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
    pub async fn touch_connected<'e, E>(&mut self, executor: E) -> Result<(), sqlx::Error>
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

    pub async fn delete_by_id<'e, E>(executor: E, id: Id) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query!("DELETE FROM \"gateway\" WHERE id = $1", id,)
            .execute(executor)
            .await?;

        Ok(())
    }

    // TODO: Split the URL into address and port fields just like in proxy
    pub async fn find_by_url<'e, E>(
        executor: E,
        address: &str,
        port: u16,
    ) -> Result<Option<Self>, sqlx::Error>
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

    pub fn url(&self) -> String {
        format!("http://{}:{}", self.address, self.port)
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
