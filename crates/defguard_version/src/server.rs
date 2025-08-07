use std::sync::{Arc, RwLock};

use tonic::{
    async_trait,
    body::BoxBody,
    codegen::http::{Request, Response},
};
use tonic_middleware::{Middleware, ServiceBound};
use tracing::{error, warn};

use crate::{ComponentInfo, SYSTEM_INFO_HEADER, SemanticVersion, SystemInfo, VERSION_HEADER};

#[derive(Clone)]
pub struct DefguardVersionMiddleware {
    own_info: ComponentInfo,
    remote_info: Arc<RwLock<Option<ComponentInfo>>>,
}

impl DefguardVersionMiddleware {
    pub fn new(own_info: ComponentInfo, remote_info: Arc<RwLock<Option<ComponentInfo>>>) -> Self {
        Self {
            own_info,
            remote_info,
        }
    }
}

#[async_trait]
impl<S> Middleware<S> for DefguardVersionMiddleware
where
    S: ServiceBound,
    S::Future: Send,
{
    async fn call(
        &self,
        request: Request<BoxBody>,
        mut service: S,
    ) -> Result<Response<BoxBody>, S::Error> {
        let client_version = request.headers().get(VERSION_HEADER);
        // .and_then(|v| v.to_str().ok())
        // .unwrap_or("unknown")
        // .to_string();

        let client_info = request.headers().get(SYSTEM_INFO_HEADER);
        // .and_then(|v| v.to_str().ok())
        // .unwrap_or("unknown")
        // .to_string();

        // error!("Remote version: {}", client_version);
        // error!("Remote system: {}", client_info);

        if let (Some(client_version), Some(_client_info)) = (client_version, client_info) {
            if let Ok(version) = client_version.to_str() {
                if let Ok(version) = SemanticVersion::try_from(version) {
                    error!("OWN VERSION: {}", self.own_info.version.to_string());
                    error!("CLIENT VERSION: {}", version.to_string());
                    // TODO
                    let system = SystemInfo {
                        os_type: "?".to_string(),
                        os_version: "?".to_string(),
                        os_edition: "?".to_string(),
                        os_codename: "?".to_string(),
                        bitness: "?".to_string(),
                        architecture: "?".to_string(),
                    };
                    *self.remote_info.write().unwrap() = Some(ComponentInfo { version, system });
                }
            }
        } else {
            warn!("Missing version and/or system info header");
        }

        let mut response = service.call(request).await?;
        response.headers_mut().insert(
            VERSION_HEADER,
            self.own_info.version.to_string().parse().unwrap(),
        );

        Ok(response)
    }
}
