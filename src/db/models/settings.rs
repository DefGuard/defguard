use super::DbPool;
use crate::secret::SecretString;
use model_derive::Model;
use sqlx::{query, Type};
use std::collections::HashMap;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Type, Debug)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    None,
    StartTls,
    ImplicitTls,
}

#[derive(Debug, Clone, Model, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(skip)]
    pub id: Option<i64>,
    pub openid_enabled: bool,
    pub ldap_enabled: bool,
    pub wireguard_enabled: bool,
    pub webhooks_enabled: bool,
    pub worker_enabled: bool,
    pub challenge_template: String,
    pub instance_name: String,
    pub main_logo_url: String,
    pub nav_logo_url: String,
    pub smtp_server: Option<String>,
    pub smtp_port: Option<i32>,
    #[model(enum)]
    pub smtp_encryption: SmtpEncryption,
    pub smtp_user: Option<String>,
    #[model(secret)]
    pub smtp_password: Option<SecretString>,
    pub smtp_sender: Option<String>,
    pub enrollment_vpn_step_optional: bool,
    pub enrollment_welcome_message: Option<String>,
    pub enrollment_welcome_email: Option<String>,
    pub enrollment_welcome_email_subject: Option<String>,
    pub enrollment_use_welcome_message_as_email: bool,
    // Instance uuid needed for desktop client
    #[serde(skip)]
    pub uuid: uuid::Uuid,
}

impl Settings {
    pub(crate) async fn get_settings<'e, E>(executor: E) -> Result<Settings, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let settings = Settings::find_by_id(executor, 1).await?;

        Ok(settings.expect("Settings not found"))
    }

    // Set default values for settings if not set yet.
    // This is only relevant to a subset of settings which are nullable
    // and we want to initialize their values.
    pub async fn init_defaults(pool: &DbPool) -> Result<(), sqlx::Error> {
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
}

mod defaults {
    pub const WELCOME_MESSAGE: &str = "Dear {{ first_name }} {{ last_name }},

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

    pub const WELCOME_EMAIL_SUBJECT: &str = "[defguard] Welcome message after enrollment";
}
