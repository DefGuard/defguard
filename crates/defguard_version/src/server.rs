use tonic::{
    async_trait,
    body::BoxBody,
    codegen::http::{Request, Response},
};
use tonic_middleware::{Middleware, ServiceBound};

use crate::{ComponentInfo, DefguardVersionError, SYSTEM_INFO_HEADER, VERSION_HEADER};

#[derive(Clone)]
pub struct DefguardVersionServerMiddleware {
    component_info: ComponentInfo,
}

impl DefguardVersionServerMiddleware {
    pub fn from_str(version: &str) -> Result<Self, DefguardVersionError> {
        Ok(Self {
            component_info: ComponentInfo::from_str(version)?,
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
        request: Request<BoxBody>,
        mut service: S,
    ) -> Result<Response<BoxBody>, S::Error> {
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
