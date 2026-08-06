use std::fmt;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgExecutor, Type, query};
use struct_patch::Patch;
use utoipa::ToSchema;

use super::deserialize_optional_field;
use crate::secret::SecretStringWrapper;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, ToSchema, Type)]
#[sqlx(type_name = "smtp_authentication", rename_all = "lowercase")]
pub enum SmtpAuthentication {
    #[default]
    None,
    Login,
    XOAuth2,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, ToSchema, Type)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    #[default]
    None,
    StartTls,
    ImplicitTls,
}

#[derive(Clone, Default, Deserialize, FromRow, PartialEq, Patch, Serialize, ToSchema)]
#[patch(attribute(derive(Deserialize, Serialize)))]
pub struct SmtpSettings {
    #[serde(rename = "smtp_server")]
    #[sqlx(rename = "smtp_server")]
    #[patch(attribute(serde(rename = "smtp_server")))]
    pub server: Option<String>,
    #[serde(rename = "smtp_port")]
    #[sqlx(rename = "smtp_port")]
    #[patch(attribute(serde(rename = "smtp_port")))]
    pub port: Option<i32>,
    #[serde(rename = "smtp_encryption")]
    #[sqlx(rename = "smtp_encryption")]
    #[patch(attribute(serde(rename = "smtp_encryption")))]
    pub encryption: SmtpEncryption,
    #[serde(rename = "smtp_user")]
    #[sqlx(rename = "smtp_user")]
    #[patch(attribute(serde(
        rename = "smtp_user",
        deserialize_with = "deserialize_optional_field",
        default
    )))]
    pub user: Option<String>,
    #[serde(rename = "smtp_password")]
    #[sqlx(rename = "smtp_password")]
    #[patch(attribute(serde(
        rename = "smtp_password",
        deserialize_with = "deserialize_optional_field",
        default
    )))]
    #[schema(value_type = Option<String>)]
    pub password: Option<SecretStringWrapper>,
    #[serde(rename = "smtp_sender")]
    #[sqlx(rename = "smtp_sender")]
    #[patch(attribute(serde(rename = "smtp_sender")))]
    pub sender: Option<String>,

    // For XOAUTH2 authentication.
    #[serde(rename = "smtp_authentication")]
    #[sqlx(rename = "smtp_authentication")]
    #[patch(attribute(serde(rename = "smtp_authentication")))]
    pub authentication: SmtpAuthentication,
    #[serde(rename = "smtp_oauth_issuer_url")]
    #[sqlx(rename = "smtp_oauth_issuer_url")]
    #[patch(attribute(serde(rename = "smtp_oauth_issuer_url")))]
    pub oauth_issuer_url: Option<String>,
    #[serde(rename = "smtp_oauth_client_id")]
    #[sqlx(rename = "smtp_oauth_client_id")]
    #[patch(attribute(serde(rename = "smtp_oauth_client_id")))]
    pub oauth_client_id: Option<String>,
    #[serde(rename = "smtp_oauth_client_secret")]
    #[sqlx(rename = "smtp_oauth_client_secret")]
    #[patch(attribute(serde(rename = "smtp_oauth_client_secret")))]
    #[schema(value_type = Option<String>)]
    pub oauth_client_secret: Option<SecretStringWrapper>,
    #[serde(rename = "smtp_oauth_refresh_token")]
    #[sqlx(rename = "smtp_oauth_refresh_token")]
    #[patch(attribute(serde(rename = "smtp_oauth_refresh_token")))]
    pub oauth_refresh_token: Option<String>,
    #[serde(rename = "smtp_oauth_tenant_id")]
    #[sqlx(rename = "smtp_oauth_tenant_id")]
    #[patch(attribute(serde(rename = "smtp_oauth_tenant_id")))]
    pub oauth_tenant_id: Option<String>,
    #[serde(rename = "smtp_tls_verify_cert")]
    #[sqlx(rename = "smtp_tls_verify_cert")]
    #[patch(attribute(serde(rename = "smtp_tls_verify_cert")))]
    pub tls_verify_cert: bool,
}

impl SmtpSettings {
    /// Setter for `oauth_refresh_token`.
    pub async fn set_oauth_refresh_token<'e, E>(
        &mut self,
        executor: E,
        refresh_token: String,
    ) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE settings SET smtp_oauth_refresh_token = $1",
            refresh_token
        )
        .execute(executor)
        .await?;

        self.oauth_refresh_token = Some(refresh_token);

        Ok(())
    }

    /// Check if all required options are properly configured.
    /// This is meant to be used to check if sending emails is enabled in current instance.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        let string_not_empty = |string: &String| !string.is_empty();
        let secret_not_empty = |secret: &SecretStringWrapper| !secret.expose_secret().is_empty();

        self.port.is_some()
            && self.server.as_ref().is_some_and(string_not_empty)
            && self.sender.as_ref().is_some_and(string_not_empty)
            && match self.authentication {
                SmtpAuthentication::None => true,
                SmtpAuthentication::Login => {
                    self.user.as_ref().is_some_and(string_not_empty)
                        && self.password.as_ref().is_some_and(secret_not_empty)
                }
                SmtpAuthentication::XOAuth2 => {
                    self.oauth_issuer_url.as_ref().is_some_and(string_not_empty)
                        && self.oauth_client_id.as_ref().is_some_and(string_not_empty)
                        && self
                            .oauth_client_secret
                            .as_ref()
                            .is_some_and(secret_not_empty)
                }
            }
    }

    /// Returns `true` is SMTP authentication is using XOAUTH2.
    #[must_use]
    pub fn is_xoauth2(&self) -> bool {
        matches!(self.authentication, SmtpAuthentication::XOAuth2)
    }
}

// Implement manually to avoid exposing secrets.
impl fmt::Debug for SmtpSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmtpSettings")
            .field("server", &self.server)
            .field("port", &self.port)
            .field("encryption", &self.encryption)
            .field("user", &self.user)
            .field("sender", &self.sender)
            .field("authentication", &self.authentication)
            .field("oauth_issuer_url", &self.oauth_issuer_url)
            .field("oauth_client_id", &self.oauth_client_id)
            .field("oauth_tenant_id", &self.oauth_tenant_id)
            .finish_non_exhaustive()
    }
}
