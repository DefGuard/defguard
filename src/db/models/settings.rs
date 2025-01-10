use std::collections::HashMap;

use sqlx::{query, query_as, PgExecutor, PgPool, Type};
use struct_patch::Patch;
use thiserror::Error;

use crate::secret::SecretString;

#[derive(Error, Debug)]
pub enum SettingsValidationError {
    #[error("Cannot enable gateway disconnect notifications. SMTP is not configured")]
    CannotEnableGatewayNotifications,
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Type, Debug)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    None,
    StartTls,
    ImplicitTls,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Patch, Serialize)]
#[patch(attribute(derive(Deserialize, Serialize)))]
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
    pub smtp_password: Option<SecretString>,
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
    pub ldap_bind_password: Option<SecretString>,
    pub ldap_group_search_base: Option<String>,
    pub ldap_user_search_base: Option<String>,
    pub ldap_user_obj_class: Option<String>,
    pub ldap_group_obj_class: Option<String>,
    pub ldap_username_attr: Option<String>,
    pub ldap_groupname_attr: Option<String>,
    pub ldap_group_member_attr: Option<String>,
    pub ldap_member_attr: Option<String>,
    // Whether to create a new account when users try to log in with external OpenID
    pub openid_create_account: bool,
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
            "SELECT openid_enabled, wireguard_enabled, webhooks_enabled, \
            worker_enabled, challenge_template, instance_name, main_logo_url, nav_logo_url, \
            smtp_server, smtp_port, smtp_encryption \"smtp_encryption: _\", smtp_user, \
            smtp_password \"smtp_password?: SecretString\", smtp_sender, \
            enrollment_vpn_step_optional, enrollment_welcome_message, \
            enrollment_welcome_email, enrollment_welcome_email_subject, \
            enrollment_use_welcome_message_as_email, uuid, ldap_url, ldap_bind_username, \
            ldap_bind_password \"ldap_bind_password?: SecretString\", \
            ldap_group_search_base, ldap_user_search_base, ldap_user_obj_class, \
            ldap_group_obj_class, ldap_username_attr, ldap_groupname_attr, \
            ldap_group_member_attr, ldap_member_attr, openid_create_account, \
            license, \
            gateway_disconnect_notifications_enabled, gateway_disconnect_notifications_inactivity_threshold, gateway_disconnect_notifications_reconnect_notification_enabled \
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
        };

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
            openid_create_account = $32, \
            license = $33, \
            gateway_disconnect_notifications_enabled = $34, \
            gateway_disconnect_notifications_inactivity_threshold = $35, \
            gateway_disconnect_notifications_reconnect_notification_enabled = $36 \
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
            &self.smtp_password as &Option<SecretString>,
            self.smtp_sender,
            self.enrollment_vpn_step_optional,
            self.enrollment_welcome_message,
            self.enrollment_welcome_email,
            self.enrollment_welcome_email_subject,
            self.enrollment_use_welcome_message_as_email,
            self.uuid,
            self.ldap_url,
            self.ldap_bind_username,
            &self.ldap_bind_password as &Option<SecretString>,
            self.ldap_group_search_base,
            self.ldap_user_search_base,
            self.ldap_user_obj_class,
            self.ldap_group_obj_class,
            self.ldap_username_attr,
            self.ldap_groupname_attr,
            self.ldap_group_member_attr,
            self.ldap_member_attr,
            self.openid_create_account,
            self.license,
            self.gateway_disconnect_notifications_enabled,
            self.gateway_disconnect_notifications_inactivity_threshold,
            self.gateway_disconnect_notifications_reconnect_notification_enabled
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub(crate) async fn save_license<'e, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE \"settings\" SET license = $1 WHERE id = 1",
            self.license,
        )
        .execute(executor)
        .await?;

        Ok(())
    }

    pub async fn get_settings<'e, E>(executor: E) -> Result<Self, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let settings = Settings::get(executor).await?;

        Ok(settings.expect("Settings not found"))
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
    ///
    /// Meant to be used to check if sending emails is enabled in current instance.
    #[must_use]
    pub fn smtp_configured(&self) -> bool {
        self.smtp_server.is_some()
            && self.smtp_port.is_some()
            && self.smtp_user.is_some()
            && self.smtp_password.is_some()
            && self.smtp_sender.is_some()
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

By completing the enrollment process, you now have now access to all company systems.

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
