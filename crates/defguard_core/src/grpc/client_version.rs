use base64::{Engine, prelude::BASE64_STANDARD};
use defguard_common::db::models::wireguard::LocationMfaMode;
use defguard_proto::{client_types::ClientPlatformInfo, proxy::DeviceInfo};
use prost::Message;
use semver::Version;

use crate::enterprise::is_business_license_active;

/// Extracts the semantic client version and decoded platform metadata from proxy device info.
///
/// Invalid or missing fields are logged and returned as `None` so feature checks can fail closed
/// without rejecting the whole request.
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

/// Features whose availability depends on client version and platform metadata.
#[derive(Debug)]
pub enum ClientFeature {
    ServiceLocations,
    PostureChecks,
    MultiStepMfa,
}

/// One supported client/platform combination for a feature.
///
/// A feature is available when at least one of its rules matches. `None` platform fields behave as
/// wildcards, while `min_version` is always required.
#[derive(Debug)]
struct ClientFeatureRule {
    /// Oldest client version that supports this rule.
    min_version: Version,
    /// Required Rust OS family reported by the client, or any family when absent.
    os_family: Option<&'static str>,
    /// Required Rust OS type reported by the client, or any type when absent.
    os_type: Option<&'static str>,
}

impl ClientFeatureRule {
    /// Returns whether the supplied platform satisfies this rule's platform predicates.
    fn matches_platform(&self, platform: Option<&ClientPlatformInfo>) -> bool {
        let requires_platform = self.os_family.is_some() || self.os_type.is_some();
        let Some(platform) = platform else {
            return !requires_platform;
        };

        self.os_family
            .is_none_or(|family| platform.os_family.eq_ignore_ascii_case(family))
            && self
                .os_type
                .is_none_or(|os_type| platform.os_type.eq_ignore_ascii_case(os_type))
    }

    /// Returns whether both client version and platform satisfy this rule.
    fn matches(&self, version: Option<&Version>, platform: Option<&ClientPlatformInfo>) -> bool {
        version.is_some_and(|version| {
            let triple = (version.major, version.minor, version.patch);
            let floor = (
                self.min_version.major,
                self.min_version.minor,
                self.min_version.patch,
            );
            triple >= floor
        }) && self.matches_platform(platform)
    }
}

impl ClientFeature {
    /// Returns all platform/version combinations that support this feature.
    fn rules(&self) -> Vec<ClientFeatureRule> {
        match self {
            Self::ServiceLocations => vec![
                ClientFeatureRule {
                    min_version: Version::new(1, 6, 0),
                    os_family: Some("windows"),
                    os_type: None,
                },
                ClientFeatureRule {
                    min_version: Version::new(2, 1, 0),
                    os_family: None,
                    os_type: Some("linux"),
                },
            ],
            Self::PostureChecks => vec![
                // We do not keep mobile client and desktop client versions in sync.
                ClientFeatureRule {
                    min_version: Version::new(1, 7, 0),
                    os_family: Some("android"),
                    os_type: None,
                },
                ClientFeatureRule {
                    min_version: Version::new(1, 7, 0),
                    os_family: Some("ios"),
                    os_type: None,
                },
                ClientFeatureRule {
                    min_version: Version::new(2, 1, 0),
                    os_family: None,
                    os_type: None,
                },
            ],
            Self::MultiStepMfa => vec![ClientFeatureRule {
                min_version: Version::new(2, 2, 0),
                os_family: None,
                os_type: None,
            }],
        }
    }

    /// Returns `true` when the supplied device info supports this feature.
    ///
    /// Missing or invalid version information never matches. Missing platform information matches only
    /// rules without platform constraints.
    pub fn is_supported_by_device(&self, info: Option<&DeviceInfo>) -> bool {
        let (version, platform) = parse_client_version_platform(info);
        let rules = self.rules();
        let supported = rules
            .iter()
            .any(|rule| rule.matches(version.as_ref(), platform.as_ref()));

        if !supported {
            debug!(
                "Client version {version:?} and platform {:?} do not match support rules {:?} for feature {self:?}",
                platform
                    .as_ref()
                    .map(|platform| (&platform.os_family, &platform.os_type)),
                rules,
            );
        }

        supported
    }
}

/// Returns `true` when a location should be omitted from a device's config because the location's
/// MFA configuration has no legacy equivalent and either the device's client version does not
/// support multi-step MFA or multi-step MFA is unavailable without an active business license.
pub fn should_omit_location_for_device(
    location_mfa_mode: Option<LocationMfaMode>,
    device_info: Option<&DeviceInfo>,
) -> bool {
    location_mfa_mode.is_none()
        && (!ClientFeature::MultiStepMfa.is_supported_by_device(device_info)
            || !is_business_license_active())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enterprise::license::{
        License, LicenseTier, SupportType, get_cached_license, set_cached_license,
    };

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

        // Linux requires >= 2.1.0.
        let info = create_device_info(
            Some("2.0.9".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "unix".to_owned(),
                os_type: "linux".to_owned(),
                version: "22.04".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not be supported on Linux below version 2.1.0"
        );

        // Linux is supported since 2.1.0.
        let info = create_device_info(
            Some("2.1.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "unix".to_owned(),
                os_type: "linux".to_owned(),
                version: "22.04".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should be supported on Linux at version 2.1.0"
        );

        // Test with unsupported OS family (macos)
        let info = create_device_info(
            Some("2.1.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "unix".to_owned(),
                os_type: "macos".to_owned(),
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
    fn test_matches_compares_release_triples() {
        let rule = ClientFeatureRule {
            min_version: Version::new(2, 2, 0),
            os_family: None,
            os_type: None,
        };

        // A pre-release of the floor itself passes the gate.
        assert!(rule.matches(Some(&Version::parse("2.2.0-alpha1").unwrap()), None,));
        // A release below the floor still fails.
        assert!(!rule.matches(Some(&Version::parse("2.1.99").unwrap()), None,));
        // Build metadata is stripped, so a build of the floor passes.
        assert!(rule.matches(Some(&Version::parse("2.2.0+build.1").unwrap()), None,));
        // Missing version information never matches.
        assert!(!rule.matches(None, None));
    }

    #[test]
    fn test_own_floor_prerelease_passes_each_feature() {
        // ServiceLocations / Windows floor 1.6.0.
        let info = create_device_info(
            Some("1.6.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should support a 1.6.0 pre-release on Windows"
        );

        // ServiceLocations / Linux floor 2.1.0.
        let info = create_device_info(
            Some("2.1.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "unix".to_owned(),
                os_type: "linux".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should support a 2.1.0 pre-release on Linux"
        );

        // PostureChecks / desktop floor 2.1.0.
        let info = create_device_info(
            Some("2.1.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "linux".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
            "PostureChecks should support a 2.1.0 pre-release on desktop"
        );

        // PostureChecks / mobile floor 1.7.0.
        let info = create_device_info(
            Some("1.7.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "android".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::PostureChecks.is_supported_by_device(Some(&info)),
            "PostureChecks should support a 1.7.0 pre-release on Android"
        );

        // A below-floor pre-release still fails (ServiceLocations / Windows).
        let info = create_device_info(
            Some("1.5.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                os_type: "Windows".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::ServiceLocations.is_supported_by_device(Some(&info)),
            "ServiceLocations should not support a below-floor pre-release on Windows"
        );
    }

    #[test]
    fn test_multi_step_mfa_feature_support() {
        // Supported at the 2.2.0 floor regardless of platform.
        for (os_family, os_type) in [
            ("windows", "Windows"),
            ("unix", "linux"),
            ("macos", "macOS"),
        ] {
            let info = create_device_info(
                Some("2.2.0".to_owned()),
                Some(ClientPlatformInfo {
                    os_family: os_family.to_owned(),
                    os_type: os_type.to_owned(),
                    ..Default::default()
                }),
            );
            assert!(
                ClientFeature::MultiStepMfa.is_supported_by_device(Some(&info)),
                "MultiStepMfa should be supported on {os_family} at version 2.2.0"
            );
        }

        // A pre-release of the floor itself passes.
        let info = create_device_info(
            Some("2.2.0-alpha1".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            ClientFeature::MultiStepMfa.is_supported_by_device(Some(&info)),
            "MultiStepMfa should support a 2.2.0 pre-release"
        );

        // Below the floor is not supported.
        let info = create_device_info(
            Some("2.1.99".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                ..Default::default()
            }),
        );
        assert!(
            !ClientFeature::MultiStepMfa.is_supported_by_device(Some(&info)),
            "MultiStepMfa should not be supported below version 2.2.0"
        );

        // Indifferent to platform: a missing platform still passes (no platform constraints).
        let info = create_device_info(Some("2.2.0".to_owned()), None);
        assert!(
            ClientFeature::MultiStepMfa.is_supported_by_device(Some(&info)),
            "MultiStepMfa should be supported without platform info"
        );

        // Missing version information fails.
        let info = create_device_info(None, None);
        assert!(
            !ClientFeature::MultiStepMfa.is_supported_by_device(Some(&info)),
            "MultiStepMfa should not be supported without version info"
        );

        // Malformed version fails.
        let info = create_device_info(Some("invalid".to_owned()), None);
        assert!(
            !ClientFeature::MultiStepMfa.is_supported_by_device(Some(&info)),
            "MultiStepMfa should not be supported with an invalid version"
        );

        // No device info fails.
        assert!(
            !ClientFeature::MultiStepMfa.is_supported_by_device(None),
            "MultiStepMfa should not be supported without device info"
        );
    }

    /// Builds a valid Business-tier license for tests that exercise licensed behavior.
    ///
    /// Setting this mutates the process-global license cache, so callers save and restore it
    /// around their body. The restore is best-effort: a parallel test that also mutates the cache
    /// can still race.
    fn business_license() -> License {
        License {
            customer_id: "test".to_owned(),
            subscription: false,
            valid_until: None,
            limits: None,
            version_date_limit: None,
            tier: LicenseTier::Business,
            support_type: SupportType::Basic,
            features: vec![],
        }
    }

    #[test]
    fn test_should_omit_location_for_device() {
        let saved_license = get_cached_license().clone();

        // Legacy client (below the MultiStepMfa floor).
        let legacy = create_device_info(
            Some("2.1.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                ..Default::default()
            }),
        );

        // A capable client (at the MultiStepMfa floor).
        let capable = create_device_info(
            Some("2.2.0".to_owned()),
            Some(ClientPlatformInfo {
                os_family: "windows".to_owned(),
                ..Default::default()
            }),
        );

        // Legacy client with legacy-derivable modes is included.
        assert!(!should_omit_location_for_device(
            Some(LocationMfaMode::Internal),
            Some(&legacy),
        ));
        assert!(!should_omit_location_for_device(
            Some(LocationMfaMode::Disabled),
            Some(&legacy),
        ));

        // Legacy client with no legacy equivalent is omitted.
        assert!(should_omit_location_for_device(None, Some(&legacy)));

        // Capable client with no legacy equivalent but no business license is omitted.
        set_cached_license(None);
        assert!(should_omit_location_for_device(None, Some(&capable)));

        // Capable client with no legacy equivalent and an active business license is included.
        set_cached_license(Some(business_license()));
        assert!(!should_omit_location_for_device(None, Some(&capable)));

        set_cached_license(saved_license);
    }
}
