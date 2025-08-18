use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tonic::{
    body::Body,
    codegen::http::{Request, Response},
    server::NamedService,
};
use tower::{Layer, Service};

use crate::{ComponentInfo, DefguardVersionError, SYSTEM_INFO_HEADER, VERSION_HEADER};

#[derive(Clone)]
pub struct DefguardVersionLayer {
    component_info: ComponentInfo,
}

impl DefguardVersionLayer {
    pub fn new(version: &str) -> Result<Self, DefguardVersionError> {
        Ok(Self {
            component_info: ComponentInfo::new(version)?,
        })
    }
}

impl<S> Layer<S> for DefguardVersionLayer {
    type Service = DefguardVersionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DefguardVersionService {
            inner,
            component_info: self.component_info.clone(),
        }
    }
}

#[derive(Clone)]
pub struct DefguardVersionService<S> {
    inner: S,
    component_info: ComponentInfo,
}

impl<S, B> Service<Request<Body>> for DefguardVersionService<S>
where
    S: Service<Request<Body>, Response = Response<B>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
{
    type Response = Response<B>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let mut inner = self.inner.clone();
        let component_info = self.component_info.clone();

        Box::pin(async move {
            let mut response = inner.call(request).await?;
            
            response.headers_mut().insert(
                VERSION_HEADER,
                component_info.version.to_string().parse().unwrap(),
            );
            response.headers_mut().insert(
                SYSTEM_INFO_HEADER,
                component_info
                    .system
                    .as_header_value()
                    .parse()
                    .unwrap(),
            );
            
            Ok(response)
        })
    }
}

impl<S> NamedService for DefguardVersionService<S>
where
    S: NamedService,
{
    const NAME: &'static str = S::NAME;
}