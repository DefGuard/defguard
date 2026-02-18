use std::{
    io,
    net::{IpAddr, Ipv4Addr},
    sync::{Arc, Mutex},
};

use ipnetwork::IpNetwork;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{
    net::UnixListener,
    sync::{broadcast, mpsc::unbounded_channel},
};
use tokio_stream::wrappers::{UnboundedReceiverStream, UnixListenerStream};
use tonic::{Request, Response, Status, Streaming, transport::Server};

use defguard_common::db::{
    models::{
        gateway::Gateway,
        wireguard::{LocationMfaMode, ServiceLocationMode, WireguardNetwork},
    },
    setup_pool,
};
use defguard_mail::Mail;
use defguard_proto::gateway::{CoreRequest, CoreResponse, gateway_server};

use super::{TONIC_SOCKET, handler::GatewayHandler};
use crate::grpc::{ClientMap, GrpcEvent, gateway::events::GatewayEvent};

// TODO: move to "gateway" repo.
struct FakeGateway;

#[tonic::async_trait]
impl gateway_server::Gateway for FakeGateway {
    type BidiStream = UnboundedReceiverStream<Result<CoreRequest, Status>>;

    async fn bidi(
        &self,
        request: Request<Streaming<CoreResponse>>,
    ) -> Result<Response<Self::BidiStream>, Status> {
        let (_tx, rx) = unbounded_channel();
        let mut stream = request.into_inner();
        tokio::spawn(async move {
            loop {
                match stream.message().await {
                    Ok(Some(_response)) => (),
                    Ok(None) => (),
                    Err(_err) => (),
                }
            }
        });

        Ok(Response::new(UnboundedReceiverStream::new(rx)))
    }
}

async fn fake_gateway() -> Result<(), io::Error> {
    let gateway = FakeGateway {};

    let uds = UnixListener::bind(TONIC_SOCKET)?;
    let uds_stream = UnixListenerStream::new(uds);

    Server::builder()
        .add_service(gateway_server::GatewayServer::new(gateway))
        .serve_with_incoming(uds_stream)
        .await
        .unwrap();

    Ok(())
}

#[sqlx::test]
async fn test_gateway(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let network = WireguardNetwork::new(
        "TestNet".to_string(),
        vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 1)), 24).unwrap()],
        50051,
        "0.0.0.0".to_string(),
        None,
        vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 0)), 24).unwrap()],
        0,
        0,
        false,
        false,
        LocationMfaMode::default(),
        ServiceLocationMode::default(),
    )
    .save(&pool)
    .await
    .unwrap();
    let gateway = Gateway::new(network.id, "http://[::]:50051")
        .save(&pool)
        .await
        .unwrap();
    let client_state = Arc::new(Mutex::new(ClientMap::new()));
    let (events_tx, _events_rx) = broadcast::channel::<GatewayEvent>(16);
    let (grpc_event_tx, _grpc_event_rx) = unbounded_channel::<GrpcEvent>();

    let mut gateway_handler =
        GatewayHandler::new(gateway, None, pool, client_state, events_tx, grpc_event_tx).unwrap();
    let handle = tokio::spawn(async move {
        gateway_handler.handle_connection().await;
    });
    handle.abort();
}
