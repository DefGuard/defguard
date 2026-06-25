use base64::{Engine, prelude::BASE64_STANDARD};
use defguard_proto::{client_types::ClientPlatformInfo, proxy::DeviceInfo};
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

/// Returns `true` when the platform's OS family is a mobile OS (Android or iOS).
/// Returns `false` for desktop platforms and when platform info is absent.
pub fn is_mobile_platform(platform: Option<&ClientPlatformInfo>) -> bool {
    platform
        .map(|p| {
            let f = p.os_family.to_lowercase();
            f == "android" || f == "ios"
        })
        .unwrap_or(false)
}

/// Represents a client feature that may have minimum version and OS family requirements.
#[derive(Debug)]
pub enum ClientFeature {
    ServiceLocations,
    PostureChecks,
}

impl ClientFeature {
    fn min_version(&self, platform: Option<&ClientPlatformInfo>) -> Option<Version> {
        match self {
            Self::ServiceLocations => Some(Version::new(1, 6, 0)),
            Self::PostureChecks => {
                if is_mobile_platform(platform) {
                    Some(Version::new(1, 7, 0))
                } else {
                    Some(Version::new(2, 1, 0))
                }
            }
        }
    }

    fn required_os_family(&self) -> Option<Vec<&'static str>> {
        match self {
            Self::ServiceLocations => Some(vec!["windows"]),
            Self::PostureChecks => None,
        }
    }

    pub fn is_supported_by_device(&self, info: Option<&DeviceInfo>) -> bool {
        let (version, platform) = parse_client_version_platform(info);

        // No minimum version = matches all
        let version_matches = self
            .min_version(platform.as_ref())
            .is_none_or(|min_version| {
                // No version info = does not match
                version
                    .as_ref()
                    .is_some_and(|version| version >= &min_version)
            });

        if !version_matches {
            debug!(
                "Client version {version:?} does not meet minimum version {:?} for feature {self:?}",
                self.min_version(platform.as_ref())
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
    fn create_device_info(
        version: Option<String>,
        platform: Option<ClientPlatformInfo>,
    ) -> DeviceInfo {
        let platform = platform.map(|p| {
            let mut buf = Vec::new();
            p.encode(&mut buf).unwrap();
            BASE64_STANDARD.encode(&buf)
        });

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
            Some("1.5.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
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
            Some("invalid.version".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "linux".to_owned(),
                os_type: "Ubuntu".to_owned(),
                version: "22.04".to_owned(),
                ..Default::default()
            }),
        );
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_none());
        assert!(platform.is_some());

        // Test with missing version field
        let info = create_device_info(
            None,
            Some(ClientPlatformInfo {
                os_family: "linux".to_owned(),
                os_type: "Ubuntu".to_owned(),
                version: "22.04".to_owned(),
                ..Default::default()
            }),
        );
        let (version, platform) = parse_client_version_platform(Some(&info));
        assert!(version.is_none());
        assert!(platform.is_some());

        // Test with missing platform field
        let info = create_device_info(Some("1.5.0".to_owned()), None);
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
            Some("1.5.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "macos".to_owned(),
                os_type: "macOS".to_owned(),
                version: "14.0".to_owned(),
                ..Default::default()
            }),
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
            Some("1.6.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported on Windows with version 1.6.0"
        );

        // Test with exact minimum version
        let info = create_device_info(
            Some("1.6.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "Windows".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported at minimum version"
        );

        // Test with higher version
        let info = create_device_info(
            Some("2.0.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "WINDOWS".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported with higher version"
        );

        // Test with version below minimum
        let info = create_device_info(
            Some("1.5.9".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported below minimum version"
        );

        // Test with wrong OS family (linux)
        let info = create_device_info(
            Some("1.6.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "linux".to_owned(),
                os_type: "Ubuntu".to_owned(),
                version: "22.04".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported on Linux"
        );

        // Test with wrong OS family (macos)
        let info = create_device_info(
            Some("1.6.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "macos".to_owned(),
                os_type: "macOS".to_owned(),
                version: "14.0".to_owned(),
                ..Default::default()
            }),
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
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported without version info"
        );

        // Test with missing platform
        let info = create_device_info(Some("1.6.0".to_owned()), None);
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported without platform info"
        );

        // Test with invalid version string
        let info = create_device_info(
            Some("invalid".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported with invalid version"
        );

        // Test case insensitivity of OS family matching
        let info = create_device_info(
            Some("1.6.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "WiNdOwS".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported with mixed-case OS family"
        );

        // Test with pre-release version above minimum
        let info = create_device_info(
            Some("1.7.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported with pre-release version above minimum"
        );

        // Test with pre-release version below minimum
        let info = create_device_info(
            Some("1.5.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                version: "11".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported with pre-release version below minimum"
        );
    }

    #[test]
    fn test_posture_checks_feature_support() {
        // Desktop platforms require >= 2.1.0.
        for os_family in ["windows", "macos", "linux"] {
            let info = create_device_info(
                Some("2.1.0".to_owned()),
                Some(ClientPlatformInfo {
                    os_family: os_family.to_owned(),
                    ..Default::default()
                }),
            );
            assert!(
                ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
                "PostureChecks should be supported on {os_family} at minimum version"
            );
        }

        // Desktop version above minimum is supported.
        let info = create_device_info(
            Some("2.5.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "linux".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
            "PostureChecks should be supported with higher desktop version"
        );

        // Desktop version below minimum is not supported.
        let info = create_device_info(
            Some("2.0.9".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "linux".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
            "PostureChecks should not be supported below minimum desktop version"
        );

        // Mobile platforms (Android/iOS) require >= 1.7.0.
        for os_family in ["android", "ios", "Android", "IOS"] {
            let info = create_device_info(
                Some("1.7.0".to_owned()),
                Some(ClientPlatformInfo {
                    os_family: os_family.to_owned(),
                    ..Default::default()
                }),
            );
            assert!(
                ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
                "PostureChecks should be supported on {os_family} at version 1.7.0"
            );
        }

        // Mobile version above minimum is supported.
        let info = create_device_info(
            Some("1.8.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "android".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
            "PostureChecks should be supported on Android above minimum version"
        );

        // Mobile version below 1.7.0 is not supported.
        for os_family in ["android", "ios"] {
            let info = create_device_info(
                Some("1.6.4".to_owned()),
                Some(ClientPlatformInfo {
                    os_family: os_family.to_owned(),
                    ..Default::default()
                }),
            );
            assert!(
                !ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
                "PostureChecks should not be supported on {os_family} below version 1.7.0"
            );
        }

        // Missing version info means the feature is not supported.
        let info = create_device_info(None, None);
        assert!(
            !ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
            "PostureChecks should not be supported without version info"
        );

        // No device info at all means the feature is not supported.
        assert!(
            !ClientFeature::PostureChecks.is_supported_by_device(None),
            "PostureChecks should not be supported without device info"
        );
    }

    #[test]
    fn test_is_mobile_platform() {
        for os_family in ["android", "Android", "ANDROID", "ios", "iOS", "IOS"] {
            let platform = ClientPlatformInfo {
                os_family: os_family.to_owned(),
                ..Default::default()
            };
            assert!(
                is_mobile_platform(Some(&platform)),
                "{os_family} should be recognised as a mobile platform"
            );
        }

        for os_family in ["windows", "macos", "linux", "darwin", "unknown"] {
            let platform = ClientPlatformInfo {
                os_family: os_family.to_owned(),
                ..Default::default()
            };
            assert!(
                !is_mobile_platform(Some(&platform)),
                "{os_family} should not be recognised as a mobile platform"
            );
        }

        assert!(
            !is_mobile_platform(None),
            "None platform should not be recognised as mobile"
        );
    }
}
