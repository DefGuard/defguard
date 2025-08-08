use http::{Request, Response};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};
use tonic::body::BoxBody;
use tower::{Layer, Service};
use tracing::{error};

use crate::{ComponentInfo, SYSTEM_INFO_HEADER, VERSION_HEADER, parse_version_headers};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Layer for adding version information to outgoing gRPC requests (client-side)
#[derive(Clone)]
pub struct DefguardVersionClientLayer {
    own_info: ComponentInfo,
    remote_info: Arc<RwLock<Option<ComponentInfo>>>,
}

impl DefguardVersionClientLayer {
    pub fn new(own_info: ComponentInfo, remote_info: Arc<RwLock<Option<ComponentInfo>>>) -> Self {
        Self {
            own_info,
            remote_info,
        }
    }
}

impl<S> Layer<S> for DefguardVersionClientLayer {
    type Service = DefguardVersionClientService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DefguardVersionClientService {
            inner,
            own_info: self.own_info.clone(),
            remote_info: Arc::clone(&self.remote_info),
        }
    }
}

/// Service that adds version metadata to outgoing requests and reads version info from responses
#[derive(Clone)]
pub struct DefguardVersionClientService<S> {
    inner: S,
    own_info: ComponentInfo,
    remote_info: Arc<RwLock<Option<ComponentInfo>>>,
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
            self.own_info
                .version
                .to_string()
                .parse()
                .expect("Failed to parse SemanticVersion as HeaderValue"),
        );
        request.headers_mut().insert(
            SYSTEM_INFO_HEADER,
            self.own_info
                .system
                .as_header_value()
                .parse()
                .expect("Failed to parse SystemInfo as HeaderValue"),
        );

        // send the request
        let response_future = self.inner.call(request);

        // handle response
        let remote_info = Arc::clone(&self.remote_info);
        let own_info = self.own_info.clone();
        Box::pin(async move {
            let response = response_future.await.map_err(Into::into)?;

            // extract version headers
            let version = response.headers().get(VERSION_HEADER);
            let info = response.headers().get(SYSTEM_INFO_HEADER);

            if let Some((version, system)) = parse_version_headers(version, info) {
                error!("OWN VERSION: {}", own_info.version);
                error!("OWN SYSTEM: {}", own_info.system);
                error!("SERVER VERSION: {}", version);
                error!("SERVER SYSTEM: {}", system);
                *remote_info.write().unwrap() = Some(ComponentInfo { version, system });
            }
            Ok(response)
        })
    }
}
