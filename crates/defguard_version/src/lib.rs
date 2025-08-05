use http::HeaderValue;
use std::{
    fmt::Display,
    pin::Pin,
    task::{Context, Poll},
};
use thiserror::Error;
use tower::{Layer, Service};
use tracing::error;

#[derive(Debug, Error)]
pub enum DefguardVersionError {
    #[error(transparent)]
    SemverError(#[from] semver::Error),
}

#[derive(Clone, Debug)]
pub struct SemanticVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// The operating system type (e.g., "Linux", "Windows", "macOS")
    pub os_type: String,
    /// The operating system version (e.g., "22.04", "11", "13.0")
    pub os_version: String,
    /// The operating system edition (e.g., "Server", "Pro", "Home")
    pub os_edition: String,
    /// The operating system codename (e.g., "jammy", "focal")
    pub os_codename: String,
    /// The system bitness (e.g., "64-bit", "32-bit")
    pub bitness: String,
    /// The system architecture (e.g., "x86_64", "aarch64", "arm")
    pub architecture: String,
}

impl From<os_info::Info> for SystemInfo {
    fn from(info: os_info::Info) -> Self {
        Self {
            os_type: info.os_type().to_string(),
            os_version: info.version().to_string(),
            os_edition: info.edition().unwrap_or_else(|| "?").to_string(),
            os_codename: info.codename().unwrap_or_else(|| "?").to_string(),
            bitness: info.bitness().to_string(),
            architecture: info.architecture().unwrap_or_else(|| "?").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub version: SemanticVersion,
    pub system: SystemInfo,
}

impl ComponentInfo {
    /// Automatically detects the current operating system and hardware information
    /// using the `os_info` crate and combines it with the provided application version.
    ///
    /// # Arguments
    /// * `version` - The application version string
    ///
    /// # Returns
    /// A new `VersionInfo` instance with version and system information
    pub fn parse(version: &str) -> Result<Self, DefguardVersionError> {
        let info = os_info::get();
        let version = semver::Version::parse(version)?;
        Ok(Self {
            version: SemanticVersion {
                major: version.major,
                minor: version.minor,
                patch: version.patch,
            },
            system: info.into(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct DefguardVersionLayer {
    pub component_info: ComponentInfo,
}

impl<S> Layer<S> for DefguardVersionLayer {
    type Service = DefguardVersionMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        DefguardVersionMiddleware {
            inner: service,
            component_info: self.component_info.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DefguardVersionMiddleware<S> {
    inner: S,
    component_info: ComponentInfo,
}

type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl<S, ReqBody, ResBody> Service<http::Request<ReqBody>> for DefguardVersionMiddleware<S>
where
    S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request<ReqBody>) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let header_value = HeaderValue::from_str(&self.component_info.version.to_string()).unwrap();
        Box::pin(async move {
            // copy request header map for later use
            let req_header_map = &req.headers().clone();
            let version = req_header_map.get("DFG-version");
            error!(
                "DFG-version: {}",
                version
                    .map(|h| h.to_str().unwrap())
                    .unwrap_or("missing header")
            );
            let mut response = inner.call(req).await?;
            response
                .headers_mut()
                .insert("DFG-version", header_value);

            Ok(response)
        })
    }
}
