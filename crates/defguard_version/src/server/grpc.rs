use std::{
    collections::{HashMap, HashSet},
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use http::HeaderValue;
use tonic::{
    Status,
    body::Body,
    codegen::http::{Request, Response},
    server::NamedService,
    service::Interceptor,
};
use tower::Service;
use tracing::{debug, error};

use crate::{
    ComponentInfo, DefguardComponent, IncompatibleComponentMetadata, IncompatibleComponents,
    SYSTEM_INFO_HEADER, VERSION_HEADER, Version, is_version_lower, server::DefguardVersionService,
};

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

/// Interceptor for `tonic` that validates client version information from request headers.
///
/// This interceptor extracts version headers from incoming gRPC requests and validates them
/// against configured version requirements. It can enforce both minimum version requirements and
/// optionally reject clients with versions higher than the server's own version.
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
/// * `own_version` – The server's own version, used for upper bound validation
/// * `component` – The expected client component type (e.g., Gateway, Core)
/// * `min_version` – Minimum required client version
/// * `fail_if_client_version_is_higher` – Whether to reject clients with versions higher than the
///   server
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
    incompatible_components: IncompatibleComponents,
}

impl DefguardVersionInterceptor {
    #[must_use]
    pub fn new(
        own_version: Version,
        component: DefguardComponent,
        min_version: Version,
        fail_if_client_version_is_higher: bool,
        incompatible_components: IncompatibleComponents,
    ) -> Self {
        Self {
            own_version,
            component,
            min_version,
            fail_if_client_version_is_higher,
            incompatible_components,
        }
    }

    #[must_use]
    fn is_component_version_supported(&self, version: Option<&Version>) -> bool {
        let Some(version) = version else {
            error!(
                "Missing {0} version information. This most likely means that {0} component uses \
                older, unsupported version. Minimal supported version is {1}.",
                self.component, self.min_version,
            );
            return false;
        };

        if is_version_lower(version, &self.min_version) {
            error!(
                "{0} version {version} is not supported. Minimal supported {0} version is {1}.",
                self.component, self.min_version
            );
            return false;
        }

        if self.fail_if_client_version_is_higher && is_version_lower(&self.own_version, version) {
            error!(
                "{} client version {version} is higher than server version {}.",
                self.component, self.own_version
            );
            return false;
        }

        debug!("{} version {version} is supported", self.component);
        true
    }
}

impl Interceptor for DefguardVersionInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        let maybe_info = ComponentInfo::from_metadata(request.metadata());
        let version = maybe_info.as_ref().map(|info| &info.version);
        if !self.is_component_version_supported(version) {
            let msg = match version {
                Some(version) => format!("Version {version} not supported"),
                None => "Missing version headers".to_string(),
            };
            let metadata =
                IncompatibleComponentMetadata::new(self.component.clone(), version.cloned());
            metadata.insert(&mut self.incompatible_components);
            return Err(Status::failed_precondition(msg));
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
