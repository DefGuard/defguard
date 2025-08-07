use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tonic::{Request, Response, Status};
use tower::{Layer, Service};
use tracing::error;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Clone)]
pub struct MetadataInterceptor<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for MetadataInterceptor<S>
where
    S: Service<Request<B>> + Clone + Send + 'static,
    S::Response: Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        // Clone the metadata for logging
        let metadata = request.metadata().clone();

        // Read what you need from request metadata
        let auth = metadata
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let mut inner = self.inner.clone();

        Box::pin(async move {
            println!("Request auth: {:?}", auth);

            // Call the actual service
            let response = inner.call(request).await?;

            // Can't modify response here easily because we don't know the type
            // But we've already read the request metadata
            Ok(response)
        })
    }
}

// For response modification, we need a more specific approach
#[derive(Clone)]
pub struct ResponseMetadataLayer<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for ResponseMetadataLayer<S>
where
    S: Service<Request<B>, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = Response<B>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Read request metadata if needed
            let client_version = request
                .metadata()
                .get("dfg-version")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string();

            error!("Remote version: {}", client_version);
            error!("Own version: TODO");
            for header in request.metadata().keys() {
                error!("key: {:?}", header);
            }
            // Call inner service
            let mut response = inner.call(request).await?;

            response
                .metadata_mut()
                .insert("dfg-version", "1.555.555".parse().unwrap());

            Ok(response)
        })
    }
}
