use std::{
    sync::{Arc, RwLock},
};

use tonic::{
    async_trait,
    body::BoxBody,
    codegen::http::{Request, Response},
};
use tonic_middleware::{Middleware, ServiceBound};
use tracing::error;

use crate::{parse_version_headers, ComponentInfo, SYSTEM_INFO_HEADER, VERSION_HEADER};

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
        let version = request.headers().get(VERSION_HEADER);
        let info = request.headers().get(SYSTEM_INFO_HEADER);

        if let Some((version, system)) = parse_version_headers(version, info) {
            error!("OWN VERSION: {}", self.own_info.version);
            error!("OWN SYSTEM: {}", self.own_info.system);
            error!("CLIENT VERSION: {}", version);
            error!("CLIENT SYSTEM: {}", system);
            *self.remote_info.write().unwrap() = Some(ComponentInfo { version, system });
        }

        let mut response = service.call(request).await?;
        response.headers_mut().insert(
            VERSION_HEADER,
            self.own_info.version.to_string().parse().unwrap(),
        );
        response.headers_mut().insert(
            SYSTEM_INFO_HEADER,
            self.own_info.system.as_header_value().parse().unwrap(),
        );
        Ok(response)
    }
}
