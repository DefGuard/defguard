use std::collections::HashMap;

use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError, PgExecutor, Type};
use struct_patch::Patch;

use super::DbPool;
use crate::secret::SecretString;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Type, Debug)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    None,
    StartTls,
    ImplicitTls,
}

#[derive(Debug, Clone, Model, Serialize, Deserialize, PartialEq, Patch)]
#[patch_derive(Serialize, Deserialize)]
pub struct Settings {
    #[serde(skip)]
    pub id: Option<i64>,
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
    #[model(enum)]
    pub smtp_encryption: SmtpEncryption,
    pub smtp_user: Option<String>,
    #[model(secret)]
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
    #[model(secret)]
    pub ldap_bind_password: Option<SecretString>,
    pub ldap_group_search_base: Option<String>,
    pub ldap_user_search_base: Option<String>,
    pub ldap_user_obj_class: Option<String>,
    pub ldap_group_obj_class: Option<String>,
    pub ldap_username_attr: Option<String>,
    pub ldap_groupname_attr: Option<String>,
    pub ldap_group_member_attr: Option<String>,
    pub ldap_member_attr: Option<String>,
}

impl Settings {
    pub async fn get_settings<'e, E>(executor: E) -> Result<Settings, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let settings = Settings::find_by_id(executor, 1).await?;

        Ok(settings.expect("Settings not found"))
    }

    // Set default values for settings if not set yet.
    // This is only relevant to a subset of settings which are nullable
    // and we want to initialize their values.
    pub async fn init_defaults(pool: &DbPool) -> Result<(), SqlxError> {
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
    pub(crate) async fn get_settings_essentials<'e, E>(executor: E) -> Result<Self, SqlxError>
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
