use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use defguard_version::{ComponentInfo, Version, is_version_lower};
use tonic::{Status, service::Interceptor};

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

#[derive(Clone)]
pub struct GatewayVersionInterceptor {
    min_version: Version,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
}

impl GatewayVersionInterceptor {
    #[must_use]
    pub fn new(
        min_version: Version,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    ) -> Self {
        Self {
            min_version,
            incompatible_components,
        }
    }

    #[must_use]
    fn is_version_supported(&self, version: Option<&Version>) -> bool {
        let Some(version) = version else {
            error!(
                "Missing gateway version information. This most likely means that gateway component uses \
                older, unsupported version. Minimal supported version is {}.",
                self.min_version,
            );
            return false;
        };

        if is_version_lower(version, &self.min_version) {
            error!(
                "Gateway version {version} is not supported. Minimal supported gateway version is {}.",
                self.min_version
            );
            return false;
        }

        debug!("Gateway version {version} is supported");
        true
    }
}

impl Interceptor for GatewayVersionInterceptor {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        let maybe_info = ComponentInfo::from_metadata(request.metadata());
        let version = maybe_info.as_ref().map(|info| &info.version);
        if !self.is_version_supported(version) {
            let msg = match version {
                Some(version) => format!("Version {version} not supported"),
                None => "Missing version headers".to_string(),
            };
            let maybe_hostname = request
                .metadata()
                .get("hostname")
                .and_then(|v| v.to_str().ok())
                .map(String::from);
            let data = IncompatibleGatewayData::new(version.cloned(), maybe_hostname);
            data.insert(&mut self.incompatible_components);
            return Err(Status::failed_precondition(msg));
        }

        Ok(request)
    }
}

#[derive(Default)]
pub struct IncompatibleComponents {
    pub gateways: HashSet<IncompatibleGatewayData>,
    pub proxy: Option<IncompatibleProxyData>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IncompatibleGatewayData {
    pub version: Option<Version>,
    pub hostname: Option<String>,
}

impl IncompatibleGatewayData {
    pub fn new(version: Option<Version>, hostname: Option<String>) -> Self {
        Self { version, hostname }
    }

    /// Inserts metadata into the HashSet while avoiding write-locking the structure unnecessarily.
    pub fn insert(&self, components: &mut Arc<RwLock<IncompatibleComponents>>) -> bool {
        if components
            .read()
            .expect("Failed to read-lock IncompatibleComponents")
            .gateways
            .contains(self)
        {
            return false;
        }
        components
            .write()
            .expect("Failed to write-lock IncompatibleComponents")
            .gateways
            .insert(self.clone())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncompatibleProxyData {
    pub version: Version,
}

impl IncompatibleProxyData {
    pub fn new(version: Version) -> Self {
        Self { version }
    }
}
