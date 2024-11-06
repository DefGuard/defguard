use std::{
    io,
    net::{IpAddr, Ipv4Addr},
};

use ipnetwork::IpNetwork;
use tokio::{
    net::UnixListener,
    sync::{broadcast, mpsc::unbounded_channel},
};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

use super::*;

pub(super) static TONIC_SOCKET: &str = "tonic.sock";

struct FakeGateway;

#[tonic::async_trait]
impl gateway_server::Gateway for FakeGateway {
    type BidiStream = UnboundedReceiverStream<Result<CoreRequest, Status>>;

    async fn bidi(
        &self,
        request: Request<Streaming<CoreResponse>>,
    ) -> Result<Response<Self::BidiStream>, Status> {
        let (_tx, rx) = mpsc::unbounded_channel();
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
async fn test_gateway(pool: PgPool) {
    let network = WireguardNetwork::new(
        "TestNet".to_string(),
        IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 1)), 24).unwrap(),
        50051,
        "0.0.0.0".to_string(),
        None,
        vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 0)), 24).unwrap()],
        false,
        0,
        0,
    )
    .save(&pool)
    .await
    .unwrap();
    let gateway = Gateway::new(network.id, "http://[::]:50051")
        .save(&pool)
        .await
        .unwrap();
    let (events_tx, _events_rx) = broadcast::channel::<ChangeEvent>(16);
    let (mail_tx, _mail_rx) = unbounded_channel::<Mail>();

    let mut gateway_handler = GatewayHandler::new(gateway, None, pool, events_tx, mail_tx).unwrap();
    let handle = tokio::spawn(async move {
        gateway_handler.handle_connection().await;
    });
    handle.abort();
}
