#[cfg(feature = "oauth")]
mod auth_code;
#[cfg(feature = "oauth")]
mod oauth2client;
#[cfg(feature = "oauth")]
mod oauth2token;

#[cfg(feature = "oauth")]
pub use {auth_code::AuthCode, oauth2client::OAuth2Client, oauth2token::OAuth2Token};

#[cfg(feature = "openid")]
#[derive(Deserialize, Serialize)]
pub struct NewOpenIDClient {
    pub name: String,
    pub redirect_uri: String,
    pub scope: Vec<String>,
    pub enabled: bool,
}
