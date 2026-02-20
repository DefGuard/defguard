use chrono::NaiveDateTime;
use defguard_common::db::{
    Id,
    models::{
        AuthenticationKey, AuthenticationKeyType, Device, MFAMethod, Settings, WebAuthn,
        WireguardNetwork,
        gateway::Gateway,
        group::Group,
        oauth2client::OAuth2Client,
        proxy::Proxy,
        settings::{LdapSyncStatus, OpenIdUsernameHandling, SmtpEncryption},
        user::User,
    },
};

use crate::{
    db::WebHook,
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
pub struct UserGroupsModifiedMetadata {
    pub user: UserNoSecrets,
    pub before: Vec<String>,
    pub after: Vec<String>,
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
pub struct VpnClientMfaFailedMetadata {
    pub location: WireguardNetwork<Id>,
    pub device: Device<Id>,
    pub method: ClientMFAMethod,
    pub message: String,
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
pub struct SettingsUpdateMetadata {
    pub before: SettingsNoSecrets,
    pub after: SettingsNoSecrets,
}

#[derive(Serialize)]
pub struct SettingsNoSecrets {
    // Modules
    pub openid_enabled: bool,
    pub wireguard_enabled: bool,
    pub webhooks_enabled: bool,
    pub worker_enabled: bool,
    // MFA
    pub challenge_template: String,
    // Branding
    pub instance_name: String,
    pub main_logo_url: String,
    pub nav_logo_url: String,
    // SMTP
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    pub smtp_encryption: SmtpEncryption,
    pub smtp_user: Option<String>,
    pub smtp_sender: Option<String>,
    // Enrollment
    pub enrollment_vpn_step_optional: bool,
    pub enrollment_welcome_message: Option<String>,
    pub enrollment_welcome_email: Option<String>,
    pub enrollment_welcome_email_subject: Option<String>,
    pub enrollment_use_welcome_message_as_email: bool,
    // LDAP
    pub ldap_url: Option<String>,
    pub ldap_bind_username: Option<String>,
    pub ldap_group_search_base: Option<String>,
    pub ldap_user_search_base: Option<String>,
    // The structural user class
    pub ldap_user_obj_class: Option<String>,
    // The structural group class
    pub ldap_group_obj_class: Option<String>,
    pub ldap_username_attr: Option<String>,
    pub ldap_groupname_attr: Option<String>,
    pub ldap_group_member_attr: Option<String>,
    pub ldap_member_attr: Option<String>,
    pub ldap_use_starttls: bool,
    pub ldap_tls_verify_cert: bool,
    pub ldap_sync_status: LdapSyncStatus,
    pub ldap_enabled: bool,
    pub ldap_sync_enabled: bool,
    pub ldap_is_authoritative: bool,
    pub ldap_uses_ad: bool,
    pub ldap_sync_interval: i32,
    // Additional object classes for users which determine the added attributes
    pub ldap_user_auxiliary_obj_classes: Vec<String>,
    // The attribute which is used to map LDAP usernames to Defguard usernames
    pub ldap_user_rdn_attr: Option<String>,
    pub ldap_sync_groups: Vec<String>,
    // Whether to create a new account when users try to log in with external OpenID
    pub openid_create_account: bool,
    pub openid_username_handling: OpenIdUsernameHandling,
    pub license: Option<String>,
    // Gateway disconnect notifications
    pub gateway_disconnect_notifications_enabled: bool,
    pub gateway_disconnect_notifications_inactivity_threshold: i32,
    pub gateway_disconnect_notifications_reconnect_notification_enabled: bool,
}

impl From<Settings> for SettingsNoSecrets {
    fn from(value: Settings) -> Self {
        Self {
            openid_enabled: value.openid_enabled,
            wireguard_enabled: value.wireguard_enabled,
            webhooks_enabled: value.webhooks_enabled,
            worker_enabled: value.worker_enabled,
            challenge_template: value.challenge_template,
            instance_name: value.instance_name,
            main_logo_url: value.main_logo_url,
            nav_logo_url: value.nav_logo_url,
            smtp_server: value.smtp_server,
            smtp_port: value.smtp_port,
            smtp_encryption: value.smtp_encryption,
            smtp_user: value.smtp_user,
            smtp_sender: value.smtp_sender,
            enrollment_vpn_step_optional: value.enrollment_vpn_step_optional,
            enrollment_welcome_message: value.enrollment_welcome_message,
            enrollment_welcome_email: value.enrollment_welcome_email,
            enrollment_welcome_email_subject: value.enrollment_welcome_email_subject,
            enrollment_use_welcome_message_as_email: value.enrollment_use_welcome_message_as_email,
            ldap_url: value.ldap_url,
            ldap_bind_username: value.ldap_bind_username,
            ldap_group_search_base: value.ldap_group_search_base,
            ldap_user_search_base: value.ldap_user_search_base,
            ldap_user_obj_class: value.ldap_user_obj_class,
            ldap_group_obj_class: value.ldap_group_obj_class,
            ldap_username_attr: value.ldap_username_attr,
            ldap_groupname_attr: value.ldap_groupname_attr,
            ldap_group_member_attr: value.ldap_group_member_attr,
            ldap_member_attr: value.ldap_member_attr,
            ldap_use_starttls: value.ldap_use_starttls,
            ldap_tls_verify_cert: value.ldap_tls_verify_cert,
            ldap_sync_status: value.ldap_sync_status,
            ldap_enabled: value.ldap_enabled,
            ldap_sync_enabled: value.ldap_sync_enabled,
            ldap_is_authoritative: value.ldap_is_authoritative,
            ldap_uses_ad: value.ldap_uses_ad,
            ldap_sync_interval: value.ldap_sync_interval,
            ldap_user_auxiliary_obj_classes: value.ldap_user_auxiliary_obj_classes,
            ldap_user_rdn_attr: value.ldap_user_rdn_attr,
            ldap_sync_groups: value.ldap_sync_groups,
            openid_create_account: value.openid_create_account,
            openid_username_handling: value.openid_username_handling,
            license: value.license,
            gateway_disconnect_notifications_enabled: value
                .gateway_disconnect_notifications_enabled,
            gateway_disconnect_notifications_inactivity_threshold: value
                .gateway_disconnect_notifications_inactivity_threshold,
            gateway_disconnect_notifications_reconnect_notification_enabled: value
                .gateway_disconnect_notifications_reconnect_notification_enabled,
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
pub struct GroupMembersModifiedMetadata {
    pub group: Group<Id>,
    pub added: Vec<UserNoSecrets>,
    pub removed: Vec<UserNoSecrets>,
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

#[derive(Serialize)]
pub struct ProxyModifiedMetadata {
    pub before: Proxy<Id>,
    pub after: Proxy<Id>,
}

#[derive(Serialize)]
pub struct ProxyDeletedMetadata {
    pub proxy: Proxy<Id>,
}

#[derive(Serialize)]
pub struct GatewayModifiedMetadata {
    pub before: Gateway<Id>,
    pub after: Gateway<Id>,
}

#[derive(Serialize)]
pub struct GatewayDeletedMetadata {
    pub gateway: Gateway<Id>,
}
