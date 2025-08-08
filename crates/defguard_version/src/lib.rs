use ::tracing::{error, warn};
use http::HeaderValue;
use semver::Version;
use std::{
    fmt::Display,
    str::FromStr,
    sync::{Arc, RwLock},
};
use thiserror::Error;

pub mod client;
pub mod server;
pub mod tracing;

static VERSION_HEADER: &str = "dfg-version";
static SYSTEM_INFO_HEADER: &str = "dfg-system-info";

#[derive(Debug, Error)]
pub enum DefguardVersionError {
    #[error(transparent)]
    SemverError(#[from] semver::Error),

    #[error("Failed to parse SystemInfo header: {0}")]
    SystemInfoParseError(String),
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

#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// The operating system type (e.g., "Linux", "Windows", "macOS")
    pub os_type: String,
    /// The operating system version (e.g., "22.04", "11", "13.0")
    pub os_version: String,
    /// The system bitness (e.g., "64-bit", "32-bit")
    pub bitness: String,
    /// The system architecture (e.g., "x86_64", "aarch64", "arm")
    pub architecture: String,
}

impl Display for SystemInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.os_type, self.os_version, self.bitness, self.architecture
        )
    }
}

impl SystemInfo {
    fn as_header_value(&self) -> String {
        format!(
            "{};{};{};{}",
            self.os_type, self.os_version, self.bitness, self.architecture
        )
    }

    fn try_from_header_value(header_value: &str) -> Result<Self, DefguardVersionError> {
        let parts: Vec<&str> = header_value.split(';').collect();
        if parts.len() != 4 {
            return Err(DefguardVersionError::SystemInfoParseError(
                header_value.to_string(),
            ));
        }

        Ok(Self {
            os_type: parts[0].to_string(),
            os_version: parts[1].to_string(),
            bitness: parts[2].to_string(),
            architecture: parts[3].to_string(),
        })
    }
}

impl From<os_info::Info> for SystemInfo {
    fn from(info: os_info::Info) -> Self {
        Self {
            os_type: info.os_type().to_string(),
            os_version: info.version().to_string(),
            bitness: info.bitness().to_string(),
            architecture: info.architecture().unwrap_or_else(|| "?").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub version: Version,
    pub system: SystemInfo,
}

impl ComponentInfo {
    pub fn try_from(version: &str) -> Result<Self, DefguardVersionError> {
        let version = Version::from_str(version)?;
        let info = os_info::get();
        Ok(Self {
            version,
            system: info.into(),
        })
    }
}

pub(crate) fn parse_version_headers(
    version: Option<&HeaderValue>,
    info: Option<&HeaderValue>,
) -> Option<(Version, SystemInfo)> {
    let Some(version) = version else {
        warn!("Missing version header");
        return None;
    };
    let Some(info) = info else {
        warn!("Missing system info header");
        return None;
    };

    let (Ok(version), Ok(info)) = (version.to_str(), info.to_str()) else {
        warn!("Failed to stringify version or system info header value");
        return None;
    };

    let Ok(version) = Version::from_str(version) else {
        warn!("Failed to parse version: {version}");
        return None;
    };

    let Ok(info) = SystemInfo::try_from_header_value(info) else {
        warn!("Failed to parse system info: {info}");
        return None;
    };

    Some((version, info))
}
