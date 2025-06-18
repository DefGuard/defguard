use chrono::NaiveDateTime;

use crate::{
    db::{
        models::{
            authentication_key::{AuthenticationKey, AuthenticationKeyType},
            oauth2client::OAuth2Client,
        },
        Device, Group, Id, MFAMethod, User, WebAuthn, WebHook, WireguardNetwork,
    },
    enterprise::db::models::{
        api_tokens::ApiToken,
        audit_stream::{AuditStream, AuditStreamType},
        openid_provider::{DirectorySyncTarget, DirectorySyncUserBehavior, OpenIdProvider},
    },
};

#[derive(Serialize)]
pub struct MfaLoginMetadata {
    pub mfa_method: MFAMethod,
}

#[derive(Serialize)]
pub struct DeviceMetadata {
    pub owner: User<Id>,
    pub device: Device<Id>,
}

#[derive(Serialize)]
pub struct DeviceModifiedMetadata {
    pub owner: User<Id>,
    pub before: Device<Id>,
    pub after: Device<Id>,
}

#[derive(Serialize)]
pub struct NetworkDeviceMetadata {
    pub device: Device<Id>,
    pub location: WireguardNetwork<Id>,
}

#[derive(Serialize)]
pub struct NetworkDeviceModifiedMetadata {
    pub location: WireguardNetwork<Id>,
    pub before: Device<Id>,
    pub after: Device<Id>,
}

#[derive(Serialize)]
pub struct UserMetadata {
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct UserModifiedMetadata {
    pub before: User<Id>,
    pub after: User<Id>,
}

#[derive(Serialize)]
pub struct MfaSecurityKeyMetadata {
    pub key: WebAuthnNoSecrets,
}

// Avoid storing secrets in metadata
#[derive(Serialize)]
pub struct WebAuthnNoSecrets {
    pub id: Id,
    pub user_id: Id,
    pub name: String,
}

impl From<WebAuthn<Id>> for WebAuthnNoSecrets {
    fn from(value: WebAuthn<Id>) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            name: value.name,
        }
    }
}

#[derive(Serialize)]
pub struct AuditStreamMetadata {
    pub stream: AuditStreamNoSecrets,
}

#[derive(Serialize)]
pub struct AuditStreamNoSecrets {
    pub id: Id,
    pub name: String,
    pub stream_type: AuditStreamType,
}

impl From<AuditStream<Id>> for AuditStreamNoSecrets {
    fn from(value: AuditStream<Id>) -> Self {
        Self {
            id: value.id,
            name: value.name,
            stream_type: value.stream_type,
        }
    }
}

#[derive(Serialize)]
pub struct AuditStreamModifiedMetadata {
    pub before: AuditStreamNoSecrets,
    pub after: AuditStreamNoSecrets,
}

#[derive(Serialize)]
pub struct VpnClientMetadata {
    pub location: WireguardNetwork<Id>,
    pub device: Device<Id>,
}

#[derive(Serialize)]
pub struct VpnClientMfaMetadata {
    pub location: WireguardNetwork<Id>,
    pub device: Device<Id>,
    pub method: MFAMethod,
}

#[derive(Serialize)]
pub struct EnrollmentDeviceAddedMetadata {
    pub device: Device<Id>,
}

#[derive(Serialize)]
pub struct EnrollmentTokenMetadata {
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct VpnLocationMetadata {
    pub location: WireguardNetwork<Id>,
}

#[derive(Serialize)]
pub struct VpnLocationModifiedMetadata {
    pub before: WireguardNetwork<Id>,
    pub after: WireguardNetwork<Id>,
}

#[derive(Serialize)]
pub struct ApiTokenMetadata {
    pub owner: User<Id>,
    pub token: ApiTokenNoSecrets,
}

#[derive(Serialize)]
pub struct ApiTokenNoSecrets {
    id: Id,
    pub user_id: Id,
    pub created_at: NaiveDateTime,
    pub name: String,
}

impl From<ApiToken<Id>> for ApiTokenNoSecrets {
    fn from(value: ApiToken<Id>) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            created_at: value.created_at,
            name: value.name,
        }
    }
}

#[derive(Serialize)]
pub struct ApiTokenRenamedMetadata {
    pub owner: User<Id>,
    pub token: ApiTokenNoSecrets,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Serialize)]
pub struct OpenIdAppMetadata {
    pub app: OAuth2ClientNoSecrets,
}

#[derive(Serialize)]
pub struct OAuth2ClientNoSecrets {
    pub id: Id,
    pub client_id: String, // unique
    pub redirect_uri: Vec<String>,
    pub scope: Vec<String>,
    pub name: String,
    pub enabled: bool,
}

impl From<OAuth2Client<Id>> for OAuth2ClientNoSecrets {
    fn from(value: OAuth2Client<Id>) -> Self {
        Self {
            id: value.id,
            client_id: value.client_id,
            redirect_uri: value.redirect_uri,
            scope: value.scope,
            name: value.name,
            enabled: value.enabled,
        }
    }
}

#[derive(Serialize)]
pub struct OpenIdAppModifiedMetadata {
    pub before: OAuth2ClientNoSecrets,
    pub after: OAuth2ClientNoSecrets,
}

#[derive(Serialize)]
pub struct OpenIdAppStateChangedMetadata {
    pub app: OAuth2ClientNoSecrets,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct OpenIdProviderMetadata {
    pub provider: OpenIdProviderNoSecrets,
}

#[derive(Serialize)]
pub struct OpenIdProviderNoSecrets {
    pub id: Id,
    pub name: String,
    pub base_url: String,
    pub client_id: String,
    pub display_name: Option<String>,
    pub google_service_account_email: Option<String>,
    pub admin_email: Option<String>,
    pub directory_sync_enabled: bool,
    pub directory_sync_interval: i32,
    pub directory_sync_user_behavior: DirectorySyncUserBehavior,
    pub directory_sync_admin_behavior: DirectorySyncUserBehavior,
    pub directory_sync_target: DirectorySyncTarget,
    pub okta_dirsync_client_id: Option<String>,
    pub directory_sync_group_match: Vec<String>,
}

impl From<OpenIdProvider<Id>> for OpenIdProviderNoSecrets {
    fn from(value: OpenIdProvider<Id>) -> Self {
        Self {
            id: value.id,
            name: value.name,
            base_url: value.base_url,
            client_id: value.client_id,
            display_name: value.display_name,
            google_service_account_email: value.google_service_account_email,
            admin_email: value.admin_email,
            directory_sync_enabled: value.directory_sync_enabled,
            directory_sync_interval: value.directory_sync_interval,
            directory_sync_user_behavior: value.directory_sync_user_behavior,
            directory_sync_admin_behavior: value.directory_sync_admin_behavior,
            directory_sync_target: value.directory_sync_target,
            okta_dirsync_client_id: value.okta_dirsync_client_id,
            directory_sync_group_match: value.directory_sync_group_match,
        }
    }
}

#[derive(Serialize)]
pub struct GroupsBulkAssignedMetadata {
    pub users: Vec<User<Id>>,
    pub groups: Vec<Group<Id>>,
}

#[derive(Serialize)]
pub struct GroupMetadata {
    pub group: Group<Id>,
}

#[derive(Serialize)]
pub struct GroupModifiedMetadata {
    pub before: Group<Id>,
    pub after: Group<Id>,
}

#[derive(Serialize)]
pub struct GroupAssignedMetadata {
    pub group: Group<Id>,
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct WebHookMetadata {
    pub webhook: WebHook<Id>,
}

#[derive(Serialize)]
pub struct WebHookModifiedMetadata {
    pub before: WebHook<Id>,
    pub after: WebHook<Id>,
}

#[derive(Serialize)]
pub struct WebHookStateChangedMetadata {
    pub webhook: WebHook<Id>,
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct AuthenticationKeyMetadata {
    pub key: AuthenticationKeyNoSecrets,
}

#[derive(Serialize)]
pub struct AuthenticationKeyNoSecrets {
    pub id: Id,
    pub yubikey_id: Option<i64>,
    pub name: Option<String>,
    pub user_id: Id,
    pub key_type: AuthenticationKeyType,
}

impl From<AuthenticationKey<Id>> for AuthenticationKeyNoSecrets {
    fn from(value: AuthenticationKey<Id>) -> Self {
        Self {
            id: value.id,
            yubikey_id: value.yubikey_id,
            name: value.name,
            user_id: value.user_id,
            key_type: value.key_type,
        }
    }
}

#[derive(Serialize)]
pub struct AuthenticationKeyRenamedMetadata {
    pub key: AuthenticationKeyNoSecrets,
    pub old_name: Option<String>,
    pub new_name: Option<String>,
}

#[derive(Serialize)]
pub struct PasswordChangedByAdminMetadata {
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct PasswordResetMetadata {
    pub user: User<Id>,
}

#[derive(Serialize)]
pub struct ClientConfigurationTokenMetadata {
    pub user: User<Id>,
}
