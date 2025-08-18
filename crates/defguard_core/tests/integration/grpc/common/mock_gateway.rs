use axum::http::Uri;
use defguard_core::grpc::proto::gateway::gateway_service_client::GatewayServiceClient;
use hyper_util::rt::TokioIo;
use tokio::io::DuplexStream;
use tonic::transport::{Channel, Endpoint};
use tower::service_fn;

pub(crate) struct MockGateway {
    client: GatewayServiceClient<Channel>,
}

impl MockGateway {
    #[must_use]
    pub(crate) async fn new(client_stream: DuplexStream) -> Self {
        // Move client to an option so we can _move_ the inner value
        // on the first attempt to connect. All other attempts will fail.
        // reference: https://github.com/hyperium/tonic/blob/master/examples/src/mock/mock.rs#L31
        let mut client = Some(client_stream);
        let channel = Endpoint::try_from("http://[::]:50051")
            .expect("Failed to create channel")
            .connect_with_connector(service_fn(move |_: Uri| {
                let client = client.take();

                async move {
                    if let Some(client) = client {
                        Ok(TokioIo::new(client))
                    } else {
                        Err(std::io::Error::other("Client already taken"))
                    }
                }
            }))
            .await
            .expect("Failed to create client channel");

        let client = GatewayServiceClient::new(channel);

        Self { client }
    }
}
