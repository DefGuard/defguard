pub mod models;

use sqlx::postgres::{PgConnectOptions, PgPool};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct NoId;
pub type Id = i64;

/// Initializes and migrates postgres database. Returns DB pool object.
pub async fn init_db(host: &str, port: u16, name: &str, user: &str, password: &str) -> PgPool {
    info!("Initializing pool of database connections");
    let opts = PgConnectOptions::new()
        .host(host)
        .port(port)
        .username(user)
        .password(password)
        .database(name);
    let pool = PgPool::connect_with(opts)
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
    oauth2authorizedapp::OAuth2AuthorizedApp,
    oauth2token::OAuth2Token,
    session::{Session, SessionState},
    user::{MFAMethod, User},
    MFAInfo, UserDetails, UserInfo,
};
