//! Defguard version information handling for gRPC communications.
//!
//! This crate provides utilities for embedding and extracting version and system information
//! in gRPC communications between Defguard components. It supports both client-side and
//! server-side middleware for automatic version header management.
//!
//! # Headers
//!
//! The crate defines two standard headers used across all Defguard gRPC communications:
//!
//! - `defguard-version`: Semantic version string (e.g., "1.2.3")
//! - `defguard-system`: Semicolon-separated system information (OS;version;arch)
//!
//! # Usage
//!
//! ## Server-side middleware
//!
//! ```
//! use semver::Version;
//! use tower::ServiceBuilder;
//! use defguard_version::server::DefguardVersionLayer;
//!
//! let version = Version::parse("1.0.0").unwrap();
//! let layer = DefguardVersionLayer::new(version);
//! let service = ServiceBuilder::new()
//!     .layer(layer)
//!     .service(my_grpc_service);
//! ```
//!
//! ## Client-side interceptor
//!
//! ```ignore
//! use semver::Version;
//! use defguard_version::client::version_interceptor;
//! use tonic::transport::Channel;
//!
//! let version = Version::parse("1.0.0").unwrap();
//! let channel = Channel::from_static("http://localhost:50051").connect().await.unwrap();
//! let client = MyServiceClient::with_interceptor(
//!     channel,
//!     version_interceptor(version)
//! );
//! ```
//!
//! ## Parsing version information
//!
//! ```
//! use defguard_version::{parse_metadata, version_info_from_metadata};
//! use tonic::metadata::MetadataMap;
//!
//! let metadata = MetadataMap::new();
//!
//! // Extract parsed version and system info
//! if let Some(component_info) = parse_metadata(&metadata) {
//!     println!("Client version: {}", component_info.version);
//!     println!("Client system: {}", component_info.system);
//! }
//!
//! // Get version info as strings (with fallback)
//! let (version_str, system_str) = version_info_from_metadata(&metadata);
//! ```

use std::{cmp::Ordering, fmt, str::FromStr};

use ::tracing::{error, warn};
pub use semver::{BuildMetadata, Error as SemverError, Version};
use thiserror::Error;
use tonic::metadata::MetadataMap;

pub mod client;
pub mod server;
pub mod tracing;

/// HTTP header name for the Defguard component version.
pub static VERSION_HEADER: &str = "defguard-version";

/// HTTP header name for the Defguard system information.
pub static SYSTEM_INFO_HEADER: &str = "defguard-system";

#[derive(Debug, Error)]
pub enum DefguardVersionError {
    #[error(transparent)]
    SemverError(#[from] semver::Error),

    #[error("Failed to parse SystemInfo header: {0}")]
    SystemInfoParseError(String),

    #[error("Invalid DefguardComponent: {0}")]
    InvalidDefguardComponent(String),
}

/// Represents the different types of Defguard components that can communicate via gRPC.
#[derive(Debug, Clone)]
pub enum DefguardComponent {
    Core,
    Proxy,
    Gateway,
}

impl FromStr for DefguardComponent {
    type Err = DefguardVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "core" => Ok(DefguardComponent::Core),
            "proxy" => Ok(DefguardComponent::Proxy),
            "gateway" => Ok(DefguardComponent::Gateway),
            _ => Err(DefguardVersionError::InvalidDefguardComponent(
                s.to_string(),
            )),
        }
    }
}

impl fmt::Display for DefguardComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefguardComponent::Core => write!(f, "core"),
            DefguardComponent::Proxy => write!(f, "proxy"),
            DefguardComponent::Gateway => write!(f, "gateway"),
        }
    }
}

/// System information about the host running a Defguard component.
///
/// This struct captures key system characteristics that are useful for
/// debugging, compatibility checking, and system analytics. The information
/// is automatically detected from the host system and can be serialized
/// into HTTP headers for transmission over gRPC.
///
/// # Examples
///
/// ```
/// use defguard_version::SystemInfo;
///
/// // Get current system information
/// let info = SystemInfo::get();
/// println!("Running on: {}", info);
///
/// // Access individual fields
/// println!("OS: {} {}", info.os_type, info.os_version);
/// println!("Architecture: {}", info.architecture);
/// ```
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// The operating system type (e.g., "Linux", "Windows", "macOS")
    pub os_type: String,
    /// The operating system version (e.g., "22.04", "11", "13.0")
    pub os_version: String,
    /// The system architecture (e.g., "x86_64", "aarch64", "arm")
    pub architecture: String,
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.os_type, self.os_version, self.architecture
        )
    }
}

impl SystemInfo {
    /// Automatically detects the operating system type, version and architecture
    /// using the `os_info` crate.
    ///
    /// # Returns
    ///
    /// A `SystemInfo` struct populated with the current system's characteristics.
    #[must_use]
    pub fn get() -> Self {
        os_info::get().into()
    }

    fn as_header_value(&self) -> String {
        format!("{};{};{}", self.os_type, self.os_version, self.architecture)
    }

    fn try_from_header_value(header_value: &str) -> Result<Self, DefguardVersionError> {
        let parts: Vec<&str> = header_value.split(';').collect();
        if parts.len() != 3 {
            return Err(DefguardVersionError::SystemInfoParseError(
                header_value.to_string(),
            ));
        }

        Ok(Self {
            os_type: parts[0].to_string(),
            os_version: parts[1].to_string(),
            architecture: parts[2].to_string(),
        })
    }
}

impl From<os_info::Info> for SystemInfo {
    fn from(info: os_info::Info) -> Self {
        Self {
            os_type: info.os_type().to_string(),
            os_version: info.version().to_string(),
            architecture: info.architecture().unwrap_or("?").to_string(),
        }
    }
}

/// Combined version and system information for a Defguard component.
///
/// This struct bundles together both the semantic version of a component
/// and the system information of the host it's running on. It's used by
/// middleware to generate the appropriate headers for gRPC communication.
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    /// The semantic version of the component
    pub version: Version,
    /// System information about the host
    pub system: SystemInfo,
}

impl ComponentInfo {
    /// Creates a new ComponentInfo with the provided version and automatically detects
    /// the current system information.
    ///
    /// # Arguments
    ///
    /// * `version` - A parsed semantic version
    ///
    /// # Examples
    ///
    /// ```
    /// use defguard_version::ComponentInfo;
    /// use semver::Version;
    ///
    /// let version = Version::parse("1.0.0").unwrap();
    /// let info = ComponentInfo::new(version);
    /// assert_eq!(info.version.major, 1);
    /// ```
    #[must_use]
    pub fn new(version: Version) -> Self {
        let info = os_info::get();
        Self {
            version,
            system: info.into(),
        }
    }
}

/// Parses version and system information from gRPC metadata headers.
///
/// This function extracts and parses the Defguard version headers from
/// gRPC metadata, returning structured version and system information.
/// If any parsing step fails, warnings are logged and `None` is returned.
///
/// # Arguments
///
/// * `metadata` - The gRPC metadata map containing headers
///
/// # Returns
///
/// * `Some((Version, SystemInfo))` - Successfully parsed version information
/// * `None` - If headers are missing or parsing fails
///
/// # Examples
///
/// ```
/// use defguard_version::parse_metadata;
/// use tonic::metadata::MetadataMap;
///
/// let metadata = MetadataMap::new();
/// if let Some((version, system)) = parse_metadata(&metadata) {
///     println!("Peer version: {}", version);
///     println!("Peer system: {}", system);
/// }
/// ```
pub fn parse_metadata(metadata: &MetadataMap) -> Option<ComponentInfo> {
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
    let Ok(version) = Version::from_str(version) else {
        warn!("Failed to parse version: {version}");
        return None;
    };
    let Ok(system) = SystemInfo::try_from_header_value(system) else {
        warn!("Failed to parse system info: {system}");
        return None;
    };

    Some(ComponentInfo { version, system })
}

/// Extracts version information from metadata as formatted strings with fallback.
///
/// This is a convenience function that calls `parse_metadata` internally and
/// returns the version and system information as strings. If parsing fails,
/// it returns "?" for both values instead of `None`.
///
/// # Arguments
///
/// * `metadata` - The gRPC metadata map containing headers
///
/// # Returns
///
/// A tuple containing:
/// * Version string (or "?" if parsing failed)
/// * System info string (or "?" if parsing failed)
///
/// # Examples
///
/// ```
/// use defguard_version::version_info_from_metadata;
/// use tonic::metadata::MetadataMap;
///
/// let metadata = MetadataMap::new();
/// let (version, system) = version_info_from_metadata(&metadata);
/// println!("Client: {} running on {}", version, system);
/// // Output might be: "Client: 1.2.3 running on Linux 22.04 64-bit x86_64"
/// // Or if headers missing: "Client: ? running on ?"
/// ```
#[must_use]
pub fn version_info_from_metadata(metadata: &MetadataMap) -> (String, String) {
    parse_metadata(metadata).map_or(("?".to_string(), "?".to_string()), |info| {
        (info.version.to_string(), info.system.to_string())
    })
}

#[must_use]
pub fn get_tracing_variables(info: &Option<ComponentInfo>) -> (String, String) {
    let version = info
        .as_ref()
        .map_or(String::from("?"), |info| info.version.to_string());
    let info = info
        .as_ref()
        .map_or(String::from("?"), |info| info.system.to_string());

    (version, info)
}

/// Compares two versions while omitting build metadata (we use it for git commit hash).
/// Returns true if v1 < v2.
pub fn is_version_lower(v1: &Version, v2: &Version) -> bool {
    v1.cmp_precedence(v2) == Ordering::Less
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.5.0").unwrap();
        let v2 = Version::parse("1.6.0").unwrap();
        assert!(is_version_lower(&v1, &v2));

        let v1 = Version::parse("1.5.0-alpha1").unwrap();
        let v2 = Version::parse("1.5.0").unwrap();
        assert!(is_version_lower(&v1, &v2));

        let v1 = Version::parse("1.5.0").unwrap();
        let v2 = Version::parse("1.6.0-alpha1").unwrap();
        assert!(is_version_lower(&v1, &v2));

        let v1 = Version::parse("1.5.0-alpha1").unwrap();
        let v2 = Version::parse("1.5.0-alpha2").unwrap();
        assert!(is_version_lower(&v1, &v2));

        let v1 = Version::parse("1.5.0+1").unwrap();
        let v2 = Version::parse("1.5.0+2").unwrap();
        assert!(!is_version_lower(&v1, &v2));

        let v1 = Version::parse("1.5.0+2").unwrap();
        let v2 = Version::parse("1.5.0+1").unwrap();
        assert!(!is_version_lower(&v1, &v2));

        let v1 = Version::parse("1.5.0-alpha1+2").unwrap();
        let v2 = Version::parse("1.5.0-alpha2+1").unwrap();
        assert!(is_version_lower(&v1, &v2));
    }
}
