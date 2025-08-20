use defguard_core::{db::setup_pool, grpc::proto::gateway::ConfigurationRequest};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tonic::Request;

use crate::grpc::common::{TestGrpcServer, make_grpc_test_server, mock_gateway::MockGateway};

async fn setup_test_server(pool: PgPool) -> (TestGrpcServer, MockGateway) {
    let (test_server, client_stream) = make_grpc_test_server(pool).await;

    // setup mock gateway
    let gateway = MockGateway::new(client_stream).await;
    (test_server, gateway)
}

#[sqlx::test]
async fn test_gateway_authorization(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (test_server, gateway) = setup_test_server(pool).await;

    // make a request without auth token
    let request = Request::new(ConfigurationRequest {
        name: Some("test_gw".into()),
    });

    // check that response is Status::Unauthorized
    let response = todo!();
}
