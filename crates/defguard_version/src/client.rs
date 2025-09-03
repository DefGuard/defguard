use tonic::{Request, Status, service::Interceptor};
use tracing::warn;

use crate::{ComponentInfo, SYSTEM_INFO_HEADER, VERSION_HEADER};

/// Adds version and system-info headers to outgoing requests
///
/// # Headers Added
///
/// - `defguard-version`: Semantic version of the component.
/// - `defguard-system`: System information including OS type, version and architecture.
#[derive(Clone)]
pub struct ClientVersionInterceptor {
    component_info: ComponentInfo,
}

impl ClientVersionInterceptor {
    #[must_use]
    pub fn new(version: crate::Version) -> Self {
        Self {
            component_info: ComponentInfo::new(version),
        }
    }
}

impl Interceptor for ClientVersionInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let metadata = request.metadata_mut();

        // Add version header
        match self.component_info.version.to_string().parse() {
            Ok(version_value) => {
                metadata.insert(VERSION_HEADER, version_value);
            }
            Err(err) => warn!("Failed to parse version: {err}"),
        }

        // Add system info header
        match self.component_info.system.as_header_value().parse() {
            Ok(system_info_value) => {
                metadata.insert(SYSTEM_INFO_HEADER, system_info_value);
            }
            Err(err) => warn!("Failed to parse system info: {err}"),
        }

        Ok(request)
    }
}
