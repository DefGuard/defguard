use crate::db::{Id, NoId};
use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{FromRow, Type};

pub mod metadata;

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
    // authentication
    UserLogin,
    UserLoginFailed,
    UserMfaLogin,
    UserMfaLoginFailed,
    RecoveryCodeUsed,
    UserLogout,
    // mfa management
    MfaDisabled,
    MfaTotpDisabled,
    MfaTotpEnabled,
    MfaEmailDisabled,
    MfaEmailEnabled,
    MfaSecurityKeyAdded,
    MfaSecurityKeyRemoved,
    // user management
    UserAdded,
    UserRemoved,
    UserModified,
    // device management
    DeviceAdded,
    DeviceRemoved,
    DeviceModified,
    NetworkDeviceAdded,
    NetworkDeviceRemoved,
    NetworkDeviceModified,
    // audit stream
    AuditStreamCreated,
    AuditStreamModified,
    AuditStreamRemoved,
    // OpenID app management
    OpenIdAppAdded,
    OpenIdAppRemoved,
    OpenIdAppModified,
    // VPN location management
    VpnLocationAdded,
    VpnLocationRemoved,
    VpnLocationModified,
    // VPN client events
    VpnClientConnected,
    VpnClientDisconnected,
    VpnClientConnectedMfa,
    VpnClientDisconnectedMfa,
    VpnClientMfaFailed,
    // Enrollment events
    EnrollmentStarted,
    EnrollmentDeviceAdded,
    EnrollmentCompleted,
    PasswordResetRequested,
    PasswordResetStarted,
    PasswordResetCompleted,
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
