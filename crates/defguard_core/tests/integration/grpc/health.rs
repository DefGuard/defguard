use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tonic_health::pb::{
    HealthCheckRequest, health_check_response::ServingStatus, health_client::HealthClient,
};

use super::common::{make_grpc_test_server, setup_grpc_pool};

#[sqlx::test]
async fn worker_service_health_is_serving(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_grpc_pool(PgPoolOptions::new(), options).await;
    let server = make_grpc_test_server(&pool).await;
    let mut client = HealthClient::new(server.client_channel.clone());

    let response = client
        .check(HealthCheckRequest {
            service: "worker.WorkerService".into(),
        })
        .await
        .expect("health check should succeed")
        .into_inner();

    assert_eq!(response.status, ServingStatus::Serving as i32);
}
