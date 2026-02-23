use chrono::NaiveDateTime;
use defguard_common::db::{Id, NoId};
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{FromRow, Type};

pub mod metadata;

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
#[sqlx(type_name = "activity_log_module", rename_all = "snake_case")]
#[serde(rename_all = "lowercase")]
pub enum ActivityLogModule {
    Defguard,
    Client,
    Vpn,
    Enrollment,
}

/// Represents activity log event type as it's stored in the DB
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
    UserMfaDisabled,
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
    UserGroupsModified,
    PasswordChanged,
    PasswordChangedByAdmin,
    PasswordReset,
    // device management
    DeviceAdded,
    DeviceRemoved,
    DeviceModified,
    NetworkDeviceAdded,
    NetworkDeviceRemoved,
    NetworkDeviceModified,
    // activity log stream
    ActivityLogStreamCreated,
    ActivityLogStreamModified,
    ActivityLogStreamRemoved,
    ClientConfigurationTokenAdded,
    // OpenID app management
    OpenIdAppAdded,
    OpenIdAppRemoved,
    OpenIdAppModified,
    OpenIdAppStateChanged,
    // OpenID provider management
    OpenIdProviderRemoved,
    OpenIdProviderModified,
    // VPN location management
    VpnLocationAdded,
    VpnLocationRemoved,
    VpnLocationModified,
    // VPN client events
    VpnClientConnected,
    VpnClientDisconnected,
    VpnClientMfaSuccess,
    VpnClientMfaFailed,
    // Enrollment events
    EnrollmentTokenAdded,
    EnrollmentStarted,
    EnrollmentDeviceAdded,
    EnrollmentCompleted,
    PasswordResetRequested,
    PasswordResetStarted,
    PasswordResetCompleted,
    // API token management,
    ApiTokenAdded,
    ApiTokenRemoved,
    ApiTokenRenamed,
    // Settings management
    SettingsUpdated,
    SettingsUpdatedPartial,
    SettingsDefaultBrandingRestored,
    // Groups management
    GroupsBulkAssigned,
    GroupAdded,
    GroupModified,
    GroupRemoved,
    GroupMemberAdded,
    GroupMemberRemoved,
    GroupMembersModified,
    // WebHook management
    WebHookAdded,
    WebHookModified,
    WebHookRemoved,
    WebHookStateChanged,
    // Authentication key management
    AuthenticationKeyAdded,
    AuthenticationKeyRemoved,
    AuthenticationKeyRenamed,
    // User SNAT bindings management
    UserSnatBindingAdded,
    UserSnatBindingRemoved,
    UserSnatBindingModified,
    // Proxy management
    ProxyModified,
    ProxyDeleted,
    // Gateway management
    GatewayModified,
    GatewayDeleted,
}

#[derive(Model, FromRow, Serialize)]
#[table(activity_log_event)]
pub struct ActivityLogEvent<I = NoId> {
    pub id: I,
    pub timestamp: NaiveDateTime,
    pub user_id: Id,
    pub username: String,
    pub location: Option<String>,
    pub ip: IpNetwork,
    #[model(enum)]
    pub event: EventType,
    #[model(enum)]
    pub module: ActivityLogModule,
    pub device: String,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
