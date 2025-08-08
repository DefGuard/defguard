use http::{Request, Response};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};
use tonic::body::BoxBody;
use tower::{Layer, Service};
use tracing::error;

use crate::{parse_version_headers, ComponentInfo, DefguardVersionError, SYSTEM_INFO_HEADER, VERSION_HEADER};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Layer for adding version information to outgoing gRPC requests (client-side)
#[derive(Clone)]
pub struct DefguardVersionClientLayer {
    component_info: ComponentInfo,
}

impl DefguardVersionClientLayer {
    pub fn from_str(version: &str) -> Result<Self, DefguardVersionError> {
        Ok(Self {
            component_info: ComponentInfo::from_str(version)?,
        })
    }
}

impl<S> Layer<S> for DefguardVersionClientLayer {
    type Service = DefguardVersionClientService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DefguardVersionClientService {
            inner,
            component_info: self.component_info.clone(),
        }
    }
}

/// Service that adds version metadata to outgoing requests and reads version info from responses
#[derive(Clone)]
pub struct DefguardVersionClientService<S> {
    inner: S,
    component_info: ComponentInfo,
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
        // add version and system info headers
        request.headers_mut().insert(
            VERSION_HEADER,
            self.component_info
                .version
                .to_string()
                .parse()
                .expect("Failed to parse SemanticVersion as HeaderValue"),
        );
        request.headers_mut().insert(
            SYSTEM_INFO_HEADER,
            self.component_info
                .system
                .as_header_value()
                .parse()
                .expect("Failed to parse SystemInfo as HeaderValue"),
        );

        // send the request
        let response_future = self.inner.call(request);
        Box::pin(async move {
            let response = response_future.await.map_err(Into::into)?;
            Ok(response)
        })
    }
}
