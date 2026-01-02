use std::time::Duration;

use defguard_core::grpc::{AUTHORIZATION_HEADER, HOSTNAME_HEADER};
use defguard_proto::gateway::{
    Configuration, ConfigurationRequest, Update,

};
use defguard_version::{Version, client::ClientVersionInterceptor};
use tokio::{
    sync::mpsc::{UnboundedSender, unbounded_channel},
    task::JoinHandle,
    time::timeout,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    Request, Response, Status, Streaming,
    metadata::MetadataValue,
    service::{Interceptor, InterceptorLayer, interceptor::InterceptedService},
    transport::Channel,
};
use tower::ServiceBuilder;

pub(crate) struct MockGateway {
    client: GatewayServiceClient<
        InterceptedService<InterceptedService<Channel, AuthInterceptor>, ClientVersionInterceptor>,
    >,
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

#[derive(Clone)]
struct AuthInterceptor {
    auth_token: Option<String>,
    hostname: Option<String>,
}

impl AuthInterceptor {
    pub(crate) fn new(auth_token: Option<String>, hostname: Option<String>) -> Self {
        Self {
            auth_token,
            hostname,
        }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        // add authorization token
        if let Some(token) = &self.auth_token {
            request.metadata_mut().insert(
                AUTHORIZATION_HEADER,
                MetadataValue::try_from(token).expect("failed to convert token into metadata"),
            );
        };

        // add gateway hostname
        if let Some(hostname) = &self.hostname {
            request.metadata_mut().insert(
                HOSTNAME_HEADER,
                MetadataValue::try_from(hostname)
                    .expect("failed to convert hostname into metadata"),
            );
        };

        Ok(request)
    }
}

impl MockGateway {
    #[must_use]
    pub(crate) async fn new(
        client_channel: Channel,
        version: Version,
        auth_token: Option<String>,
        hostname: Option<String>,
    ) -> Self {
        let intercepted_channel = ServiceBuilder::new()
            .layer(InterceptorLayer::new(ClientVersionInterceptor::new(
                version,
            )))
            .layer(InterceptorLayer::new(AuthInterceptor::new(
                auth_token,
                hostname.clone(),
            )))
            .service(client_channel);

        let client = GatewayServiceClient::new(intercepted_channel);

        Self {
            client,
            hostname,
            stats_update_thread_handle: None,
            updates_stream: None,
        }
    }

    // Fetch gateway config from core
    pub(crate) async fn get_gateway_config(&mut self) -> Result<Response<Configuration>, Status> {
        let request = Request::new(ConfigurationRequest {
            name: self.hostname.clone(),
        });

        self.client.config(request).await
    }

    pub(crate) async fn connect_to_updates_stream(&mut self) {
        let request = Request::new(());

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

        let request = Request::new(UnboundedReceiverStream::new(rx));

        let mut client = self.client.clone();
        let task_handle = tokio::spawn(async move {
            client.stats(request).await.expect("stats stream closed");
        });

        self.stats_update_thread_handle = Some(task_handle);

        tx
    }

    pub(crate) fn hostname(&self) -> String {
        self.hostname.clone().unwrap_or_default()
    }
}
