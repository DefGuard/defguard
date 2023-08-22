use crate::{db::DbPool, secret::SecretString};
// use model_derive::Model;
use sqlx::{query, Type};
use std::collections::HashMap;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Type, Debug)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    None,
    StartTls,
    ImplicitTls,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    // #[model(enum)]
    pub smtp_encryption: SmtpEncryption,
    pub smtp_user: Option<String>,
    pub smtp_password: Option<SecretString>,
    pub smtp_sender: Option<String>,
    pub enrollment_vpn_step_optional: bool,
    pub enrollment_welcome_message: Option<String>,
    pub enrollment_welcome_email: Option<String>,
    pub enrollment_welcome_email_subject: Option<String>,
    pub enrollment_use_welcome_message_as_email: bool,
}

// FIXME: implement `SecretString` handling in `Model`
impl Settings {
    pub async fn find_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Self>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query_as!(Self,
            "SELECT id \"id?\", \"openid_enabled\", \"ldap_enabled\", \"wireguard_enabled\", \"webhooks_enabled\", \
            \"worker_enabled\", \"challenge_template\", \"instance_name\", \"main_logo_url\", \"nav_logo_url\", \
            \"smtp_server\", \"smtp_port\", \"smtp_encryption\" \"smtp_encryption: _\", \"smtp_user\", \
            smtp_password \"smtp_password?: SecretString\", \
            \"smtp_sender\", \"enrollment_vpn_step_optional\", \"enrollment_welcome_message\", \"enrollment_welcome_email\", \
            \"enrollment_welcome_email_subject\", \"enrollment_use_welcome_message_as_email\" \
            FROM \"settings\" WHERE id = $1",id).fetch_optional(executor).await
    }
    pub async fn all<'e, E>(executor: E) -> Result<Vec<Self>, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query_as!(Self,
            "SELECT id \"id?\", \"openid_enabled\", \"ldap_enabled\", \"wireguard_enabled\", \"webhooks_enabled\", \
            \"worker_enabled\", \"challenge_template\", \"instance_name\", \"main_logo_url\", \"nav_logo_url\", \
            \"smtp_server\", \"smtp_port\", \"smtp_encryption\" \"smtp_encryption: _\", \"smtp_user\", \
            smtp_password \"smtp_password?: SecretString\", \
            \"smtp_sender\", \"enrollment_vpn_step_optional\", \"enrollment_welcome_message\", \"enrollment_welcome_email\", \
            \"enrollment_welcome_email_subject\", \"enrollment_use_welcome_message_as_email\" \
            FROM \"settings\"").fetch_all(executor).await
    }
    pub async fn delete<'e, E>(self, executor: E) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        if let Some(id) = self.id {
            sqlx::query!("DELETE FROM \"settings\" WHERE id = $1", id)
                .execute(executor)
                .await?;
        }
        Ok(())
    }
    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        match self.id {
            None => {
                let id = sqlx::query_scalar!("INSERT INTO \"settings\" (\"openid_enabled\", \"ldap_enabled\", \"wireguard_enabled\", \"webhooks_enabled\", \"worker_enabled\", \"challenge_template\", \"instance_name\", \"main_logo_url\", \"nav_logo_url\", \"smtp_server\", \"smtp_port\", \"smtp_encryption\", \"smtp_user\", \
                \"smtp_password\", \
                \"smtp_sender\", \"enrollment_vpn_step_optional\", \"enrollment_welcome_message\", \"enrollment_welcome_email\", \"enrollment_welcome_email_subject\", \"enrollment_use_welcome_message_as_email\") VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20) RETURNING id",
                self.openid_enabled,self.ldap_enabled,self.wireguard_enabled,self.webhooks_enabled,self.worker_enabled,self.challenge_template,self.instance_name,self.main_logo_url,self.nav_logo_url,self.smtp_server,self.smtp_port,
                &self.smtp_encryption as &SmtpEncryption, self.smtp_user,&self.smtp_password as &Option<SecretString>,
                self.smtp_sender,self.enrollment_vpn_step_optional,self.enrollment_welcome_message,self.enrollment_welcome_email,self.enrollment_welcome_email_subject,self.enrollment_use_welcome_message_as_email,).fetch_one(executor).await? ;
                self.id = Some(id);
            }
            Some(id) => {
                sqlx::query!("UPDATE \"settings\" SET \"openid_enabled\" = $2, \"ldap_enabled\" = $3, \"wireguard_enabled\" = $4, \"webhooks_enabled\" = $5, \"worker_enabled\" = $6, \"challenge_template\" = $7, \"instance_name\" = $8, \"main_logo_url\" = $9, \"nav_logo_url\" = $10, \"smtp_server\" = $11, \"smtp_port\" = $12, \"smtp_encryption\" = $13, \"smtp_user\" = $14, \
                \"smtp_password\" = $15, \
                \"smtp_sender\" = $16, \"enrollment_vpn_step_optional\" = $17, \"enrollment_welcome_message\" = $18, \"enrollment_welcome_email\" = $19, \"enrollment_welcome_email_subject\" = $20, \"enrollment_use_welcome_message_as_email\" = $21 WHERE id = $1",
                id,self.openid_enabled,self.ldap_enabled,self.wireguard_enabled,self.webhooks_enabled,self.worker_enabled,self.challenge_template,self.instance_name,self.main_logo_url,self.nav_logo_url,self.smtp_server,self.smtp_port,
                &self.smtp_encryption as &SmtpEncryption,self.smtp_user,&self.smtp_password as &Option<SecretString>,
                self.smtp_sender,self.enrollment_vpn_step_optional,self.enrollment_welcome_message,self.enrollment_welcome_email,self.enrollment_welcome_email_subject,self.enrollment_use_welcome_message_as_email,).execute(executor).await? ;
            }
        }
        Ok(())
    }
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
