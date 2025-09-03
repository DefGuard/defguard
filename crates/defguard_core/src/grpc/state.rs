//! Shared info about connected Defguard Proxy.

use std::sync::{LazyLock, RwLock};

use defguard_version::{DefguardComponent, tracing::VersionInfo};

pub(crate) static PROXY_STATE: LazyLock<RwLock<VersionInfo>> = LazyLock::new(|| {
    RwLock::new(VersionInfo {
        component: Some(DefguardComponent::Proxy),
        info: None,
        version: None,
        is_supported: false,
    })
});
