use std::fmt;

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::{
    db::{Id, NoId},
    types::proxy::ProxyInfo,
};

#[derive(Clone, Debug, Deserialize, Model, Serialize, ToSchema, PartialEq)]
pub struct Proxy<I = NoId> {
    pub id: I,
    pub name: String,
    pub address: String,
    pub port: i32,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    pub version: Option<String>,
    pub certificate: Option<String>,
    pub certificate_expiry: Option<NaiveDateTime>,
    pub modified_at: NaiveDateTime,
    pub modified_by: Id,
}

impl fmt::Display for Proxy<NoId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl fmt::Display for Proxy<Id> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[ID {}] {}", self.id, self.name)
    }
}

impl Proxy {
    /// Creates a new `Proxy` instance with the given connection details.
    ///
    /// # Parameters
    /// - `name`: Human-readable proxy name.
    /// - `address`: Network address (IP or hostname) of the proxy for grpc connection.
    /// - `port`: TCP port the proxy listens on.
    /// - `modified_by`: Identifier of the user who created or last modified this proxy.
    #[must_use]
    pub fn new<S: Into<String>>(name: S, address: S, port: i32, modified_by: Id) -> Self {
        Self {
            id: NoId,
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

impl Proxy<Id> {
    pub async fn find_by_address_port(
        pool: &PgPool,
        address: &str,
        port: i32,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Proxy,
            "SELECT * FROM proxy WHERE address = $1 AND port = $2",
            address,
            port
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<ProxyInfo>> {
        sqlx::query_as!(
            ProxyInfo,
            "SELECT proxy.*, u.first_name modified_by_firstname, u.last_name modified_by_lastname \
            FROM proxy JOIN \"user\" u on proxy.modified_by = u.id",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn mark_connected(&mut self, pool: &PgPool, version: &str) -> sqlx::Result<()> {
        self.version = Some(version.to_string());
        self.connected_at = Some(Utc::now().naive_utc());
        self.save(pool).await?;

        Ok(())
    }

    pub async fn mark_disconnected(&mut self, pool: &PgPool) -> sqlx::Result<()> {
        self.disconnected_at = Some(Utc::now().naive_utc());
        self.save(pool).await?;

        Ok(())
    }
}
