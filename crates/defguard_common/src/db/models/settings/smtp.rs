use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use struct_patch::Patch;

use super::deserialize_optional_field;
use crate::secret::SecretStringWrapper;

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Type)]
#[sqlx(type_name = "smtp_encryption", rename_all = "lowercase")]
pub enum SmtpEncryption {
    #[default]
    None,
    StartTls,
    ImplicitTls,
    XOAuth2,
}

#[derive(Clone, Default, Deserialize, FromRow, PartialEq, Patch, Serialize)]
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
    pub password: Option<SecretStringWrapper>,
    #[serde(rename = "smtp_sender")]
    #[sqlx(rename = "smtp_sender")]
    #[patch(attribute(serde(rename = "smtp_sender")))]
    pub sender: Option<String>,

    // For XOAUTH2 authentication.
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
    pub oauth_client_secret: Option<SecretStringWrapper>,
    #[serde(rename = "smtp_oauth_refresh_token")]
    #[sqlx(rename = "smtp_oauth_refresh_token")]
    #[patch(attribute(serde(rename = "smtp_oauth_refresh_token")))]
    pub oauth_refresh_token: Option<String>,
}
