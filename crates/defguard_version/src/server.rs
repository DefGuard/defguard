use tonic::{
    async_trait,
    body::BoxBody,
    codegen::http::{Request, Response},
};
use tonic_middleware::{Middleware, ServiceBound};
use tracing::error;

#[derive(Default, Clone)]
pub struct DefguardVersionMiddleware;

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
        let client_version = request
            .headers()
            .get("dfg-version")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        error!("Remote version: {}", client_version);
        error!("Own version: TODO");
        for header in request.headers().keys() {
            error!("key: {:?}", header);
        }
        // Call inner service
        let mut response = service.call(request).await?;

        response
            .headers_mut()
            .insert("dfg-version", "1.555.555".parse().unwrap());

        Ok(response)
    }
}
