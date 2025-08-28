use std::time::Duration;

use defguard_core::{grpc::proto::gateway::{
    gateway_service_client::GatewayServiceClient, Configuration, ConfigurationRequest, StatsUpdate, Update
}, VERSION};
use defguard_version::{client::version_interceptor, Version};
use tokio::{
    sync::mpsc::{UnboundedSender, unbounded_channel},
    task::JoinHandle,
    time::timeout,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    Request, Response, Status, Streaming, metadata::MetadataValue,
    service::interceptor::InterceptedService, transport::Channel,
};

// TODO what magic spell goes here?
type InterceptorFn = todo!();

pub(crate) struct MockGateway {
    client: GatewayServiceClient<InterceptedService<Channel, InterceptorFn>>,
    auth_token: Option<String>,
    hostname: Option<String>,
    stats_update_thread_handle: Option<JoinHandle<()>>,
    updates_stream: Option<Streaming<Update>>,
}

impl Drop for MockGateway {
    fn drop(&mut self) {
        if let Some(handle) = &self.stats_update_thread_handle {
            handle.abort();
        }
    }
}

impl MockGateway {
    #[must_use]
    pub(crate) async fn new(client_channel: Channel) -> Self {
        // Initialize client with version interceptor
        let client = GatewayServiceClient::with_interceptor(
            client_channel,
            Box::new(version_interceptor(Version::parse(VERSION).unwrap())),
        );

        Self {
            client,
            auth_token: None,
            hostname: None,
            stats_update_thread_handle: None,
            updates_stream: None,
        }
    }

    // Add required authorization and hostname headers to gRPC requests
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

    // Fetch gateway config from core
    pub(crate) async fn get_gateway_config(&mut self) -> Result<Response<Configuration>, Status> {
        let mut request = Request::new(ConfigurationRequest {
            name: self.hostname.clone(),
        });

        self.add_request_metadata(&mut request);

        self.client.config(request).await
    }

    pub(crate) async fn connect_to_updates_stream(&mut self) {
        let mut request = Request::new(());

        self.add_request_metadata(&mut request);

        let updates_stream = self.client.updates(request).await.unwrap().into_inner();

        self.updates_stream = Some(updates_stream);
    }

    pub(crate) fn disconnect_from_updates_stream(&mut self) {
        self.updates_stream = None;
    }

    #[must_use]
    pub(crate) async fn receive_next_update(&mut self) -> Option<Update> {
        match &mut self.updates_stream {
            Some(stream) => match timeout(Duration::from_millis(100), stream.message()).await {
                Ok(result) => result.expect("failed to reveive update message"),
                Err(_) => None,
            },
            None => None,
        }
    }

    // Connect to interface stats update endpoint
    // and return a tx which can be used to send stats updates to test gRPC server
    #[must_use]
    pub(crate) async fn setup_stats_update_stream(&mut self) -> UnboundedSender<StatsUpdate> {
        let (tx, rx) = unbounded_channel();

        let mut request = Request::new(UnboundedReceiverStream::new(rx));

        self.add_request_metadata(&mut request);

        let mut client = self.client.clone();
        let task_handle = tokio::spawn(async move {
            client.stats(request).await.expect("stats stream closed");
        });

        self.stats_update_thread_handle = Some(task_handle);

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
