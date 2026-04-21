use chrono::NaiveDateTime;
use serde::Serialize;
use utoipa::ToSchema;

use crate::db::{Id, models::proxy::Proxy};

// Used by the proxy manager to control proxies (start/shutdown).
pub enum ProxyControlMessage {
    StartConnection(Id),
    ShutdownConnection(Id),
    Purge(Id),
    /// Broadcast an already-provisioned certificate to all connected proxies.
    BroadcastHttpsCerts {
        cert_pem: String,
        key_pem: String,
    },
    ClearHttpsCerts,
}

#[derive(ToSchema, Serialize)]
pub struct ProxyInfo {
    pub id: Id,
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
}

impl From<Proxy<Id>> for ProxyInfo {
    fn from(value: Proxy<Id>) -> Self {
        Self {
            id: value.id,
            name: value.name,
            address: value.address,
            port: value.port,
            connected_at: value.connected_at,
            disconnected_at: value.disconnected_at,
            version: value.version,
            enabled: value.enabled,
            certificate_serial: value.certificate_serial,
            certificate_expiry: value.certificate_expiry,
            modified_at: value.modified_at,
            modified_by: value.modified_by,
        }
    }
}
