use std::fmt::Display;
use thiserror::Error;
use tonic::{Status, service::Interceptor};
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

#[derive(Clone)]
pub struct DefguardVersionInterceptor {
    component_info: ComponentInfo,
}

impl DefguardVersionInterceptor {
    pub fn new(component_info: ComponentInfo) -> Self {
        Self { component_info }
    }
}

impl Interceptor for DefguardVersionInterceptor {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        // Read client version from metadata
        let client_version = req
            .metadata()
            .get("dfg-version")
            .map(|v| v.to_str().unwrap_or("unknown"))
            .unwrap_or("missing");

        let server_version = self.component_info.version.to_string();
        error!("Client version: {}", client_version);
        error!("Server version: {}", server_version);

        // Add server version to response metadata
        req.metadata_mut().insert(
            "dfg-version",
            server_version
                .parse()
                .map_err(|_| Status::internal("Failed to set server version metadata"))?,
        );

        Ok(req)
    }
}
