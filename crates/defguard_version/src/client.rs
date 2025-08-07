use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use http::{Request, Response};
use tonic::body::BoxBody;
use tower::{Layer, Service};
use tracing::{debug, error};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Layer for adding version information to outgoing gRPC requests (client-side)
#[derive(Clone)]
pub struct DefguardVersionClientLayer {
    version: String,
}

impl DefguardVersionClientLayer {
    pub fn new(version: String) -> Self {
        Self { version }
    }
}

impl<S> Layer<S> for DefguardVersionClientLayer {
    type Service = DefguardVersionClientService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DefguardVersionClientService {
            inner,
            version: self.version.clone(),
        }
    }
}

/// Service that adds version metadata to outgoing requests and reads version info from responses
#[derive(Clone)]
pub struct DefguardVersionClientService<S> {
    inner: S,
    version: String,
}

impl<S> Service<Request<BoxBody>> for DefguardVersionClientService<S>
where
    S: Service<Request<BoxBody>, Response = Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = Response<BoxBody>;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut request: Request<BoxBody>) -> Self::Future {
        // Add our version to the outgoing request metadata
        request.headers_mut().insert(
            "dfg-version",
            self.version
                .parse()
                .expect("Version should be valid header value"),
        );

        debug!("Client: Sending dfg-version: {}", self.version);

        // Call the inner service directly (don't clone)
        let future = self.inner.call(request);
        let version = self.version.clone();

        Box::pin(async move {
            // Make the request
            let response = future.await.map_err(Into::into)?;

            // Read server version from response metadata
            let server_version = response
                .headers()
                .get("dfg-version")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown");

            error!("Client: Received server dfg-version: {}", server_version);

            Ok(response)
        })
    }
}

/// Convenience function to create a version layer for clients
pub fn version_layer(version: String) -> DefguardVersionClientLayer {
    DefguardVersionClientLayer::new(version)
}
