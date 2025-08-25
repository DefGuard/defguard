//! Server-side middleware for adding Defguard version information to gRPC responses.
//!
//! This module provides a tower-based middleware layer that automatically adds version
//! and system information headers to all gRPC responses. The middleware is designed to
//! work with tonic's interceptor system and maintains compatibility with both regular
//! and intercepted services.
//!
//! # Headers Added
//!
//! - `defguard-version`: The semantic version of the Defguard component
//! - `defguard-system`: System information including OS type, version, and architecture
//!
//! # Usage
//!
//! ```
//! use tower::ServiceBuilder;
//! use defguard_version::server::DefguardVersionLayer;
//! use semver::Version;
//!
//! let my_grpc_service = ServiceBuilder::new();
//! let version = Version::parse("1.0.0").unwrap();
//! let version_layer = DefguardVersionLayer::new(version).unwrap();
//! let service = ServiceBuilder::new()
//!     .layer(version_layer)
//!     .service(my_grpc_service);
//! ```

use http::{HeaderMap, HeaderValue};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tonic::{
    body::Body,
    codegen::http::{Request, Response},
    server::NamedService,
};
use tower::{Layer, Service};
use tracing::{error, info, warn};

use crate::{
    ComponentInfo, DefguardComponent, SYSTEM_INFO_HEADER, SystemInfo, VERSION_HEADER, Version,
};

/// A tower `Layer` that adds Defguard version and system information headers to gRPC responses.
///
/// This layer wraps any service and ensures that all responses include version metadata
/// in HTTP headers. The layer is designed to be composable with other tower layers and
/// maintains the original service's `NamedService` implementation for tonic compatibility.
///
/// # Fields
///
/// * `component_info` - Contains version and system information that will be added to response
///   headers.
#[derive(Clone)]
pub struct DefguardVersionLayer {
    component_info: ComponentInfo,
    remote_component: DefguardComponent,
    remote_component_min_version: Version,
}

impl DefguardVersionLayer {
    /// Creates a new version layer with the specified version string.
    ///
    /// # Arguments
    ///
    /// * `version` - Semantic version of the component
    ///
    /// # Returns
    ///
    /// * `Ok(DefguardVersionLayer)` - A new layer instance
    /// * `Err(DefguardVersionError)` - If the version string cannot be parsed
    pub fn new(
        version: Version,
        remote_component: DefguardComponent,
        remote_component_min_version: Version,
    ) -> Self {
        Self {
            component_info: ComponentInfo::new(version),
            remote_component,
            remote_component_min_version,
        }
    }
}

impl<S> Layer<S> for DefguardVersionLayer {
    type Service = DefguardVersionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DefguardVersionService {
            inner,
            component_info: self.component_info.clone(),
            remote_component: self.remote_component.clone(),
            remote_component_min_version: self.remote_component_min_version.clone(),
        }
    }
}

/// A tower `Service` that wraps another service and adds version headers to responses.
///
/// This service is created by the `DefguardVersionLayer` and implements the actual
/// header injection logic. It maintains full compatibility with the wrapped service's
/// interface while adding the version metadata functionality.
///
/// # Type Parameters
///
/// * `S` - The inner service type being wrapped
///
/// # Fields
///
/// * `inner` - The wrapped service that handles the actual request processing
/// * `component_info` - Version and system information to be added to response headers
#[derive(Clone)]
pub struct DefguardVersionService<S> {
    inner: S,
    component_info: ComponentInfo,
    remote_component: DefguardComponent,
    remote_component_min_version: Version,
}

impl<S> DefguardVersionService<S> {
    /// Checks if remote component version meets minimum version requirements.
    fn is_version_supported(&self, version: Option<&Version>) -> bool {
        let Some(version) = version else {
            error!(
                "Missing core component version information. This most likely means that core component uses unsupported version."
            );
            return false;
        };
        if version < &self.remote_component_min_version {
            error!(
                "{} version {version} is not supported. Minimal supported {} version is {}.",
                self.remote_component, self.remote_component, self.remote_component_min_version
            );
            return false;
        }

        info!("{} version {version} is supported", self.remote_component);
        true
    }

    pub fn parse_headers(metadata: &HeaderMap) -> Option<ComponentInfo> {
        let Some(version) = metadata.get(VERSION_HEADER) else {
            warn!("Missing version header");
            return None;
        };
        let Some(system) = metadata.get(SYSTEM_INFO_HEADER) else {
            warn!("Missing system info header");
            return None;
        };
        let (Ok(version), Ok(system)) = (version.to_str(), system.to_str()) else {
            warn!("Failed to stringify version or system info header value");
            return None;
        };
        let Ok(version) = Version::parse(version) else {
            warn!("Failed to parse version: {version}");
            return None;
        };
        let Ok(system) = SystemInfo::try_from_header_value(system) else {
            warn!("Failed to parse system info: {system}");
            return None;
        };

        Some(ComponentInfo { version, system })
    }
}

impl<S, B> Service<Request<Body>> for DefguardVersionService<S>
where
    S: Service<Request<Body>, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = Response<B>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Delegate readiness polling to the inner service
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
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

        // Check remote component version
        let maybe_info = Self::parse_headers(request.headers());
        let version = maybe_info.as_ref().map(|info| &info.version);
        let is_remote_version_supported = self.is_version_supported(version);
        Box::pin(async move {
			if !is_remote_version_supported {
				// TODO: return error response
				error!("REMOTE VERSION NOT SUPPORTED");
			}
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

/// Implementation of `NamedService` that delegates to the inner service.
///
/// This ensures that the wrapped service maintains its original service name
/// for tonic's service discovery and routing mechanisms. The version middleware
/// is transparent from the perspective of service identification.
impl<S> NamedService for DefguardVersionService<S>
where
    S: NamedService,
{
    const NAME: &'static str = S::NAME;
}
