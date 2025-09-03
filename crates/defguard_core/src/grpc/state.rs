use std::sync::{LazyLock, RwLock};

use defguard_version::{DefguardComponent, tracing::VersionInfo};

pub(crate) const PROXY_STATE: LazyLock<RwLock<VersionInfo>> = LazyLock::new(|| {
    RwLock::new(VersionInfo {
        component: Some(DefguardComponent::Proxy),
        info: None,
        version: None,
        is_supported: false,
    })
});
