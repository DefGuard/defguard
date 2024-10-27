pub mod models;

use sqlx::postgres::{PgConnectOptions, PgPool};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct NoId;
pub type Id = i64;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub(crate) enum TriggerOperation {
    Insert,
    Update,
    Delete,
}

#[derive(Deserialize)]
pub(crate) struct ChangeNotification<T> {
    pub operation: TriggerOperation,
    pub old: Option<T>,
    pub new: Option<T>,
}

/// Initializes PostgreSQL database and runs the migrations.
/// Returns database pool object.
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
    session::{Session, SessionState},
    user::{MFAMethod, User},
    MFAInfo, UserDetails, UserInfo,
};
