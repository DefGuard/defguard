#[cfg(feature = "oauth")]
pub mod auth_code;
#[cfg(feature = "oauth")]
mod oauth2client;
#[cfg(feature = "oauth")]
pub mod oauth2token;
#[cfg(feature = "openid")]
pub mod openid;

pub use oauth2client::OAuth2Client;
