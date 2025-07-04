use chrono::NaiveDateTime;

use crate::{
    db::{
        Device, Group, Id, MFAMethod, User, WebAuthn, WebHook, WireguardNetwork,
        models::{
            authentication_key::{AuthenticationKey, AuthenticationKeyType},
            oauth2client::OAuth2Client,
        },
    },
    enterprise::db::models::{
        activity_log_stream::{ActivityLogStream, ActivityLogStreamType},
        api_tokens::ApiToken,
        openid_provider::{DirectorySyncTarget, DirectorySyncUserBehavior, OpenIdProvider},
        snat::UserSnatBinding,
    },
    events::ClientMFAMethod,
};

#[derive(Serialize)]
pub struct LoginFailedMetadata {
    pub message: String,
}

#[derive(Serialize)]
pub struct MfaLoginMetadata {
    pub mfa_method: MFAMethod,
}

#[derive(Serialize)]
pub struct MfaLoginFailedMetadata {
    pub mfa_method: MFAMethod,
    pub message: String,
}

#[derive(Serialize)]
pub struct UserNoSecrets {
    pub id: Id,
    pub username: String,
    pub last_name: String,
    pub first_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub mfa_enabled: bool,
    pub is_active: bool,
    pub from_ldap: bool,
    pub ldap_pass_randomized: bool,
    pub ldap_rdn: Option<String>,
    pub openid_sub: Option<String>,
    pub totp_enabled: bool,
    pub email_mfa_enabled: bool,
    pub mfa_method: MFAMethod,
}

impl From<User<Id>> for UserNoSecrets {
    fn from(value: User<Id>) -> Self {
        Self {
            id: value.id,
            username: value.username,
            last_name: value.last_name,
            first_name: value.first_name,
            email: value.email,
            phone: value.phone,
            mfa_enabled: value.mfa_enabled,
            is_active: value.is_active,
            from_ldap: value.from_ldap,
            ldap_pass_randomized: value.ldap_pass_randomized,
            ldap_rdn: value.ldap_rdn,
            openid_sub: value.openid_sub,
            totp_enabled: value.totp_enabled,
            email_mfa_enabled: value.email_mfa_enabled,
            mfa_method: value.mfa_method,
        }
    }
}

#[derive(Serialize)]
pub struct DeviceMetadata {
    pub owner: UserNoSecrets,
    pub device: Device<Id>,
}

#[derive(Serialize)]
pub struct DeviceModifiedMetadata {
    pub owner: UserNoSecrets,
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
    pub user: UserNoSecrets,
}

#[derive(Serialize)]
pub struct UserModifiedMetadata {
    pub before: UserNoSecrets,
    pub after: UserNoSecrets,
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
pub struct ActivityLogStreamMetadata {
    pub stream: ActivityLogStreamNoSecrets,
}

#[derive(Serialize)]
pub struct ActivityLogStreamModifiedMetadata {
    pub before: ActivityLogStreamNoSecrets,
    pub after: ActivityLogStreamNoSecrets,
}

#[derive(Serialize)]
pub struct ActivityLogStreamNoSecrets {
    pub id: Id,
    pub name: String,
    pub stream_type: ActivityLogStreamType,
}

impl From<ActivityLogStream<Id>> for ActivityLogStreamNoSecrets {
    fn from(value: ActivityLogStream<Id>) -> Self {
        Self {
            id: value.id,
            name: value.name,
            stream_type: value.stream_type,
        }
    }
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
    pub method: ClientMFAMethod,
}

#[derive(Serialize)]
pub struct EnrollmentDeviceAddedMetadata {
    pub device: Device<Id>,
}

#[derive(Serialize)]
pub struct EnrollmentTokenMetadata {
    pub user: UserNoSecrets,
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
    pub owner: UserNoSecrets,
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
    pub owner: UserNoSecrets,
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
    pub users: Vec<UserNoSecrets>,
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
    pub user: UserNoSecrets,
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
    pub user: UserNoSecrets,
}

#[derive(Serialize)]
pub struct PasswordResetMetadata {
    pub user: UserNoSecrets,
}

#[derive(Serialize)]
pub struct UserMfaDisabledMetadata {
    pub user: UserNoSecrets,
}

#[derive(Serialize)]
pub struct ClientConfigurationTokenMetadata {
    pub user: UserNoSecrets,
}
#[derive(Serialize)]
pub struct UserSnatBindingMetadata {
    pub user: UserNoSecrets,
    pub binding: UserSnatBinding<Id>,
}

#[derive(Serialize)]
pub struct UserSnatBindingModifiedMetadata {
    pub user: UserNoSecrets,
    pub before: UserSnatBinding<Id>,
    pub after: UserSnatBinding<Id>,
}
