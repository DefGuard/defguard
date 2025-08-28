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

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use http::HeaderValue;
use tonic::{
    body::Body,
    codegen::http::{Request, Response},
    server::NamedService,
    service::Interceptor,
};
use tower::{Layer, Service};
use tracing::{debug, error};

use crate::{
    ComponentInfo, DefguardComponent, SYSTEM_INFO_HEADER, VERSION_HEADER, Version, parse_metadata,
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
    #[must_use]
    pub fn new(version: crate::Version) -> Self {
        Self {
            component_info: ComponentInfo::new(version),
        }
    }
}

impl<S> Layer<S> for DefguardVersionLayer {
    type Service = DefguardVersionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DefguardVersionService {
            inner,
            component_info: self.component_info.clone(),
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
}

impl<S, B> Service<Request<Body>> for DefguardVersionService<S>
where
    S: Service<Request<Body>, Response = Response<B>> + Clone + Send + 'static,
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

/// A tonic interceptor that validates client version information from request headers.
///
/// This interceptor extracts version headers from incoming gRPC requests and validates
/// them against configured version requirements. It can enforce both minimum version
/// requirements and optionally reject clients with versions higher than the server's
/// own version.
///
/// # Version Validation Rules
///
/// 1. **Missing Version**: If the client doesn't provide version headers, the request is rejected
/// 2. **Below Minimum**: If the client version is below `min_version`, the request is rejected
/// 3. **Too High** (optional): If `fail_if_client_version_is_higher` is true and the client
///    version exceeds `own_version`, the request is rejected
///
/// # Fields
///
/// * `own_version` - The server's own version, used for upper bound validation
/// * `component` - The expected client component type (e.g., Gateway, Core)
/// * `min_version` - Minimum required client version
/// * `fail_if_client_version_is_higher` - Whether to reject clients with versions higher than the server
#[derive(Clone)]
pub struct DefguardVersionInterceptor {
    own_version: Version,
    component: DefguardComponent,
    min_version: Version,
    /// When true, reject clients with versions higher than the server's own version.
    /// This is used as a workaround for version compatibility checking in gateway->core
    /// communication, where the core UI needs to display version compatibility errors
    /// that would normally only be detectable on the gateway side (core < gateway).
    fail_if_client_version_is_higher: bool,
}

impl DefguardVersionInterceptor {
    #[must_use]
    pub fn new(
        own_version: Version,
        component: DefguardComponent,
        min_version: Version,
        fail_if_client_version_is_higher: bool,
    ) -> Self {
        Self {
            own_version,
            component,
            min_version,
            fail_if_client_version_is_higher,
        }
    }

    pub fn is_component_version_supported(&self, version: Option<&Version>) -> bool {
        let Some(version) = version else {
            error!(
                "Missing {} version information. This most likely means that {} component uses older, unsupported version. Minimal supported version is {}.",
                self.component, self.component, self.min_version,
            );
            return false;
        };
        if version < &self.min_version {
            error!(
                "{} version {version} is not supported. Minimal supported {} version is {}.",
                self.component, self.component, self.min_version
            );
            return false;
        }

        if self.fail_if_client_version_is_higher && version > &self.own_version {
            error!("{} version {version} is too high.", self.component);
            return false;
        }

        debug!("{} version {version} is supported", self.component);
        true
    }
}

impl Interceptor for DefguardVersionInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let maybe_info = parse_metadata(request.metadata());
        let version = maybe_info.as_ref().map(|info| &info.version);
        if !self.is_component_version_supported(version) {
            let msg = match version {
                Some(version) => format!("Version {version} not supported"),
                None => "Missing version headers".to_string(),
            };
            return Err(tonic::Status::failed_precondition(msg));
        }

        Ok(request)
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
