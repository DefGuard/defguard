use tonic::{
    async_trait, body::Body,
    codegen::http::{Request, Response},
};
use tonic_middleware::{Middleware, ServiceBound};

use crate::{ComponentInfo, DefguardVersionError, SYSTEM_INFO_HEADER, VERSION_HEADER};

#[derive(Clone)]
pub struct DefguardVersionServerMiddleware {
    component_info: ComponentInfo,
}

impl DefguardVersionServerMiddleware {
    pub fn new(version: &str) -> Result<Self, DefguardVersionError> {
        Ok(Self {
            component_info: ComponentInfo::new(version)?,
        })
    }
}

#[async_trait]
impl<S> Middleware<S> for DefguardVersionServerMiddleware
where
    S: ServiceBound,
    S::Future: Send,
{
    async fn call(
        &self,
        request: Request<Body>,
        mut service: S,
    ) -> Result<Response<Body>, S::Error> {
        let mut response = service.call(request).await?;
        response.headers_mut().insert(
            VERSION_HEADER,
            self.component_info.version.to_string().parse().unwrap(),
        );
        response.headers_mut().insert(
            SYSTEM_INFO_HEADER,
            self.component_info
                .system
                .as_header_value()
                .parse()
                .unwrap(),
        );
        Ok(response)
    }
}
