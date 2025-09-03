use defguard_version::{Version, is_version_lower};

const MIN_PROXY_VERSION: Version = Version::new(1, 5, 0);
pub const MIN_GATEWAY_VERSION: Version = Version::new(1, 5, 0);

/// Checks if Defguard Proxy version meets minimum version requirements.
pub(crate) fn is_proxy_version_supported(version: Option<&Version>) -> bool {
    let Some(version) = version else {
        error!(
            "Missing proxy component version information. This most likely means that proxy \
            component uses older, unsupported version. Minimal supported proxy version is \
            {MIN_PROXY_VERSION}."
        );
        return false;
    };

    if is_version_lower(version, &MIN_PROXY_VERSION) {
        error!(
            "Proxy version {version} is not supported. Minimal supported proxy version is \
            {MIN_PROXY_VERSION}."
        );

        return false;
    }

    info!("Proxy version {version} is supported");
    true
}
