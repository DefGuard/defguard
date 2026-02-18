use http::Uri;

/// Rewrites the request URI scheme to https for the TLS connector.
///
/// Tonic expects an http URI for its endpoint, but a custom connector performs
/// the TLS handshake and requires https to select the TLS path.
#[derive(Clone, Debug)]
pub struct HttpsSchemeConnector<C> {
    inner: C,
}

impl<C> HttpsSchemeConnector<C> {
    pub const fn new(inner: C) -> Self {
        Self { inner }
    }
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

impl<C> tower_service::Service<Uri> for HttpsSchemeConnector<C>
where
    C: tower_service::Service<Uri, Error = BoxError> + Clone + Send + 'static,
    C::Future: Send,
{
    type Response = C::Response;
    type Error = BoxError;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let mut parts = uri.into_parts();
        parts.scheme = Some(http::uri::Scheme::HTTPS);
        let https_uri = match Uri::from_parts(parts) {
            Ok(uri) => uri,
            Err(err) => {
                return Box::pin(async move { Err(err.into()) });
            }
        };
        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(https_uri).await })
    }
}
