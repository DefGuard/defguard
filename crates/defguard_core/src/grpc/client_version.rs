use base64::{Engine, prelude::BASE64_STANDARD};
use defguard_proto::proxy::{ClientPlatformInfo, DeviceInfo};
use prost::Message;
use semver::Version;

pub(crate) fn parse_client_version_platform(
    info: Option<&DeviceInfo>,
) -> (Option<Version>, Option<ClientPlatformInfo>) {
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
        let binary = BASE64_STANDARD
            .decode(p)
            .map_err(|e| {
                error!("Failed to decode base64 platform string: {e}");
                e
            })
            .ok()?;
        let platform_info = ClientPlatformInfo::decode(&*binary)
            .map_err(|e| {
                error!("Failed to decode ClientPlatformInfo from bytes: {e}");
                e
            })
            .ok()?;
        Some(platform_info)
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
