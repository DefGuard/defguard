pub mod models;

#[cfg(test)]
use sqlx::postgres::PgPoolOptions;
use sqlx::postgres::{PgConnectOptions, PgPool};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema, Eq, Default, Hash)]
pub struct NoId;
pub type Id = i64;

/// Initializes and migrates postgres database. Returns DB pool object.
pub async fn init_db(host: &str, port: u16, name: &str, user: &str, password: &str) -> PgPool {
    info!("Initializing DB pool");
    let opts = PgConnectOptions::new()
        .host(host)
        .port(port)
        .username(user)
        .password(password)
        .database(name);
    let pool = PgPool::connect_with(opts)
        .await
        .expect("Database connection failed");
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("Cannot run database migrations.");
    pool
}

pub use models::{
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
    MFAInfo, UserDetails, UserInfo,
};

#[cfg(test)]
// Helper function to instantiate pool manually as a workaround for issues with `sqlx::test` macro
// reference: https://github.com/launchbadge/sqlx/issues/2567#issuecomment-2009849261
pub async fn setup_pool(options: PgConnectOptions) -> PgPool {
    PgPoolOptions::new().connect_with(options).await.unwrap()
}
