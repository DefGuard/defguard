use std::{
    pin::Pin,
    task::{Context, Poll},
};

use axum::{extract::Request, http, response::Response};
use http::HeaderValue;
use tower::Service;

use crate::{SYSTEM_INFO_HEADER, VERSION_HEADER, server::DefguardVersionService};

impl<S, B> Service<Request> for DefguardVersionService<S>
where
    S: Service<Request, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;
    type Response = Response<B>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Delegate readiness polling to the inner service
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let mut inner = self.inner.clone();

        // Pre-parse header values
        let parsed_info = (
            self.component_info
                .version
                .to_string()
                .parse::<HeaderValue>()
                .ok(),
            self.component_info
                .system
                .as_header_value()
                .parse::<HeaderValue>()
                .ok(),
        );

        Box::pin(async move {
            // Process the request with the inner service first
            let mut response = inner.call(request).await?;

            // Add version headers
            if let (Some(version), Some(system)) = parsed_info {
                response.headers_mut().insert(VERSION_HEADER, version);
                response.headers_mut().insert(SYSTEM_INFO_HEADER, system);
            }

            Ok(response)
        })
    }
}
