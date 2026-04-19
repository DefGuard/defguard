use std::fmt;

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool, query, query_as};
use utoipa::ToSchema;

use crate::{
    db::{Id, NoId},
    types::proxy::ProxyInfo,
};

#[derive(Clone, Deserialize, Model, Serialize, ToSchema, PartialEq)]
pub struct Proxy<I = NoId> {
    pub id: I,
    pub name: String,
    pub address: String,
    pub port: i32,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    pub version: Option<String>,
    pub enabled: bool,
    pub certificate_serial: Option<String>,
    pub certificate_expiry: Option<NaiveDateTime>,
    pub modified_at: NaiveDateTime,
    pub modified_by: String,
    #[serde(skip)]
    pub core_client_cert_der: Option<Vec<u8>>,
    #[serde(skip)]
    pub core_client_cert_key_der: Option<Vec<u8>>,
    pub core_client_cert_expiry: Option<NaiveDateTime>,
}

impl<I: fmt::Debug> fmt::Debug for Proxy<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Proxy")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("address", &self.address)
            .field("port", &self.port)
            .field("connected_at", &self.connected_at)
            .field("disconnected_at", &self.disconnected_at)
            .field("version", &self.version)
            .field("enabled", &self.enabled)
            .field("certificate_serial", &self.certificate_serial)
            .field("certificate_expiry", &self.certificate_expiry)
            .field("modified_at", &self.modified_at)
            .field("modified_by", &self.modified_by)
            .field(
                "core_client_cert_der",
                &self.core_client_cert_der.as_ref().map(|_| "<redacted>"),
            )
            .field("core_client_cert_key_der", &"<redacted>")
            .field("core_client_cert_expiry", &self.core_client_cert_expiry)
            .finish()
    }
}

impl fmt::Display for Proxy<NoId> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
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
    pub fn new<S: Into<String>>(name: S, address: S, port: i32, modified_by: S) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            address: address.into(),
            port,
            connected_at: None,
            disconnected_at: None,
            certificate_serial: None,
            certificate_expiry: None,
            version: None,
            enabled: true,
            modified_by: modified_by.into(),
            modified_at: Utc::now().naive_utc(),
            core_client_cert_der: None,
            core_client_cert_key_der: None,
            core_client_cert_expiry: None,
        }
    }
}

impl Proxy<Id> {
    /// Returns `true` if this proxy is currently considered connected.
    ///
    /// A proxy is connected when `connected_at` is set and either
    /// `disconnected_at` is absent or `connected_at > disconnected_at`.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        match (self.connected_at, self.disconnected_at) {
            (Some(c), Some(d)) => c > d,
            (Some(_), None) => true,
            _ => false,
        }
    }

    /// Mark all proxies currently considered connected as disconnected.
    pub async fn mark_all_disconnected<'e, E: PgExecutor<'e>>(executor: E) -> sqlx::Result<()> {
        query!(
            "UPDATE proxy \
			 SET disconnected_at = NOW() \
			 WHERE connected_at IS NOT NULL \
			 AND (disconnected_at IS NULL OR disconnected_at < connected_at)",
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    /// Fetch all enabled Proxies.
    pub async fn all_enabled<'e, E: PgExecutor<'e>>(executor: E) -> sqlx::Result<Vec<Self>> {
        query_as!(Self, "SELECT * FROM proxy WHERE enabled")
            .fetch_all(executor)
            .await
    }

    pub async fn find_by_address_port(
        pool: &PgPool,
        address: &str,
        port: i32,
    ) -> sqlx::Result<Option<Self>> {
        query_as!(
            Self,
            "SELECT * FROM proxy WHERE address = $1 AND port = $2",
            address,
            port
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<ProxyInfo>> {
        query_as!(
            ProxyInfo,
            "SELECT id, name, address, port, connected_at, disconnected_at, \
             version, enabled, certificate_serial, certificate_expiry, \
             modified_at, modified_by \
             FROM proxy",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn mark_connected(&mut self, pool: &PgPool, version: String) -> sqlx::Result<()> {
        self.version = Some(version);
        self.connected_at = Some(Utc::now().naive_utc());
        self.save(pool).await?;

        Ok(())
    }

    pub async fn mark_disconnected(&mut self, pool: &PgPool) -> sqlx::Result<()> {
        self.disconnected_at = Some(Utc::now().naive_utc());
        self.save(pool).await?;

        Ok(())
    }

    /// Fetch all enabled, but one. Used for expired licence.
    pub async fn leave_one_enabled<'e, E: PgExecutor<'e>>(executor: E) -> sqlx::Result<Vec<Self>> {
        query_as!(
            Self,
            "SELECT * FROM proxy WHERE enabled AND id NOT IN (\
                SELECT id FROM proxy WHERE enabled LIMIT 1
            )"
        )
        .fetch_all(executor)
        .await
    }
}
