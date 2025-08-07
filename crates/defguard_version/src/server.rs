use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tonic::{Request, Response, Status};
use tower::{Layer, Service};
use tracing::error;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// For response modification, we need a more specific approach
#[derive(Clone)]
pub struct DefguardVersionLayer;

impl<S> Layer<S> for DefguardVersionLayer {
    type Service = DefguardVersionMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DefguardVersionMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct DefguardVersionMiddleware<S> {
	inner: S,
}

// impl<S, B> Service<Request<B>> for DefguardVersionMiddleware<S>
// where
//     S: Service<Request<B>, Response = Response<B>> + Clone + Send + 'static,
//     S::Future: Send + 'static,
//     B: Send + 'static,
// {
//     type Response = Response<B>;
//     type Error = S::Error;
//     type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.inner.poll_ready(cx)
//     }

//     fn call(&mut self, request: Request<B>) -> Self::Future {
//         let mut inner = self.inner.clone();

//         Box::pin(async move {
//             // Read request metadata if needed
//             let client_version = request
//                 .metadata()
//                 .get("dfg-version")
//                 .and_then(|v| v.to_str().ok())
//                 .unwrap_or("unknown")
//                 .to_string();

//             error!("Remote version: {}", client_version);
//             error!("Own version: TODO");
//             for header in request.metadata().keys() {
//                 error!("key: {:?}", header);
//             }
//             // Call inner service
//             let mut response = inner.call(request).await?;

//             response
//                 .metadata_mut()
//                 .insert("dfg-version", "1.555.555".parse().unwrap());

//             Ok(response)
//         })
//     }
// }


impl<S, ReqBody> Service<Request<ReqBody>> for DefguardVersionMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<MyResponseType>, Error = Status> + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = Response<MyResponseType>;
    type Error = Status;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let fut = self.inner.call(req);

        Box::pin(async move {
            let mut response = fut.await?;

            // 🔁 Modify the inner message
            let modified = modify_response(response.into_inner());

            Ok(Response::new(modified))
        })
    }
}
