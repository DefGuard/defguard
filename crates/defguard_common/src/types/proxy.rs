use chrono::NaiveDateTime;
use serde::Serialize;
use utoipa::ToSchema;

use crate::db::Id;

// Used by the proxy manager to control proxies (start/shutdown).
pub enum ProxyControlMessage {
    StartConnection(Id),
    ShutdownConnection(Id),
    Purge(Id),
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
    pub certificate: Option<String>,
    pub certificate_expiry: Option<NaiveDateTime>,
    pub modified_at: NaiveDateTime,
    pub modified_by: Id,
    pub modified_by_firstname: String,
    pub modified_by_lastname: String,
}
