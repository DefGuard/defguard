use defguard_proto::proxy::DeviceInfo;
use semver::Version;

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
                "Client OS {:?} does not meet required OS {:?} for feature {:?}",
                platform.as_ref().map(|p| &p.os_family),
                self.required_os_family(),
                self
            );
        }

        version_matches && platform_matches
    }
}
