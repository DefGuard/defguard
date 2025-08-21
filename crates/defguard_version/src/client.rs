use tonic::{Request, Status};
use tracing::warn;

use crate::{ComponentInfo, SYSTEM_INFO_HEADER, VERSION_HEADER};

/// Adds version and system-info headers to outgoing requests
///
/// # Headers Added
///
/// - `defguard-version`: Semantic version of the component
/// - `defguard-system`: System information including OS type, version and architecture
///
/// # Examples
/// ```ignore
/// use semver::Version;
/// use tonic::transport::Channel;
///
/// use defguard_version::client::version_interceptor;
/// let version = Version::parse("1.0.0").unwrap();
/// let interceptor = version_interceptor(version);
/// let channel = Channel::from_static("http://localhost:50051").connect().await.unwrap();
/// let client = MyClient::with_interceptor(channel, interceptor);
/// ```
pub fn version_interceptor(
    version: crate::Version,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    let component_info = ComponentInfo::new(version);

    move |mut request: Request<()>| -> Result<Request<()>, Status> {
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
