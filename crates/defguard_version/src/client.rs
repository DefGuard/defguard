use tonic::{Request, Status};
use tracing::warn;

use crate::{ComponentInfo, SYSTEM_INFO_HEADER, VERSION_HEADER};

/// Adds version and system-info headers to outgoing requests
///
/// # Headers Added
///
/// - `defguard-version`: The semantic version of the Defguard component
/// - `defguard-system`: System information including OS type, version, and architecture
///
/// # Examples
/// ```ignore
/// use tonic::transport::Channel;
///
/// use defguard_version::client::version_interceptor;
/// let interceptor = version_interceptor("1.0.0");
/// let channel = Channel::from_static("http://localhost:50051").connect().await.unwrap();
/// let client = MyClient::with_interceptor(channel, interceptor);
/// ```
pub fn version_interceptor(
    version: &str,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    let component_info = ComponentInfo::new(version)
        .inspect_err(|err| warn!("Failed to get component info: {err}"))
        .ok();

    move |mut request: Request<()>| -> Result<Request<()>, Status> {
        let Some(component_info) = &component_info else {
            return Ok(request);
        };

        let metadata = request.metadata_mut();

        // Add version header
        match component_info.version.to_string().parse() {
            Ok(version_value) => {
                metadata.insert(VERSION_HEADER, version_value);
            }
            Err(err) => warn!("Failed to parse version: {err}"),
        }

        // Add system info header
        match component_info.system.as_header_value().parse() {
            Ok(system_info_value) => {
                metadata.insert(SYSTEM_INFO_HEADER, system_info_value);
            }
            Err(err) => warn!("Failed to parse system info: {err}"),
        }

        Ok(request)
    }
}
