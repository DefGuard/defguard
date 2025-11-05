use defguard_proto::proxy::DeviceInfo;
use semver::Version;

#[derive(Debug)]
pub(crate) struct ClientPlatform {
    /// The general OS family, e.g., "windows", "macos", "linux"
    os_family: String,
    /// Specific OS type, e.g., "Ubuntu", "Debian"
    /// May sometimes be the same as `os_family`, e.g., "Windows"
    #[allow(dead_code)]
    os_type: String,
    #[allow(dead_code)]
    version: String,
    #[allow(dead_code)]
    architecture: Option<String>,
    #[allow(dead_code)]
    edition: Option<String>,
    #[allow(dead_code)]
    codename: Option<String>,
    /// "32-bit", "64-bit"
    #[allow(dead_code)]
    biteness: Option<String>,
}

impl TryFrom<&str> for ClientPlatform {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.split(';').collect();
        let mut os_family = None;
        let mut os_type = None;
        let mut version = None;
        let mut architecture = None;
        let mut edition = None;
        let mut codename = None;
        let mut biteness = None;
        // The expected format is:
        // "os_family={}; os_type={}; version={}; edition={}; codename={}; bitness={}; architecture={}",
        for part in parts {
            let kv: Vec<&str> = part.trim().splitn(2, '=').collect();
            if kv.len() != 2 {
                continue;
            }
            match kv[0].trim() {
                "os_family" => {
                    let trimmed = kv[1].trim();
                    os_family = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
                "os_type" => {
                    let trimmed = kv[1].trim();
                    os_type = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
                "version" => {
                    let trimmed = kv[1].trim();
                    version = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
                "architecture" => {
                    let trimmed = kv[1].trim();
                    architecture = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
                "edition" => {
                    let trimmed = kv[1].trim();
                    edition = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
                "codename" => {
                    let trimmed = kv[1].trim();
                    codename = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
                "bitness" => {
                    let trimmed = kv[1].trim();
                    biteness = if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    };
                }
                _ => {}
            }
        }

        let (Some(os_family), Some(os_type), Some(version)) = (os_family, os_type, version) else {
            let msg = format!(
                "invalid client platform string: {value}. OS family, its concrete type and version are required."
            );
            error!(msg);
            return Err(msg);
        };

        Ok(Self {
            os_family,
            os_type,
            version,
            architecture,
            edition,
            codename,
            biteness,
        })
    }
}

pub(crate) fn parse_client_version_platform(
    info: Option<&DeviceInfo>,
) -> (Option<Version>, Option<ClientPlatform>) {
    let Some(info) = info else {
        debug!("Device information is missing from the request");
        return (None, None);
    };

    let version = info.version.as_ref().map_or_else(
        || None,
        |v| {
            Version::parse(v).map_or_else(
                |_| {
                    error!("Invalid version string: {v}");
                    None
                },
                Some,
            )
        },
    );

    let platform = info.platform.as_ref().and_then(|p| {
        ClientPlatform::try_from(p.as_str()).map_or_else(
            |_| {
                error!("Invalid platform string: {p}");
                None
            },
            Some,
        )
    });

    (version, platform)
}

/// Represents a client feature that may have minimum version and OS family requirements.
#[derive(Debug)]
pub(crate) enum ClientFeature {
    ServiceLocations,
}

impl ClientFeature {
    const fn min_version(&self) -> Option<Version> {
        match self {
            Self::ServiceLocations => Some(Version::new(1, 6, 0)),
        }
    }

    fn required_os_family(&self) -> Option<Vec<&'static str>> {
        match self {
            Self::ServiceLocations => Some(vec!["windows"]),
        }
    }

    pub(crate) fn is_supported_by_device(&self, info: Option<&DeviceInfo>) -> bool {
        let (version, platform) = parse_client_version_platform(info);

        // No minimum version = matches all
        let version_matches = self.min_version().is_none_or(|min_version| {
            // No version info = does not match
            version
                .as_ref()
                .is_some_and(|version| version >= &min_version)
        });

        if !version_matches {
            debug!(
                "Client version {:?} does not meet minimum version {:?} for feature {:?}",
                version,
                self.min_version(),
                self
            );
        }

        // No required OS family = matches all
        let platform_matches = self.required_os_family().is_none_or(|platforms| {
            platforms.iter().any(|p| {
                platform
                    .as_ref()
                    .is_some_and(|platform| platform.os_family.eq_ignore_ascii_case(p))
            })
        });

        if !platform_matches {
            debug!(
                "Client OS {:?} does not meet required OS {:?} for feature {self:?}",
                platform.as_ref().map(|p| &p.os_family),
                self.required_os_family()
            );
        }

        version_matches && platform_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create DeviceInfo
    fn create_device_info(version: Option<String>, platform: Option<String>) -> DeviceInfo {
        DeviceInfo {
            version,
            platform,
            ..Default::default()
        }
    }

    #[test]
    fn test_client_platform_try_from() {
        // Test valid full platform string
        let platform_str = "os_family=linux; os_type=Ubuntu; version=22.04; architecture=x86_64; edition=Desktop; codename=jammy; bitness=64-bit";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_ok());
        let platform = result.unwrap();
        assert_eq!(platform.os_family, "linux");
        assert_eq!(platform.os_type, "Ubuntu");
        assert_eq!(platform.version, "22.04");
        assert_eq!(platform.architecture, Some("x86_64".to_string()));
        assert_eq!(platform.edition, Some("Desktop".to_string()));
        assert_eq!(platform.codename, Some("jammy".to_string()));
        assert_eq!(platform.biteness, Some("64-bit".to_string()));

        // Test minimal valid platform string (only required fields)
        let platform_str = "os_family=windows; os_type=Windows; version=11";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_ok());
        let platform = result.unwrap();
        assert_eq!(platform.os_family, "windows");
        assert_eq!(platform.os_type, "Windows");
        assert_eq!(platform.version, "11");
        assert_eq!(platform.architecture, None);

        // Test with empty optional fields
        let platform_str = "os_family=macos; os_type=macOS; version=14.0; architecture=; edition=; codename=; bitness=";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_ok());
        let platform = result.unwrap();
        assert_eq!(platform.os_family, "macos");
        assert_eq!(platform.architecture, None);
        assert_eq!(platform.edition, None);

        // Test missing required field (os_family)
        let platform_str = "os_type=Ubuntu; version=22.04";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("OS family"));

        // Test missing required field (os_type)
        let platform_str = "os_family=linux; version=22.04";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_err());

        // Test missing required field (version)
        let platform_str = "os_family=linux; os_type=Ubuntu";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_err());

        // Test with extra whitespace
        let platform_str = "  os_family = linux ;  os_type = Ubuntu  ; version = 22.04  ";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_ok());
        let platform = result.unwrap();
        assert_eq!(platform.os_family, "linux");
        assert_eq!(platform.os_type, "Ubuntu");
        assert_eq!(platform.version, "22.04");

        // Test with unknown keys (should be ignored)
        let platform_str = "os_family=linux; os_type=Ubuntu; version=22.04; unknown_key=value";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_ok());

        // Test with malformed key-value pairs (missing equals sign)
        let platform_str = "os_family=linux; os_type=Ubuntu; version=22.04; malformed_field";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_ok());

        // Test empty string
        let platform_str = "";
        let result = ClientPlatform::try_from(platform_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_client_version_platform() {
        // Test with valid version and platform
        let info = create_device_info(
            Some("1.5.0".to_string()),
            Some("os_family=windows; os_type=Windows; version=11".to_string()),
        );
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_some());
        assert_eq!(version.unwrap(), Version::new(1, 5, 0));
        assert!(platform.is_some());
        assert_eq!(platform.unwrap().os_family, "windows");

        // Test with no DeviceInfo
        let (version, platform) = parse_client_version_platform(None);
        assert!(version.is_none());
        assert!(platform.is_none());

        // Test with invalid version string
        let info = create_device_info(
            Some("invalid.version".to_string()),
            Some("os_family=linux; os_type=Ubuntu; version=22.04".to_string()),
        );
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_none());
        assert!(platform.is_some());

        // Test with invalid platform string
        let info = create_device_info(Some("1.5.0".to_string()), Some("invalid".to_string()));
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_some());
        assert!(platform.is_none());

        // Test with missing version field
        let info = create_device_info(
            None,
            Some("os_family=linux; os_type=Ubuntu; version=22.04".to_string()),
        );
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_none());
        assert!(platform.is_some());

        // Test with missing platform field
        let info = create_device_info(Some("1.5.0".to_string()), None);
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_some());
        assert!(platform.is_none());

        // Test with both fields missing
        let info = create_device_info(None, None);
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_none());
        assert!(platform.is_none());

        // Test with pre-release version
        let info = create_device_info(
            Some("1.5.0-alpha1".to_string()),
            Some("os_family=macos; os_type=macOS; version=14.0".to_string()),
        );
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_some());
        assert_eq!(version.unwrap(), Version::parse("1.5.0-alpha1").unwrap());
        assert!(platform.is_some());
    }

    #[test]
    fn test_client_feature_is_supported_by_device() {
        // Test ServiceLocations feature with supported version and OS
        let info = create_device_info(
            Some("1.6.0".to_string()),
            Some("os_family=windows; os_type=Windows; version=11".to_string()),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported on Windows with version 1.6.0"
        );

        // Test with exact minimum version
        let info = create_device_info(
            Some("1.6.0".to_string()),
            Some("os_family=Windows; os_type=Windows; version=11".to_string()),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported at minimum version"
        );

        // Test with higher version
        let info = create_device_info(
            Some("2.0.0".to_string()),
            Some("os_family=WINDOWS; os_type=Windows; version=11".to_string()),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported with higher version"
        );

        // Test with version below minimum
        let info = create_device_info(
            Some("1.5.9".to_string()),
            Some("os_family=windows; os_type=Windows; version=11".to_string()),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported below minimum version"
        );

        // Test with wrong OS family (linux)
        let info = create_device_info(
            Some("1.6.0".to_string()),
            Some("os_family=linux; os_type=Ubuntu; version=22.04".to_string()),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported on Linux"
        );

        // Test with wrong OS family (macos)
        let info = create_device_info(
            Some("1.6.0".to_string()),
            Some("os_family=macos; os_type=macOS; version=14.0".to_string()),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported on macOS"
        );

        // Test with no DeviceInfo
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(None),
            "ServiceLocations should not be supported without device info"
        );

        // Test with missing version
        let info = create_device_info(
            None,
            Some("os_family=windows; os_type=Windows; version=11".to_string()),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported without version info"
        );

        // Test with missing platform
        let info = create_device_info(Some("1.6.0".to_string()), None);
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported without platform info"
        );

        // Test with invalid version string
        let info = create_device_info(
            Some("invalid".to_string()),
            Some("os_family=windows; os_type=Windows; version=11".to_string()),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported with invalid version"
        );

        // Test with invalid platform string
        let info = create_device_info(Some("1.6.0".to_string()), Some("invalid".to_string()));
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported with invalid platform"
        );

        // Test case insensitivity of OS family matching
        let info = create_device_info(
            Some("1.6.0".to_string()),
            Some("os_family=WiNdOwS; os_type=Windows; version=11".to_string()),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported with mixed-case OS family"
        );

        // Test with pre-release version above minimum
        let info = create_device_info(
            Some("1.7.0-alpha1".to_string()),
            Some("os_family=windows; os_type=Windows; version=11".to_string()),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported with pre-release version above minimum"
        );

        // Test with pre-release version below minimum
        let info = create_device_info(
            Some("1.5.0-alpha1".to_string()),
            Some("os_family=windows; os_type=Windows; version=11".to_string()),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported with pre-release version below minimum"
        );
    }
}
