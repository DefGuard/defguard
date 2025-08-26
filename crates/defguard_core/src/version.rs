use defguard_version::Version;

const MIN_PROXY_VERSION: Version = Version::new(1, 5, 0);
pub const MIN_GATEWAY_VERSION: Version = Version::new(1, 6, 0);

/// Checks if the proxy version meets minimum version requirements.
pub(crate) fn is_proxy_version_supported(version: Option<&Version>) -> bool {
    let Some(version) = version else {
        error!(
            "Missing proxy component version information. This most likely means that proxy component uses older, unsupported version. Minimal supported proxy version is {MIN_PROXY_VERSION}."
        );
        return false;
    };
    if version < &MIN_PROXY_VERSION {
        error!(
            "Proxy version {version} is not supported. Minimal supported proxy version is {MIN_PROXY_VERSION}."
        );
        return false;
    }

    info!("Proxy version {version} is supported");
    true
}

/// Checks if the gateway version meets minimum version requirements.
pub(crate) fn is_gateway_version_supported(version: Option<&Version>) -> bool {
    let Some(version) = version else {
        error!(
            "Missing gateway component version information. This most likely means that gateway component uses unsupported version. Minimal supported gateway version is {MIN_GATEWAY_VERSION}"
        );
        return false;
    };
    if version < &MIN_GATEWAY_VERSION {
        error!(
            "Gateway version {version} is not supported. Minimal supported gateway version is {MIN_GATEWAY_VERSION}."
        );
        return false;
    }

    info!("Gateway version {version} is supported");
    true
}
