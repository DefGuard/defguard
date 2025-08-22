use std::collections::VecDeque;

use axum::http::Uri;
use defguard_core::grpc::proto::gateway::{
    Configuration, ConfigurationRequest, StatsUpdate, Update,
    gateway_service_client::GatewayServiceClient,
};
use hyper_util::rt::TokioIo;
use tokio::{
    io::DuplexStream,
    sync::mpsc::{UnboundedSender, unbounded_channel},
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    Request, Response, Status, Streaming,
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

    fn add_request_metadata<T>(&self, request: &mut Request<T>) {
        // add authorization token
        if let Some(token) = &self.auth_token {
            request.metadata_mut().insert(
                "authorization",
                MetadataValue::try_from(token).expect("failed to convert token into metadata"),
            );
        };

        // add gateway hostname
        if let Some(hostname) = &self.hostname {
            request.metadata_mut().insert(
                "hostname",
                MetadataValue::try_from(hostname)
                    .expect("failed to convert hostname into metadata"),
            );
        };
    }

    pub(crate) async fn get_gateway_config(&mut self) -> Result<Response<Configuration>, Status> {
        let mut request = Request::new(ConfigurationRequest {
            name: self.hostname.clone(),
        });

        self.add_request_metadata(&mut request);

        self.client.config(request).await
    }

    #[must_use]
    pub(crate) async fn connect_to_updates_stream(&mut self) -> Streaming<Update> {
        let mut request = Request::new(());

        self.add_request_metadata(&mut request);

        self.client.updates(request).await.unwrap().into_inner()
    }

    #[must_use]
    pub(crate) async fn setup_stats_update_stream(&mut self) -> UnboundedSender<StatsUpdate> {
        let (tx, rx) = unbounded_channel();

        self.client
            .stats(UnboundedReceiverStream::new(rx))
            .await
            .unwrap();

        tx
    }

    pub(crate) fn set_token(&mut self, token: &str) {
        self.auth_token = Some(token.into())
    }

    pub(crate) fn clear_token(&mut self) {
        self.auth_token = None;
    }

    pub(crate) fn set_hostname(&mut self, hostname: &str) {
        self.hostname = Some(hostname.into())
    }

    pub(crate) fn clear_hostname(&mut self) {
        self.hostname = None;
    }

    pub(crate) fn hostname(&self) -> String {
        self.hostname.clone().unwrap_or_default()
    }
}
