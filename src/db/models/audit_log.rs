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

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    UserLogin,
    UserLogout,
    DeviceAdded,
    DeviceRemoved,
    DeviceModified,
    AuditStreamCreated,
    AuditStreamModified,
    AuditStreamRemoved,
}

#[derive(Model, FromRow, Serialize)]
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
