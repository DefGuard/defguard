use std::{
    fmt::Display,
    sync::{Arc, RwLock},
};
use thiserror::Error;
use tonic::{Status, service::Interceptor};
use tracing::error;

#[derive(Debug, Error)]
pub enum DefguardVersionError {
    #[error(transparent)]
    SemverError(#[from] semver::Error),
}

#[derive(Clone, Debug)]
pub struct DefguardVersionSet {
    pub own: ComponentInfo,
    pub core: Arc<RwLock<Option<ComponentInfo>>>,
    pub proxy: Arc<RwLock<Option<ComponentInfo>>>,
    pub gateway: Arc<RwLock<Option<ComponentInfo>>>,
}

impl DefguardVersionSet {
    pub fn try_from(version: &str) -> Result<Self, DefguardVersionError> {
        Ok(Self {
            own: ComponentInfo::try_from(version)?,
            core: Arc::new(RwLock::new(None)),
            proxy: Arc::new(RwLock::new(None)),
            gateway: Arc::new(RwLock::new(None)),
        })
    }
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
    pub fn try_from(version: &str) -> Result<Self, DefguardVersionError> {
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
pub enum DefguardComponent {
    Core,
    Proxy,
    Gateway,
}

#[derive(Clone)]
pub struct DefguardVersionInterceptor {
    component: DefguardComponent,
    version_set: Arc<RwLock<DefguardVersionSet>>,
}

impl DefguardVersionInterceptor {
    pub fn new(component: DefguardComponent, version_set: Arc<RwLock<DefguardVersionSet>>) -> Self {
        Self {
            component,
            version_set,
        }
    }
}

impl Interceptor for DefguardVersionInterceptor {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        // read and set client version from metadata
        let client_version = req
            .metadata()
            .get("dfg-version")
            .map(|v| v.to_str().unwrap_or("unknown"))
            .unwrap_or("missing");
        // TODO set appropriate component version
        // match self.component {
        // 	DefguardComponent::Core => self.version_set.write().unwrap().core =
        // }
		for header in req.metadata().keys() {
			error!("key: {:?}", header);
		}
        let own_version = &self.version_set.read().unwrap().own.version;
        error!("Remote version: {}", client_version);
        error!("Own version: {}", own_version);

        // add own version to response metadata
        req.metadata_mut().insert(
            "dfg-version",
            own_version
                .to_string()
                .parse()
                .map_err(|_| Status::internal("Failed to set server version metadata"))?,
        );

        Ok(req)
    }
}
