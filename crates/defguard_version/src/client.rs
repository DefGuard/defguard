use tonic::{Request, Status};
use tracing::warn;

use crate::{ComponentInfo, SYSTEM_INFO_HEADER, VERSION_HEADER};

/// Adds version and system-info headers to outgoing requests
///
/// # Examples
/// ```rust
/// use defguard_version::client::version_interceptor;
/// let interceptor = version_interceptor("1.0.0");
/// let client = MyClient::with_interceptor(channel, interceptor);
/// ```
pub fn version_interceptor(
    version: &str,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone {
    let component_info = ComponentInfo::new(version);
    if let Err(ref err) = component_info {
        warn!("Failed to get component info: {err}");
    };
    let component_info = component_info.ok();

    move |mut request: Request<()>| -> Result<Request<()>, Status> {
        let Some(component_info) = &component_info else {
            return Ok(request);
        };

        let metadata = request.metadata_mut();

        // Add version header
        let version_value = component_info
            .version
            .to_string()
            .parse()
            .map_err(|_| Status::internal("Failed to parse version as metadata value"))?;
        metadata.insert(VERSION_HEADER, version_value);

        // Add system info header
        let system_info_value = component_info
            .system
            .as_header_value()
            .parse()
            .map_err(|_| Status::internal("Failed to parse system info as metadata value"))?;
        metadata.insert(SYSTEM_INFO_HEADER, system_info_value);

        Ok(request)
    }
}
