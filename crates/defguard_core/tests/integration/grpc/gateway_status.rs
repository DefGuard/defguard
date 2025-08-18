use defguard_core::db::setup_pool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::grpc::common::make_grpc_test_client;

#[sqlx::test]
async fn test_gateway_connect(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let test_client = make_grpc_test_client(pool).await;
    todo!()
}
