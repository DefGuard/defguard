use serde::{Deserialize, Serialize};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tracing::info;
use utoipa::ToSchema;

pub mod models;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema, Eq, Default, Hash)]
pub struct NoId;
pub type Id = i64;

// helper for easier migration handling with a custom `migration` folder location
// reference: https://docs.rs/sqlx/latest/sqlx/attr.test.html#automatic-migrations-requires-migrate-feature
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

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
    MIGRATOR
        .run(&pool)
        .await
        .expect("Cannot run database migrations.");
    pool
}

// Helper function to instantiate pool manually as a workaround for issues with `sqlx::test` macro
// reference: https://github.com/launchbadge/sqlx/issues/2567#issuecomment-2009849261
pub async fn setup_pool(options: PgConnectOptions) -> PgPool {
    let pool = PgPoolOptions::new().connect_with(options).await.unwrap();
    MIGRATOR
        .run(&pool)
        .await
        .expect("Cannot run database migrations.");
    pool
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TriggerOperation {
    Insert,
    Update,
    Delete,
}

#[derive(Deserialize)]
pub struct ChangeNotification<T> {
    pub operation: TriggerOperation,
    pub old: Option<T>,
    pub new: Option<T>,
}
