use http::{Request, Response};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll},
};
use tonic::body::BoxBody;
use tower::{Layer, Service};
use tracing::{error, warn};

use crate::{ComponentInfo, SYSTEM_INFO_HEADER, SemanticVersion, SystemInfo, VERSION_HEADER};

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
        // Add our version to the outgoing request metadata
        request.headers_mut().insert(
            VERSION_HEADER,
            self.own_info
                .version
                .to_string()
                .parse()
                // TODO
                .expect("Version should be valid header value"),
        );

        // TODO add system info header

        // Call the inner service directly (don't clone)
        let future = self.inner.call(request);

        let remote_info = Arc::clone(&self.remote_info);
        let own_info = self.own_info.clone();
        Box::pin(async move {
            // Make the request
            let response = future.await.map_err(Into::into)?;

            let server_version = response.headers().get(VERSION_HEADER);
            let server_info = response.headers().get(SYSTEM_INFO_HEADER);

            if let (Some(server_version), _) = (server_version, server_info) {
                if let Ok(version) = server_version.to_str() {
                    if let Ok(version) = SemanticVersion::try_from(version) {
                        error!("OWN VERSION: {}", own_info.version.to_string());
                        error!("SERVER VERSION: {}", version.to_string());
                        // TODO
                        let system = SystemInfo {
                            os_type: "?".to_string(),
                            os_version: "?".to_string(),
                            os_edition: "?".to_string(),
                            os_codename: "?".to_string(),
                            bitness: "?".to_string(),
                            architecture: "?".to_string(),
                        };
                        *remote_info.write().unwrap() = Some(ComponentInfo { version, system });
                    }
                }
            } else {
                warn!("Missing version and/or system info header");
            }

            // // Read server version from response metadata
            // let server_version = response
            //     .headers()
            //     .get(VERSION_HEADER);
            //     // .and_then(|v| v.to_str().ok())
            //     // .unwrap_or("unknown");

            // error!("Client: Received server dfg-version: {}", server_version);

            Ok(response)
        })
    }
}
