pub mod models;

use sqlx::postgres::PgConnectOptions;

pub type DbPool = sqlx::postgres::PgPool;

/// Initializes and migrates postgres database. Returns DB pool object.
pub async fn init_db(host: &str, port: u16, name: &str, user: &str, password: &str) -> DbPool {
    info!("Initializing DB pool");
    let opts = PgConnectOptions::new()
        .host(host)
        .port(port)
        .username(user)
        .password(password)
        .database(name);
    let pool = DbPool::connect_with(opts)
        .await
        .expect("Database connection failed");
    sqlx::migrate!()
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
    wallet::Wallet,
    webauthn::WebAuthn,
    webhook::{AppEvent, HWKeyUserData, WebHook},
    wireguard::{GatewayEvent, WireguardNetwork, WireguardPeerStats},
    MFAInfo, UserDetails, UserInfo,
};
