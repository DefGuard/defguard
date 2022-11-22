#[cfg(feature = "oauth")]
pub mod authorization_code;
#[cfg(feature = "oauth")]
pub mod oauth;
#[cfg(feature = "oauth")]
mod oauth2client;
#[cfg(feature = "openid")]
pub mod openid;

pub use oauth2client::OAuth2Client;
