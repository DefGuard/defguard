use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::Type;

use crate::db::{Id, NoId};

#[derive(Debug, Serialize, Type)]
#[sqlx(type_name = "audit_module", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AuditModule {
    Defguard,
    Client,
    Vpn,
    Enrollment,
}

#[derive(Model, Serialize)]
#[table(audit_event)]
pub struct AuditEvent<I = NoId> {
    id: I,
    timestamp: NaiveDateTime,
    user_id: Id,
    ip: IpNetwork,
    event: String,
    #[model(enum)]
    module: AuditModule,
    device: String,
    details: Option<String>,
    // metadata
}
