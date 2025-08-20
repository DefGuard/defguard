use axum::http::Uri;
use defguard_core::grpc::proto::gateway::{
    ConfigurationRequest, gateway_service_client::GatewayServiceClient,
};
use hyper_util::rt::TokioIo;
use tokio::io::DuplexStream;
use tonic::{
    Request, Status,
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
};
use tower::service_fn;

pub(crate) struct MockGateway {
    client: GatewayServiceClient<Channel>,
    auth_token: Option<String>,
    hostname: Option<String>,
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

        Self {
            client,
            auth_token: None,
            hostname: None,
        }
    }

    // pub(crate) async fn get_gateway_config(
    //     &self,
    // ) -> Result<defguard_core::grpc::proto::gateway::Configuration, Status> {
    //     let request = Request::new(ConfigurationRequest {
    //         name: self.hostname,
    //     });
    //     if let Some(token) = self.auth_token {
    //         request
    //             .metadata_mut()
    //             .insert("authorization", MetadataValue::try_from(token));
    //     };

    //     self.client.config(request).await
    // }
}
