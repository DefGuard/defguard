use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{FromRow, Type};

use crate::db::{Id, NoId};

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "audit_module", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AuditModule {
    Defguard,
    Client,
    Vpn,
    Enrollment,
}

#[derive(Model, Serialize, FromRow)]
#[table(audit_event)]
pub struct AuditEvent<I = NoId> {
    pub id: I,
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub ip: IpNetwork,
    pub event: String,
    #[model(enum)]
    pub module: AuditModule,
    pub device: String,
    pub details: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
