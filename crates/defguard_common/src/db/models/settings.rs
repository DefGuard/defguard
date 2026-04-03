use std::{collections::HashMap, fmt, net::IpAddr, time::Duration};

use base64::{Engine, prelude::BASE64_STANDARD};
use rand::{RngCore, rngs::OsRng};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sqlx::{PgExecutor, PgPool, Type, query, query_as};
use struct_patch::Patch;
use thiserror::Error;
use tracing::{debug, info, warn};
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;
use webauthn_rs::prelude::WebauthnBuilder;

use crate::{
    config::DefGuardConfig, db::Id, global_value, secret::SecretStringWrapper, types::AuthFlowType,
};

global_value!(SETTINGS, Option<Settings>, None, set_settings, get_settings);

/// Initializes global `SETTINGS` struct at program startup
pub async fn initialize_current_settings(pool: &PgPool) -> sqlx::Result<()> {
    debug!("Initializing global settings struct");
    if let Some(settings) = Settings::get(pool).await? {
        set_settings(Some(settings));
    } else {
        debug!(
            "Settings not found in DB. Using default values to initialize global settings struct"
        );
        set_settings(Some(Settings::default()));
    }
    Ok(())
}

/// Helper function which stores updated `Settings` in the database and also updates the global
/// `SETTINGS` struct.
pub async fn update_current_settings<'e, E: sqlx::PgExecutor<'e>>(
    executor: E,
    mut new_settings: Settings,
) -> Result<(), SettingsSaveError> {
    debug!("Updating current settings to: {new_settings:?}");
    new_settings.validate()?;
    new_settings.save(executor).await?;
    set_settings(Some(new_settings));
    Ok(())
}

#[derive(Error, Debug)]
pub enum SettingsValidationError {
    #[error("Cannot enable gateway disconnect notifications. SMTP is not configured")]
    CannotEnableGatewayNotifications,
    #[error("Invalid defguard_url `{0}`, url has to be a domain, not IP")]
    InvalidDefguardUrl(String),
}

#[derive(Error, Debug)]
pub enum SettingsInitializationError {
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Save(#[from] SettingsSaveError),
    #[error("Missing required setting: {0}")]
    Missing(&'static str),
    #[error("Invalid required setting `{0}`: {1}")]
    Invalid(&'static str, &'static str),
}

#[derive(Error, Debug, Clone)]
pub enum SettingsUrlError {
    #[error("Unable to parse defguard_url `{0}`")]
    InvalidDefguardUrl(String),
    #[error("Unable to derive webauthn_rp_id: defguard_url has no host: {0}")]
    MissingDefguardHost(String),
    #[error("Unable to derive webauthn_rp_id: defguard_url has no domain: {0}")]
    MissingDefguardDomain(String),
    #[error("defguard_url cannot use an IP address host: {0}")]
    DefguardUrlUsesIpAddress(String),
    #[error("Invalid WebAuthn configuration for defguard_url `{0}`: {1}")]
    InvalidWebauthnConfiguration(String, String),
}

#[derive(Error, Debug)]
pub enum SettingsSaveError {
    #[error(transparent)]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Validation(#[from] SettingsValidationError),
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Type, Debug, Default)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    #[default]
    None,
    StartTls,
    ImplicitTls,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Serialize, PartialEq, ToSchema, Type)]
#[sqlx(type_name = "openid_username_handling", rename_all = "snake_case")]
pub enum OpenIdUsernameHandling {
    #[default]
    /// Removes all forbidden characters
    RemoveForbidden,
    /// Replaces all forbidden characters with `_`
    ReplaceForbidden,
    /// Removes the email domain, replaces all other forbidden characters with `_`
    PruneEmailDomain,
}

#[derive(Clone, Debug, Copy, Eq, PartialEq, Deserialize, Serialize, Default, Type)]
#[sqlx(type_name = "ldap_sync_status", rename_all = "lowercase")]
pub enum LdapSyncStatus {
    InSync,
    #[default]
    OutOfSync,
}

impl LdapSyncStatus {
    #[must_use]
    pub fn is_out_of_sync(&self) -> bool {
        matches!(self, LdapSyncStatus::OutOfSync)
    }
}

#[derive(Clone, Deserialize, PartialEq, Patch, Serialize, Default)]
#[patch(attribute(derive(Deserialize, Serialize, Debug)))]
pub struct Settings {
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
    pub smtp_password: Option<SecretStringWrapper>,
    pub smtp_sender: Option<String>,
    // Enrollment
    pub enrollment_vpn_step_optional: bool,
    pub enrollment_welcome_message: Option<String>,
    pub enrollment_welcome_email: Option<String>,
    pub enrollment_welcome_email_subject: Option<String>,
    pub enrollment_use_welcome_message_as_email: bool,
    pub enrollment_send_welcome_email: bool,
    // Instance UUID needed for desktop client
    #[serde(skip)]
    pub uuid: Uuid,
    // LDAP
    pub ldap_url: Option<String>,
    pub ldap_bind_username: Option<String>,
    pub ldap_bind_password: Option<SecretStringWrapper>,
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
    pub ldap_remote_enrollment_enabled: bool,
    pub ldap_remote_enrollment_send_invite: bool,
    // Whether to create a new account when users try to log in with external OpenID
    pub openid_create_account: bool,
    pub openid_username_handling: OpenIdUsernameHandling,
    pub license: Option<String>,
    // Gateway disconnect notifications
    pub gateway_disconnect_notifications_enabled: bool,
    pub gateway_disconnect_notifications_inactivity_threshold: i32,
    pub gateway_disconnect_notifications_reconnect_notification_enabled: bool,
    // General settings
    pub defguard_url: String,
    pub default_admin_group_name: String,
    pub authentication_period_days: i32,
    pub mfa_code_timeout_seconds: i32,
    pub public_proxy_url: String,
    pub default_admin_id: Option<Id>,
    // 1.6 config options
    pub secret_key: Option<String>,
    pub enable_stats_purge: bool,
    stats_purge_frequency_hours: i32,
    stats_purge_threshold_days: i32,
    enrollment_token_timeout_hours: i32,
    password_reset_token_timeout_hours: i32,
    enrollment_session_timeout_minutes: i32,
    password_reset_session_timeout_minutes: i32,
}

// Implement manually to avoid exposing the license key.
impl fmt::Debug for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Settings")
            .field("openid_enabled", &self.openid_enabled)
            .field("wireguard_enabled", &self.wireguard_enabled)
            .field("webhooks_enabled", &self.webhooks_enabled)
            .field("worker_enabled", &self.worker_enabled)
            .field("challenge_template", &self.challenge_template)
            .field("instance_name", &self.instance_name)
            .field("main_logo_url", &self.main_logo_url)
            .field("nav_logo_url", &self.nav_logo_url)
            .field("smtp_server", &self.smtp_server)
            .field("smtp_port", &self.smtp_port)
            .field("smtp_encryption", &self.smtp_encryption)
            .field("smtp_user", &self.smtp_user)
            .field("smtp_password", &self.smtp_password)
            .field("smtp_sender", &self.smtp_sender)
            .field(
                "enrollment_vpn_step_optional",
                &self.enrollment_vpn_step_optional,
            )
            .field(
                "enrollment_welcome_message",
                &self.enrollment_welcome_message,
            )
            .field("enrollment_welcome_email", &self.enrollment_welcome_email)
            .field(
                "enrollment_welcome_email_subject",
                &self.enrollment_welcome_email_subject,
            )
            .field(
                "enrollment_use_welcome_message_as_email",
                &self.enrollment_use_welcome_message_as_email,
            )
            .field(
                "enrollment_send_welcome_email",
                &self.enrollment_send_welcome_email,
            )
            .field("uuid", &self.uuid)
            .field("ldap_url", &self.ldap_url)
            .field("ldap_bind_username", &self.ldap_bind_username)
            .field("ldap_bind_password", &self.ldap_bind_password)
            .field("ldap_group_search_base", &self.ldap_group_search_base)
            .field("ldap_user_search_base", &self.ldap_user_search_base)
            .field("ldap_user_obj_class", &self.ldap_user_obj_class)
            .field("ldap_group_obj_class", &self.ldap_group_obj_class)
            .field("ldap_username_attr", &self.ldap_username_attr)
            .field("ldap_groupname_attr", &self.ldap_groupname_attr)
            .field("ldap_group_member_attr", &self.ldap_group_member_attr)
            .field("ldap_member_attr", &self.ldap_member_attr)
            .field("ldap_use_starttls", &self.ldap_use_starttls)
            .field("ldap_tls_verify_cert", &self.ldap_tls_verify_cert)
            .field("ldap_sync_status", &self.ldap_sync_status)
            .field("ldap_enabled", &self.ldap_enabled)
            .field("ldap_sync_enabled", &self.ldap_sync_enabled)
            .field("ldap_is_authoritative", &self.ldap_is_authoritative)
            .field("ldap_uses_ad", &self.ldap_uses_ad)
            .field("ldap_sync_interval", &self.ldap_sync_interval)
            .field(
                "ldap_user_auxiliary_obj_classes",
                &self.ldap_user_auxiliary_obj_classes,
            )
            .field("ldap_user_rdn_attr", &self.ldap_user_rdn_attr)
            .field("ldap_sync_groups", &self.ldap_sync_groups)
            .field("openid_create_account", &self.openid_create_account)
            .field("openid_username_handling", &self.openid_username_handling)
            .field(
                "gateway_disconnect_notifications_enabled",
                &self.gateway_disconnect_notifications_enabled,
            )
            .field(
                "gateway_disconnect_notifications_inactivity_threshold",
                &self.gateway_disconnect_notifications_inactivity_threshold,
            )
            .field(
                "gateway_disconnect_notifications_reconnect_notification_enabled",
                &self.gateway_disconnect_notifications_reconnect_notification_enabled,
            )
            .field("defguard_url", &self.defguard_url)
            .field("default_admin_group_name", &self.default_admin_group_name)
            .field(
                "authentication_period_days",
                &self.authentication_period_days,
            )
            .field("mfa_code_timeout_seconds", &self.mfa_code_timeout_seconds)
            .field("public_proxy_url", &self.public_proxy_url)
            .field("default_admin_id", &self.default_admin_id)
            .finish_non_exhaustive()
    }
}

impl Settings {
    pub(crate) fn validate_secret_key(secret_key: &str) -> Result<(), SettingsInitializationError> {
        if secret_key.trim().len() != secret_key.len() {
            return Err(SettingsInitializationError::Invalid(
                "secret_key",
                "cannot have leading or trailing whitespace",
            ));
        }

        if secret_key.len() < 64 {
            return Err(SettingsInitializationError::Invalid(
                "secret_key",
                "must be at least 64 characters long",
            ));
        }

        Ok(())
    }

    /// Generates length 64 random base64 string.
    fn generate_secret_key() -> String {
        let mut bytes = [0_u8; 48];
        OsRng.fill_bytes(&mut bytes);
        BASE64_STANDARD.encode(bytes)
    }

    /// Parse `defguard_url` and reject unsupported host forms.
    fn parse_defguard_url(&self) -> Result<Url, SettingsUrlError> {
        let url = Url::parse(&self.defguard_url)
            .map_err(|_| SettingsUrlError::InvalidDefguardUrl(self.defguard_url.clone()))?;
        let host = url
            .host_str()
            .ok_or_else(|| SettingsUrlError::MissingDefguardHost(self.defguard_url.clone()))?;
        if host.parse::<IpAddr>().is_ok() {
            return Err(SettingsUrlError::DefguardUrlUsesIpAddress(
                self.defguard_url.clone(),
            ));
        }
        Ok(url)
    }

    /// Derive the WebAuthn relying party ID from `defguard_url`.
    fn webauthn_rp_id(&self) -> Result<String, SettingsUrlError> {
        let url = self.parse_defguard_url()?;
        let domain = url
            .domain()
            .map(str::to_string)
            .or_else(|| match url.host_str() {
                Some("localhost") => Some("localhost".to_string()),
                _ => None,
            });

        domain.ok_or_else(|| SettingsUrlError::MissingDefguardDomain(self.defguard_url.clone()))
    }

    /// Derive the cookie domain from `defguard_url`.
    pub fn cookie_domain(&self) -> Result<String, SettingsUrlError> {
        let url = self.parse_defguard_url()?;
        url.host_str()
            .map(ToString::to_string)
            .ok_or_else(|| SettingsUrlError::MissingDefguardHost(self.defguard_url.clone()))
    }

    /// Build a WebAuthn configuration from the current Defguard URL.
    pub fn build_webauthn(&self) -> Result<webauthn_rs::Webauthn, SettingsUrlError> {
        let url = self.parse_defguard_url()?;
        let rp_id = self.webauthn_rp_id()?;
        let builder = WebauthnBuilder::new(&rp_id, &url).map_err(|err| {
            SettingsUrlError::InvalidWebauthnConfiguration(
                self.defguard_url.clone(),
                err.to_string(),
            )
        })?;
        builder.build().map_err(|err| {
            SettingsUrlError::InvalidWebauthnConfiguration(
                self.defguard_url.clone(),
                err.to_string(),
            )
        })
    }

    pub async fn get<'e, E>(executor: E) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT openid_enabled, wireguard_enabled, webhooks_enabled, worker_enabled, \
            challenge_template, instance_name, main_logo_url, nav_logo_url, smtp_server, \
            smtp_port, smtp_encryption \"smtp_encryption: _\", smtp_user, \
            smtp_password \"smtp_password?: SecretStringWrapper\", smtp_sender, \
            enrollment_vpn_step_optional, enrollment_welcome_message, \
            enrollment_welcome_email, enrollment_welcome_email_subject, \
            enrollment_use_welcome_message_as_email, enrollment_send_welcome_email, \
            uuid, ldap_url, ldap_bind_username, \
            ldap_bind_password \"ldap_bind_password?: SecretStringWrapper\", \
            ldap_group_search_base, ldap_user_search_base, ldap_user_obj_class, \
            ldap_group_obj_class, ldap_username_attr, ldap_groupname_attr, \
            ldap_group_member_attr, ldap_member_attr, openid_create_account, \
            license, gateway_disconnect_notifications_enabled, ldap_use_starttls, \
            ldap_tls_verify_cert, gateway_disconnect_notifications_inactivity_threshold, \
            gateway_disconnect_notifications_reconnect_notification_enabled, \
            ldap_sync_status \"ldap_sync_status: LdapSyncStatus\", \
            ldap_enabled, ldap_sync_enabled, ldap_is_authoritative, \
            ldap_sync_interval, ldap_user_auxiliary_obj_classes, ldap_uses_ad, \
            ldap_user_rdn_attr, ldap_sync_groups, ldap_remote_enrollment_enabled, ldap_remote_enrollment_send_invite, \
            openid_username_handling \"openid_username_handling: OpenIdUsernameHandling\", \
            defguard_url, \
            default_admin_group_name, authentication_period_days, mfa_code_timeout_seconds, \
            public_proxy_url, \
            default_admin_id, secret_key, enable_stats_purge, \
            stats_purge_frequency_hours, stats_purge_threshold_days, \
            enrollment_token_timeout_hours, password_reset_token_timeout_hours, \
            enrollment_session_timeout_minutes, password_reset_session_timeout_minutes \
            FROM \"settings\" WHERE id = 1",
        )
        .fetch_optional(executor)
        .await
    }

    /// Checks if given settings are correct
    pub fn validate(&mut self) -> Result<(), SettingsValidationError> {
        debug!("Validating settings: {self:?}");
        if self.uuid.is_nil() {
            warn!("Detected empty UUID in settings. Generating a new one.");
            self.uuid = Uuid::new_v4();
        }
        self.build_webauthn()
            .map_err(|_| SettingsValidationError::InvalidDefguardUrl(self.defguard_url.clone()))?;
        // Check if gateway disconnect notifications can be enabled, since it requires SMTP to be
        // configured.
        if self.gateway_disconnect_notifications_enabled && !self.smtp_configured() {
            warn!("Cannot enable gateway disconnect notifications. SMTP is not configured.");
            return Err(SettingsValidationError::CannotEnableGatewayNotifications);
        }

        Ok(())
    }

    pub async fn save<'e, E>(&self, executor: E) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE \"settings\" SET \
            openid_enabled = $1, \
            wireguard_enabled = $2, \
            webhooks_enabled = $3, \
            worker_enabled = $4, \
            challenge_template = $5, \
            instance_name = $6, \
            main_logo_url = $7, \
            nav_logo_url = $8, \
            smtp_server = $9, \
            smtp_port = $10, \
            smtp_encryption = $11, \
            smtp_user = $12, \
            smtp_password = $13, \
            smtp_sender = $14, \
            enrollment_vpn_step_optional = $15, \
            enrollment_welcome_message = $16, \
            enrollment_welcome_email = $17, \
            enrollment_welcome_email_subject = $18, \
            enrollment_use_welcome_message_as_email = $19, \
            enrollment_send_welcome_email = $20, \
            uuid = $21, \
            ldap_url = $22, \
            ldap_bind_username = $23, \
            ldap_bind_password  = $24, \
            ldap_group_search_base = $25, \
            ldap_user_search_base = $26, \
            ldap_user_obj_class = $27, \
            ldap_group_obj_class = $28, \
            ldap_username_attr = $29, \
            ldap_groupname_attr = $30, \
            ldap_group_member_attr = $31, \
            ldap_member_attr = $32, \
            ldap_use_starttls = $33, \
            ldap_tls_verify_cert = $34, \
            openid_create_account = $35, \
            license = $36, \
            gateway_disconnect_notifications_enabled = $37, \
            gateway_disconnect_notifications_inactivity_threshold = $38, \
            gateway_disconnect_notifications_reconnect_notification_enabled = $39, \
            ldap_sync_status = $40, \
            ldap_enabled = $41, \
            ldap_sync_enabled = $42, \
            ldap_is_authoritative = $43, \
            ldap_sync_interval = $44, \
            ldap_user_auxiliary_obj_classes = $45, \
            ldap_uses_ad = $46, \
            ldap_user_rdn_attr = $47, \
            ldap_sync_groups = $48, \
            openid_username_handling = $49, \
            defguard_url = $50, \
            default_admin_group_name = $51, \
            authentication_period_days = $52, \
            mfa_code_timeout_seconds = $53, \
            public_proxy_url = $54, \
            default_admin_id = $55, \
            secret_key = $56, \
            enable_stats_purge = $57, \
            stats_purge_frequency_hours = $58, \
            stats_purge_threshold_days = $59, \
            enrollment_token_timeout_hours = $60, \
            password_reset_token_timeout_hours = $61, \
            enrollment_session_timeout_minutes = $62, \
            password_reset_session_timeout_minutes = $63 \
            WHERE id = 1",
            self.openid_enabled,
            self.wireguard_enabled,
            self.webhooks_enabled,
            self.worker_enabled,
            self.challenge_template,
            self.instance_name,
            self.main_logo_url,
            self.nav_logo_url,
            self.smtp_server,
            self.smtp_port,
            &self.smtp_encryption as &SmtpEncryption,
            self.smtp_user,
            &self.smtp_password as &Option<SecretStringWrapper>,
            self.smtp_sender,
            self.enrollment_vpn_step_optional,
            self.enrollment_welcome_message,
            self.enrollment_welcome_email,
            self.enrollment_welcome_email_subject,
            self.enrollment_use_welcome_message_as_email,
            self.enrollment_send_welcome_email,
            self.uuid,
            self.ldap_url,
            self.ldap_bind_username,
            &self.ldap_bind_password as &Option<SecretStringWrapper>,
            self.ldap_group_search_base,
            self.ldap_user_search_base,
            self.ldap_user_obj_class,
            self.ldap_group_obj_class,
            self.ldap_username_attr,
            self.ldap_groupname_attr,
            self.ldap_group_member_attr,
            self.ldap_member_attr,
            self.ldap_use_starttls,
            self.ldap_tls_verify_cert,
            self.openid_create_account,
            self.license,
            self.gateway_disconnect_notifications_enabled,
            self.gateway_disconnect_notifications_inactivity_threshold,
            self.gateway_disconnect_notifications_reconnect_notification_enabled,
            &self.ldap_sync_status as &LdapSyncStatus,
            self.ldap_enabled,
            self.ldap_sync_enabled,
            self.ldap_is_authoritative,
            self.ldap_sync_interval,
            &self.ldap_user_auxiliary_obj_classes as &Vec<String>,
            self.ldap_uses_ad,
            self.ldap_user_rdn_attr,
            &self.ldap_sync_groups as &Vec<String>,
            &self.openid_username_handling as &OpenIdUsernameHandling,
            self.defguard_url,
            self.default_admin_group_name,
            self.authentication_period_days,
            self.mfa_code_timeout_seconds,
            self.public_proxy_url,
            self.default_admin_id,
            self.secret_key,
            self.enable_stats_purge,
            self.stats_purge_frequency_hours,
            self.stats_purge_threshold_days,
            self.enrollment_token_timeout_hours,
            self.password_reset_token_timeout_hours,
            self.enrollment_session_timeout_minutes,
            self.password_reset_session_timeout_minutes,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    #[must_use]
    pub fn get_current_settings() -> Self {
        // fetch global settings
        let maybe_settings = get_settings().clone();

        // panic if settings have not been initialized, since it should happen at startup
        maybe_settings.expect("Global settings have not been initialized")
    }

    // Set default values for settings if not set yet.
    // This is only relevant to a subset of settings which are nullable
    // and we want to initialize their values.
    pub async fn initialize_runtime_defaults(
        pool: &PgPool,
    ) -> Result<(), SettingsInitializationError> {
        info!("Initializing runtime default settings");

        let default_settings = HashMap::from([
            ("enrollment_welcome_message", defaults::WELCOME_MESSAGE),
            ("enrollment_welcome_email", defaults::WELCOME_MESSAGE),
            (
                "enrollment_welcome_email_subject",
                defaults::WELCOME_EMAIL_SUBJECT,
            ),
        ]);

        for (field, value) in default_settings {
            let query_string = format!("UPDATE settings SET {field} = $1 WHERE {field} IS NULL");
            query(&query_string).bind(value).execute(pool).await?;
        }

        let mut settings = Settings::get(pool).await?.unwrap_or_default();

        match settings.secret_key.as_deref() {
            Some(secret_key) => {
                Settings::validate_secret_key(secret_key)?;
            }
            None => {
                settings.secret_key = Some(Settings::generate_secret_key());
            }
        }

        update_current_settings(pool, settings).await?;

        Ok(())
    }

    /// Check if all required SMTP options are configured.
    /// User & password can be empty for no-auth servers.
    ///
    /// Meant to be used to check if sending emails is enabled in current instance.
    #[must_use]
    pub fn smtp_configured(&self) -> bool {
        self.smtp_server.is_some()
            && self.smtp_port.is_some()
            && self.smtp_sender.is_some()
            && self.smtp_server != Some(String::new())
            && self.smtp_sender != Some(String::new())
    }

    /// Check if all required LDAP options are configured.
    ///
    /// Meant to be used to check if LDAP integration can be enabled.
    #[must_use]
    pub fn ldap_configured(&self) -> bool {
        let non_empty = |opt: &Option<String>| opt.as_deref().is_some_and(|s| !s.is_empty());
        non_empty(&self.ldap_url)
            && non_empty(&self.ldap_bind_username)
            && self.ldap_bind_password.is_some()  // just check the presence, don't expose the secret
            && non_empty(&self.ldap_username_attr)
            && non_empty(&self.ldap_user_search_base)
            && non_empty(&self.ldap_user_obj_class)
            && non_empty(&self.ldap_member_attr)
            && non_empty(&self.ldap_groupname_attr)
            && non_empty(&self.ldap_group_obj_class)
            && non_empty(&self.ldap_group_member_attr)
            && non_empty(&self.ldap_group_search_base)
    }

    #[must_use]
    pub fn ldap_using_username_as_rdn(&self) -> bool {
        self.ldap_user_rdn_attr
            .as_deref()
            .is_none_or(|rdn| rdn.is_empty() || Some(rdn) == self.ldap_username_attr.as_deref())
    }

    /// Get the DefGuard URL from the current settings
    pub fn url() -> Result<Url, url::ParseError> {
        let settings = Settings::get_current_settings();
        Url::parse(&settings.defguard_url)
    }

    /// Returns configured URL with "auth/callback" appended to the path.
    pub fn callback_url(&self) -> Result<Url, url::ParseError> {
        let mut url = Url::parse(&self.defguard_url)?;
        // Append "auth/callback" to the URL.
        if let Ok(mut path_segments) = url.path_segments_mut() {
            path_segments.extend(&["auth", "callback"]);
        }
        Ok(url)
    }

    #[must_use]
    pub fn authentication_timeout(&self) -> Duration {
        Duration::from_secs(self.authentication_period_days as u64 * 24 * 3600)
    }

    #[must_use]
    pub fn stats_purge_frequency(&self) -> Duration {
        Duration::from_secs(self.stats_purge_frequency_hours as u64 * 3600)
    }

    #[must_use]
    pub fn stats_purge_threshold(&self) -> Duration {
        Duration::from_secs(self.stats_purge_threshold_days as u64 * 24 * 3600)
    }

    #[must_use]
    pub fn enrollment_token_timeout(&self) -> Duration {
        Duration::from_secs(self.enrollment_token_timeout_hours as u64 * 3600)
    }

    #[must_use]
    pub fn password_reset_token_timeout(&self) -> Duration {
        Duration::from_secs(self.password_reset_token_timeout_hours as u64 * 3600)
    }

    #[must_use]
    pub fn enrollment_session_timeout(&self) -> Duration {
        Duration::from_secs(self.enrollment_session_timeout_minutes as u64 * 60)
    }

    #[must_use]
    pub fn password_reset_session_timeout(&self) -> Duration {
        Duration::from_secs(self.password_reset_session_timeout_minutes as u64 * 60)
    }

    pub fn secret_key_required(&self) -> Result<&str, SettingsInitializationError> {
        let secret_key = self
            .secret_key
            .as_deref()
            .ok_or(SettingsInitializationError::Missing("secret_key"))?;

        Settings::validate_secret_key(secret_key)?;

        Ok(secret_key)
    }

    pub fn proxy_public_url(&self) -> Result<Url, url::ParseError> {
        Url::parse(&self.public_proxy_url)
    }

    #[allow(deprecated)]
    fn apply_from_config(&mut self, config: &DefGuardConfig) {
        let minute = 60;
        let hour = minute * 60;
        let day = hour * 24;

        if let Some(url) = &config.url {
            self.defguard_url = url.to_string();
        }
        if let Some(secret_key) = &config.secret_key {
            let secret_key = secret_key.expose_secret();
            if let Err(err) = Settings::validate_secret_key(secret_key) {
                warn!(
                    "Invalid secret_key provided in deprecated config, generating new one: {err}"
                );
                self.secret_key = Some(Settings::generate_secret_key());
            } else {
                self.secret_key = Some(secret_key.to_string());
            }
        }
        if let Some(enrollment_url) = &config.enrollment_url {
            self.public_proxy_url = enrollment_url.to_string();
        }
        if let Some(mfa_code_timeout) = config.mfa_code_timeout {
            self.mfa_code_timeout_seconds = mfa_code_timeout.as_secs() as i32;
        }
        if let Some(session_timeout) = config.session_timeout {
            self.authentication_period_days = (session_timeout.as_secs() / day) as i32;
        }
        if let Some(disable_stats_purge) = config.disable_stats_purge {
            self.enable_stats_purge = !disable_stats_purge;
        }
        if let Some(stats_purge_frequency) = config.stats_purge_frequency {
            self.stats_purge_frequency_hours = (stats_purge_frequency.as_secs() / hour) as i32;
        }
        if let Some(stats_purge_threshold) = config.stats_purge_threshold {
            self.stats_purge_threshold_days = (stats_purge_threshold.as_secs() / day) as i32;
        }
        if let Some(enrollment_token_timeout) = config.enrollment_token_timeout {
            self.enrollment_token_timeout_hours =
                (enrollment_token_timeout.as_secs() / hour) as i32;
        }
        if let Some(password_reset_token_timeout) = config.password_reset_token_timeout {
            self.password_reset_token_timeout_hours =
                (password_reset_token_timeout.as_secs() / hour) as i32;
        }
        if let Some(enrollment_session_timeout) = config.enrollment_session_timeout {
            self.enrollment_session_timeout_minutes =
                (enrollment_session_timeout.as_secs() / minute) as i32;
        }
        if let Some(password_reset_session_timeout) = config.password_reset_session_timeout {
            self.password_reset_session_timeout_minutes =
                (password_reset_session_timeout.as_secs() / minute) as i32;
        }
    }

    pub async fn update_from_config<'e, E>(
        &mut self,
        executor: E,
        config: &DefGuardConfig,
    ) -> Result<(), SettingsSaveError>
    where
        E: PgExecutor<'e>,
    {
        info!("Updating Settings from DefguardConfig: {config:?}");
        self.apply_from_config(config);

        update_current_settings(executor, self.clone()).await?;

        info!("Updated Settings from DefguardConfig: {config:?}");
        Ok(())
    }

    /// Returns configured Edge Component URL with the correct callback path appended depending on auth flow type.
    pub fn edge_callback_url(&self, auth_flow_type: AuthFlowType) -> Result<Url, url::ParseError> {
        let mut url = self.proxy_public_url()?;
        // Append callback segments to the URL.
        if let Ok(mut path_segments) = url.path_segments_mut() {
            match auth_flow_type {
                AuthFlowType::Enrollment => path_segments.extend(&["openid", "callback"]),
                AuthFlowType::Mfa => path_segments.extend(&["openid", "mfa", "callback"]),
            };
        }
        Ok(url)
    }
}

#[derive(Serialize)]
pub struct SettingsEssentials {
    pub instance_name: String,
    pub main_logo_url: String,
    pub nav_logo_url: String,
    pub wireguard_enabled: bool,
    pub webhooks_enabled: bool,
    pub worker_enabled: bool,
    pub openid_enabled: bool,
}

impl SettingsEssentials {
    pub async fn get_settings_essentials<'e, E>(executor: E) -> sqlx::Result<Self>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            SettingsEssentials,
            "SELECT s.instance_name, s.main_logo_url, s.nav_logo_url, s.wireguard_enabled, \
            s.webhooks_enabled, s.worker_enabled, s.openid_enabled \
			FROM settings s \
			WHERE s.id = 1 \
			LIMIT 1"
        )
        .fetch_one(executor)
        .await
    }
}

pub mod defaults {
    pub static WELCOME_MESSAGE: &str = "Dear {{ first_name }} {{ last_name }},

By completing the enrollment process, you now have access to all company systems.

Your login to all systems is: {{ username }}

## Company systems

Here are the most important company systems:

- Defguard: {{ defguard_url }} - where you can change your password and manage your VPN devices
- our chat system: https://chat.example.com - join our default room #TownHall
- knowledge base: https://example.com ...
- our JIRA: https://example.atlassian.net...

## Governance

To kickoff your onboarding, please get familiar with:

- our employee handbook: https://knowledgebase.example.com/Welcome
- security policy: https://knowledgebase.example.com/security

If you have any questions contact our HR:
John Hary - mobile +48 123 123 123

The person that enrolled you is:
{{ admin_first_name }} {{ admin_last_name }},
email: {{ admin_email }}
mobile: {{ admin_phone }}

--
Sent by Defguard {{ defguard_version }}
Star us on GitHub! https://github.com/defguard/defguard\
";

    pub static WELCOME_EMAIL_SUBJECT: &str = "Defguard: Welcome message after enrollment";
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use humantime::Duration;
    use reqwest::Url;
    use secrecy::SecretString;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::db::setup_pool;

    #[test]
    fn test_smtp_config() {
        let mut settings = Settings::default();
        assert!(!settings.smtp_configured());

        // incomplete SMTP config
        settings.smtp_server = Some("localhost".into());
        settings.smtp_port = Some(587);
        assert!(!settings.smtp_configured());

        // no-auth SMTP config
        settings.smtp_sender = Some("no-reply@defguard.net".into());
        assert!(settings.smtp_configured());

        // add non-default encryption
        settings.smtp_encryption = SmtpEncryption::StartTls;
        assert!(settings.smtp_configured());

        // add auth info
        settings.smtp_user = Some("smtp_user".into());
        settings.smtp_password = Some(SecretStringWrapper::from_str("hunter2").unwrap());
        assert!(settings.smtp_configured());
    }

    #[test]
    fn dg25_32_test_dont_expose_license_key() {
        let key = "0000000000000000";
        let settings = Settings {
            license: Some(key.to_string()),
            ..Default::default()
        };

        let debug = format!("{settings:?}");
        assert!(!debug.contains("license"));
        assert!(!debug.contains(key));
    }

    #[test]
    fn test_callback_url() {
        let mut s = Settings {
            defguard_url: "https://defguard.example.com".into(),
            ..Default::default()
        };
        assert_eq!(
            s.callback_url().unwrap().as_str(),
            "https://defguard.example.com/auth/callback"
        );

        s.defguard_url = "https://defguard.example.com:8443/path".into();
        assert_eq!(
            s.callback_url().unwrap().as_str(),
            "https://defguard.example.com:8443/path/auth/callback"
        );
    }

    #[test]
    #[allow(deprecated)]
    fn test_apply_from_config_maps_migrated_fields() {
        let mut settings = Settings {
            defguard_url: "https://defguard.example.com".into(),
            ..Default::default()
        };
        let mut config = DefGuardConfig::new_test_config();

        config.secret_key = Some(SecretString::from("a".repeat(64)));
        config.enrollment_url = Some(Url::parse("https://proxy.example.com").unwrap());
        config.mfa_code_timeout = Some(Duration::from(std::time::Duration::from_secs(75)));
        config.session_timeout = Some(Duration::from(std::time::Duration::from_secs(
            10 * 24 * 3600,
        )));
        config.disable_stats_purge = Some(true);
        config.stats_purge_frequency =
            Some(Duration::from(std::time::Duration::from_secs(5 * 3600)));
        config.stats_purge_threshold = Some(Duration::from(std::time::Duration::from_secs(
            12 * 24 * 3600,
        )));
        config.enrollment_token_timeout =
            Some(Duration::from(std::time::Duration::from_secs(7 * 3600)));
        config.password_reset_token_timeout =
            Some(Duration::from(std::time::Duration::from_secs(9 * 3600)));
        config.enrollment_session_timeout =
            Some(Duration::from(std::time::Duration::from_secs(15 * 60)));
        config.password_reset_session_timeout =
            Some(Duration::from(std::time::Duration::from_secs(20 * 60)));

        settings.apply_from_config(&config);

        assert_eq!(
            settings.secret_key.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
        assert_eq!(settings.webauthn_rp_id().unwrap(), "defguard.example.com");
        assert_eq!(settings.public_proxy_url, "https://proxy.example.com/");
        assert_eq!(settings.mfa_code_timeout_seconds, 75);
        assert_eq!(settings.authentication_period_days, 10);
        assert!(!settings.enable_stats_purge);
        assert_eq!(settings.stats_purge_frequency_hours, 5);
        assert_eq!(settings.stats_purge_threshold_days, 12);
        assert_eq!(settings.enrollment_token_timeout_hours, 7);
        assert_eq!(settings.password_reset_token_timeout_hours, 9);
        assert_eq!(settings.enrollment_session_timeout_minutes, 15);
        assert_eq!(settings.password_reset_session_timeout_minutes, 20);
    }

    #[test]
    fn test_apply_from_config_keeps_values_when_config_is_none() {
        let mut settings = Settings {
            defguard_url: "https://defguard.example.com".into(),
            secret_key: Some("z".repeat(64)),
            public_proxy_url: "https://proxy.initial".into(),
            mfa_code_timeout_seconds: 123,
            authentication_period_days: 9,
            enable_stats_purge: false,
            ..Default::default()
        };
        let config = DefGuardConfig::new_test_config();
        let existing_secret = "z".repeat(64);

        settings.apply_from_config(&config);

        assert_eq!(
            settings.secret_key.as_deref(),
            Some(existing_secret.as_str())
        );
        assert_eq!(settings.webauthn_rp_id().unwrap(), "defguard.example.com");
        assert_eq!(settings.public_proxy_url, "https://proxy.initial");
        assert_eq!(settings.mfa_code_timeout_seconds, 123);
        assert_eq!(settings.authentication_period_days, 9);
        assert!(!settings.enable_stats_purge);
    }

    #[test]
    fn test_webauthn_rp_id_rejects_invalid_defguard_url() {
        let mut settings = Settings {
            defguard_url: "this is not an url".into(),
            ..Default::default()
        };
        let config = DefGuardConfig::new_test_config();

        settings.apply_from_config(&config);

        assert!(matches!(
            settings.webauthn_rp_id(),
            Err(SettingsUrlError::InvalidDefguardUrl(_))
        ));
    }

    #[test]
    fn test_parse_defguard_url_parses_valid_hostname_url() {
        let settings = Settings {
            defguard_url: "https://defguard.example.com:8443/path".into(),
            ..Default::default()
        };

        let url = settings.parse_defguard_url().unwrap();

        assert_eq!(url.host_str(), Some("defguard.example.com"));
        assert_eq!(url.port(), Some(8443));
        assert_eq!(url.path(), "/path");
    }

    #[test]
    fn test_parse_defguard_url_rejects_ip_host() {
        let settings = Settings {
            defguard_url: "http://127.0.0.1:8000".into(),
            ..Default::default()
        };

        assert!(matches!(
            settings.parse_defguard_url(),
            Err(SettingsUrlError::DefguardUrlUsesIpAddress(_))
        ));
    }

    #[test]
    fn test_cookie_domain_derives_from_defguard_url() {
        let settings = Settings {
            defguard_url: "https://defguard.example.com:8443/path".into(),
            ..Default::default()
        };

        assert_eq!(settings.cookie_domain().unwrap(), "defguard.example.com");
    }

    #[test]
    fn test_cookie_domain_allows_localhost() {
        let settings = Settings {
            defguard_url: "http://localhost:8000".into(),
            ..Default::default()
        };

        assert_eq!(settings.cookie_domain().unwrap(), "localhost");
    }

    #[test]
    fn test_cookie_domain_rejects_ip_hosts() {
        let settings = Settings {
            defguard_url: "http://127.0.0.1:8000".into(),
            ..Default::default()
        };

        assert!(matches!(
            settings.cookie_domain(),
            Err(SettingsUrlError::DefguardUrlUsesIpAddress(_))
        ));
    }

    #[test]
    fn test_validate_accepts_valid_hostname() {
        let mut settings = Settings {
            defguard_url: "https://defguard.example.com".into(),
            ..Default::default()
        };

        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_invalid_url() {
        let mut settings = Settings {
            defguard_url: "not a url".into(),
            ..Default::default()
        };

        assert!(matches!(
            settings.validate(),
            Err(SettingsValidationError::InvalidDefguardUrl(_))
        ));
    }

    #[test]
    #[allow(deprecated)]
    fn test_apply_from_config_invalid_secret_key_generates_new() {
        let mut settings = Settings::default();
        let mut config = DefGuardConfig::new_test_config();
        config.secret_key = Some(SecretString::from(" short ".to_string()));

        settings.apply_from_config(&config);

        let generated = settings.secret_key.expect("secret key should be generated");
        assert_eq!(generated.len(), 64);
        assert_ne!(generated, " short ");
        assert!(Settings::validate_secret_key(&generated).is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_apply_from_config_valid_secret_key_is_used() {
        let mut settings = Settings::default();
        let mut config = DefGuardConfig::new_test_config();
        let valid_secret = "b".repeat(64);
        config.secret_key = Some(SecretString::from(valid_secret.clone()));

        settings.apply_from_config(&config);

        assert_eq!(settings.secret_key.as_deref(), Some(valid_secret.as_str()));
    }

    #[sqlx::test]
    #[allow(deprecated)]
    async fn test_update_from_config_persists_and_updates_current_settings(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool).await.unwrap();

        let mut settings = Settings::get_current_settings();
        settings.defguard_url = "https://defguard.example.com".into();
        update_current_settings(&pool, settings.clone())
            .await
            .unwrap();

        let mut config = DefGuardConfig::new_test_config();
        config.mfa_code_timeout = Some(Duration::from(std::time::Duration::from_secs(90)));
        config.session_timeout = Some(Duration::from(std::time::Duration::from_secs(
            2 * 24 * 3600,
        )));
        config.disable_stats_purge = Some(true);

        settings.update_from_config(&pool, &config).await.unwrap();

        let current = Settings::get_current_settings();
        let from_db = Settings::get(&pool).await.unwrap().unwrap();

        assert_eq!(current.mfa_code_timeout_seconds, 90);
        assert_eq!(current.authentication_period_days, 2);
        assert!(!current.enable_stats_purge);

        assert_eq!(from_db.mfa_code_timeout_seconds, 90);
        assert_eq!(from_db.authentication_period_days, 2);
        assert!(!from_db.enable_stats_purge);
    }

    #[sqlx::test]
    async fn test_initialize_runtime_defaults_keeps_valid_defguard_url(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        initialize_current_settings(&pool).await.unwrap();

        let mut settings = Settings::get_current_settings();
        settings.defguard_url = "https://defguard.example.com:8443/path".into();
        settings.secret_key = Some("a".repeat(64));
        update_current_settings(&pool, settings).await.unwrap();

        Settings::initialize_runtime_defaults(&pool).await.unwrap();

        let current = Settings::get_current_settings();
        let from_db = Settings::get(&pool).await.unwrap().unwrap();

        assert_eq!(current.webauthn_rp_id().unwrap(), "defguard.example.com");
        assert_eq!(from_db.webauthn_rp_id().unwrap(), "defguard.example.com");
    }

    #[test]
    fn test_edge_callback_url() {
        let mut s = Settings {
            public_proxy_url: "https://edge.example.com".into(),
            ..Default::default()
        };

        assert_eq!(
            s.edge_callback_url(AuthFlowType::Enrollment)
                .unwrap()
                .as_str(),
            "https://edge.example.com/openid/callback"
        );
        assert_eq!(
            s.edge_callback_url(AuthFlowType::Mfa).unwrap().as_str(),
            "https://edge.example.com/openid/mfa/callback"
        );

        s.public_proxy_url = "https://edge.example.com:8443/path".into();
        assert_eq!(
            s.edge_callback_url(AuthFlowType::Enrollment)
                .unwrap()
                .as_str(),
            "https://edge.example.com:8443/path/openid/callback"
        );
        assert_eq!(
            s.edge_callback_url(AuthFlowType::Mfa).unwrap().as_str(),
            "https://edge.example.com:8443/path/openid/mfa/callback"
        );
    }
}
