pub mod models;

pub use models::{
    MFAInfo, UserDetails, UserInfo,
    device::{AddDevice, Device},
    group::Group,
    oauth2authorizedapp::OAuth2AuthorizedApp,
    oauth2token::OAuth2Token,
    session::{Session, SessionState},
    settings::Settings,
    user::{MFAMethod, User},
    webauthn::WebAuthn,
    webhook::{AppEvent, HWKeyUserData, WebHook},
    wireguard::{GatewayEvent, WireguardNetwork},
    yubikey::YubiKey,
};

use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema, Eq, Default, Hash)]
pub struct NoId;
pub type Id = i64;
