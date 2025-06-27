use std::collections::HashMap;

use sqlx::{PgExecutor, PgPool, Type, query, query_as};
use struct_patch::Patch;
use thiserror::Error;

use crate::{enterprise::ldap::sync::SyncStatus, global_value, secret::SecretStringWrapper};

global_value!(SETTINGS, Option<Settings>, None, set_settings, get_settings);

/// Initializes global `SETTINGS` struct at program startup
pub async fn initialize_current_settings(pool: &PgPool) -> Result<(), sqlx::Error> {
    debug!("Initializing global settings strut");
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

/// Helper function which stores updated `Settings` in the DB and also updates the global `SETTINGS` struct
pub async fn update_current_settings<'e, E: sqlx::PgExecutor<'e>>(
    executor: E,
    new_settings: Settings,
) -> Result<(), sqlx::Error> {
    debug!("Updating current settings to: {new_settings:?}");
    new_settings.save(executor).await?;
    set_settings(Some(new_settings));
    Ok(())
}

#[derive(Error, Debug)]
pub enum SettingsValidationError {
    #[error("Cannot enable gateway disconnect notifications. SMTP is not configured")]
    CannotEnableGatewayNotifications,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Type, Debug, Default)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    #[default]
    None,
    StartTls,
    ImplicitTls,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Type, Debug, Default, Copy)]
#[sqlx(type_name = "openid_username_handling", rename_all = "snake_case")]
pub enum OpenidUsernameHandling {
    #[default]
    /// Removes all forbidden characters
    RemoveForbidden,
    /// Replaces all forbidden characters with `_`
    ReplaceForbidden,
    /// Removes the email domain, replaces all other forbidden characters with `_`
    PruneEmailDomain,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Patch, Serialize, Default)]
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
    // Instance UUID needed for desktop client
    #[serde(skip)]
    pub uuid: uuid::Uuid,
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
    pub ldap_sync_status: SyncStatus,
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
    pub openid_username_handling: OpenidUsernameHandling,
    pub use_openid_for_mfa: bool,
    pub license: Option<String>,
    // Gateway disconnect notifications
    pub gateway_disconnect_notifications_enabled: bool,
    pub gateway_disconnect_notifications_inactivity_threshold: i32,
    pub gateway_disconnect_notifications_reconnect_notification_enabled: bool,
}

impl Settings {
    pub async fn get<'e, E>(executor: E) -> Result<Option<Self>, sqlx::Error>
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
            enrollment_use_welcome_message_as_email, uuid, ldap_url, ldap_bind_username, \
            ldap_bind_password \"ldap_bind_password?: SecretStringWrapper\", \
            ldap_group_search_base, ldap_user_search_base, ldap_user_obj_class, \
            ldap_group_obj_class, ldap_username_attr, ldap_groupname_attr, \
            ldap_group_member_attr, ldap_member_attr, openid_create_account, \
            license, gateway_disconnect_notifications_enabled, ldap_use_starttls, ldap_tls_verify_cert, \
            gateway_disconnect_notifications_inactivity_threshold, \
            gateway_disconnect_notifications_reconnect_notification_enabled, \
            ldap_sync_status \"ldap_sync_status: SyncStatus\", \
            ldap_enabled, ldap_sync_enabled, ldap_is_authoritative, \
            ldap_sync_interval, ldap_user_auxiliary_obj_classes, ldap_uses_ad, \
            ldap_user_rdn_attr, ldap_sync_groups, \
            openid_username_handling \"openid_username_handling: OpenidUsernameHandling\", use_openid_for_mfa \
            FROM \"settings\" WHERE id = 1",
        )
        .fetch_optional(executor)
        .await
    }

    /// Checks if given settings are correct
    pub fn validate(&self) -> Result<(), SettingsValidationError> {
        debug!("Validating settings: {self:?}");
        // check if gateway disconnect notifications can be enabled, since it requires SMTP to be configured
        if self.gateway_disconnect_notifications_enabled && !self.smtp_configured() {
            warn!("Cannot enable gateway disconnect notifications. SMTP is not configured.");
            return Err(SettingsValidationError::CannotEnableGatewayNotifications);
        }

        Ok(())
    }

    pub async fn save<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
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
            uuid = $20, \
            ldap_url = $21, \
            ldap_bind_username = $22, \
            ldap_bind_password  = $23, \
            ldap_group_search_base = $24, \
            ldap_user_search_base = $25, \
            ldap_user_obj_class = $26, \
            ldap_group_obj_class = $27, \
            ldap_username_attr = $28, \
            ldap_groupname_attr = $29, \
            ldap_group_member_attr = $30, \
            ldap_member_attr = $31, \
            ldap_use_starttls = $32, \
            ldap_tls_verify_cert = $33, \
            openid_create_account = $34, \
            license = $35, \
            gateway_disconnect_notifications_enabled = $36, \
            gateway_disconnect_notifications_inactivity_threshold = $37, \
            gateway_disconnect_notifications_reconnect_notification_enabled = $38, \
            ldap_sync_status = $39, \
            ldap_enabled = $40, \
            ldap_sync_enabled = $41, \
            ldap_is_authoritative = $42, \
            ldap_sync_interval = $43, \
            ldap_user_auxiliary_obj_classes = $44, \
            ldap_uses_ad = $45, \
            ldap_user_rdn_attr = $46, \
            ldap_sync_groups = $47, \
            openid_username_handling = $48, \
            use_openid_for_mfa = $49 \
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
            &self.ldap_sync_status as &SyncStatus,
            self.ldap_enabled,
            self.ldap_sync_enabled,
            self.ldap_is_authoritative,
            self.ldap_sync_interval,
            &self.ldap_user_auxiliary_obj_classes as &Vec<String>,
            self.ldap_uses_ad,
            self.ldap_user_rdn_attr,
            &self.ldap_sync_groups as &Vec<String>,
            &self.openid_username_handling as &OpenidUsernameHandling,
            self.use_openid_for_mfa,
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
    pub async fn init_defaults(pool: &PgPool) -> Result<(), sqlx::Error> {
        info!("Initializing default settings");

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

    pub fn ldap_using_username_as_rdn(&self) -> bool {
        self.ldap_user_rdn_attr
            .as_deref()
            .is_none_or(|rdn| rdn.is_empty() || Some(rdn) == self.ldap_username_attr.as_deref())
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
    pub(crate) async fn get_settings_essentials<'e, E>(executor: E) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            SettingsEssentials,
            "SELECT instance_name, main_logo_url, nav_logo_url, wireguard_enabled, \
            webhooks_enabled, worker_enabled, openid_enabled \
            FROM settings WHERE id = 1"
        )
        .fetch_one(executor)
        .await
    }
}

impl From<Settings> for SettingsEssentials {
    fn from(settings: Settings) -> Self {
        SettingsEssentials {
            webhooks_enabled: settings.webhooks_enabled,
            wireguard_enabled: settings.wireguard_enabled,
            worker_enabled: settings.worker_enabled,
            openid_enabled: settings.openid_enabled,
            nav_logo_url: settings.nav_logo_url,
            instance_name: settings.instance_name,
            main_logo_url: settings.main_logo_url,
        }
    }
}

mod defaults {
    pub static WELCOME_MESSAGE: &str = "Dear {{ first_name }} {{ last_name }},

By completing the enrollment process, you now have access to all company systems.

Your login to all systems is: {{ username }}

## Company systems

Here are the most important company systems:

- defguard: {{ defguard_url }} - where you can change your password and manage your VPN devices
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
Sent by defguard {{ defguard_version }}
Star us on GitHub! https://github.com/defguard/defguard\
";

    pub static WELCOME_EMAIL_SUBJECT: &str = "[defguard] Welcome message after enrollment";
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

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
}
