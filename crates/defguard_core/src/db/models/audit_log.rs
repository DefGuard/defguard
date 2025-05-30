use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{FromRow, Type};

use crate::db::{Id, NoId};

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "audit_module", rename_all = "snake_case")]
#[serde(rename_all = "lowercase")]
pub enum AuditModule {
    Defguard,
    Client,
    Vpn,
    Enrollment,
}

/// Represents audit event type as it's stored in the DB
///
/// To make searching and exporting the type is stored as text and not a custom Postgres enum.
/// Variant names are renamed to `snake_case` so `UserLogin` becomes `user_login` in the DB table.
#[derive(Clone, Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // user management
    UserLogin,
    UserLogout,
    MfaDisabled,
    MfaTotpDisabled,
    MfaTotpEnabled,
    MfaEmailDisabled,
    MfaEmailEnabled,
    MfaSecurityKeyAdded,
    MfaSecurityKeyRemoved,
    UserAdded,
    UserRemoved,
    UserModified,
    // device management
    DeviceAdded,
    DeviceRemoved,
    DeviceModified,
    // OpenID app management
    OpenIdAppAdded,
    OpenIdAppRemoved,
    OpenIdAppModified,
    // VPN location management
    VpnLocationAdded,
    VpnLocationRemoved,
    VpnLocationModified,
}

#[derive(Model, FromRow)]
#[table(audit_event)]
pub struct AuditEvent<I = NoId> {
    pub id: I,
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub ip: IpNetwork,
    #[model(enum)]
    pub event: EventType,
    #[model(enum)]
    pub module: AuditModule,
    pub device: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct DeviceAddedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct DeviceRemovedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct DeviceModifiedMetadata {
    pub device_names: Vec<String>,
}

#[derive(Serialize)]
pub struct UserAddedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct UserModifiedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct UserRemovedMetadata {
    pub username: String,
}

#[derive(Serialize)]
pub struct MfaSecurityKeyRemovedMetadata {
    pub key_id: Id,
    pub key_name: String,
}

#[derive(Serialize)]
pub struct MfaSecurityKeyAddedMetadata {
    pub key_id: Id,
    pub key_name: String,
}

#[derive(Serialize)]
pub struct NetworkDeviceAddedMetadata {
    device_id: Id,
    device_name: String,
    location_id: Id,
    location: String,
}
