use chrono::NaiveDateTime;
use serde::Serialize;
use utoipa::ToSchema;

use crate::db::Id;

// Used by the proxy manager to control proxies (start/shutdown).
pub enum ProxyControlMessage {
    StartConnection(Id),
    ShutdownConnection(Id),
    Purge(Id),
    /// Trigger ACME HTTP-01 certificate issuance on the specified proxy.
    TriggerAcme {
        proxy_id: Id,
        domain: String,
        use_staging: bool,
    },
    /// Broadcast an already-provisioned certificate to all connected proxies.
    BroadcastHttpsCerts {
        cert_pem: String,
        key_pem: String,
    },
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
    pub certificate: Option<String>,
    pub certificate_expiry: Option<NaiveDateTime>,
    pub modified_at: NaiveDateTime,
    pub modified_by: String,
}
