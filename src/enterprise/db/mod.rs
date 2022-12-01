#[cfg(feature = "openid")]
mod auth_code;
#[cfg(feature = "openid")]
mod oauth2client;
#[cfg(feature = "openid")]
mod oauth2token;

#[cfg(feature = "openid")]
pub use {auth_code::AuthCode, oauth2client::OAuth2Client, oauth2token::OAuth2Token};

#[cfg(feature = "openid")]
#[derive(Deserialize, Serialize)]
pub struct NewOpenIDClient {
    pub name: String,
    pub redirect_uri: Vec<String>,
    pub scope: Vec<String>,
    pub enabled: bool,
}
