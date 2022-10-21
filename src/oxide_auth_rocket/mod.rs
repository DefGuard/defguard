//! This is a copy of that crate
//! https://github.com/HeroicKatora/oxide-auth/tree/master/oxide-auth-rocket
//! with the following pull request applied
//! https://github.com/HeroicKatora/oxide-auth/pull/132

mod failure;

use std::{io::Cursor, marker::PhantomData};

use rocket::{
    data::ToByteUnit,
    http::{ContentType, Header, Status},
    outcome::Outcome,
    request::FromRequest,
    response::{self, Responder},
    {uri, Data, Request, Response},
};

use oxide_auth::{
    endpoint::{NormalizedParameter, WebRequest, WebResponse},
    frontends::dev::{Cow, QueryParameter, Url},
};

pub use self::failure::OAuthFailure;
pub use oxide_auth::frontends::simple::{endpoint::Generic, request::NoError};

/// Request guard that also buffers OAuth data internally.
pub struct OAuthRequest<'r> {
    auth: Option<String>,
    pub query: Result<NormalizedParameter, WebError>,
    body: Result<Option<NormalizedParameter>, WebError>,
    lifetime: PhantomData<&'r ()>,
}

/// Response type for Rocket OAuth requests
///
/// A simple wrapper type around a simple `rocket::Response<'r>` that implements `WebResponse`.
#[derive(Debug, Default)]
pub struct OAuthResponse<'r>(Response<'r>);

/// Request error at the http layer.
///
/// For performance and consistency reasons, the processing of a request body and data is delayed
/// until it is actually required. This in turn means that some invalid requests will only be
/// caught during the OAuth process. The possible errors are collected in this type.
#[derive(Clone, Copy, Debug)]
pub enum WebError {
    /// A parameter was encoded incorrectly.
    ///
    /// This may happen for example due to a query parameter that is not valid utf8 when the query
    /// parameters are necessary for OAuth processing.
    Encoding,
    /// The body was needed but not provided.
    BodyNeeded,
    /// Form data was requested but the request was not a form.
    NotAForm,
}

impl<'r> OAuthRequest<'r> {
    /// Create the request data from request headers.
    ///
    /// Some oauth methods need additionally the body data which you can attach later.
    #[must_use]
    pub fn new(request: &Request<'_>) -> Self {
        let default_query_uri = uri!("?b");
        let default_query = default_query_uri.query().unwrap();
        let query = request.uri().query().unwrap_or(default_query);
        let query = match serde_urlencoded::from_str(&query.to_string()) {
            Ok(query) => Ok(query),
            Err(_) => Err(WebError::Encoding),
        };

        let body = match request.content_type() {
            Some(ct) if *ct == ContentType::Form => Ok(None),
            _ => Err(WebError::NotAForm),
        };

        let mut all_auth = request.headers().get("Authorization");
        let optional = all_auth.next();

        // Duplicate auth header, just treat it as no authorization.
        let auth = if all_auth.next().is_some() {
            None
        } else {
            optional.map(str::to_owned)
        };

        OAuthRequest {
            auth,
            query,
            body,
            lifetime: PhantomData,
        }
    }

    /// Provide the body of the request.
    ///
    /// Some, but not all operations, require reading their data from a urlencoded POST body. To
    /// simplify the implementation of primitives and handlers, this type is the central request
    /// type for both these use cases. When you forget to provide the body to a request, the oauth
    /// system will return an error the moment the request is used.
    pub async fn add_body(&mut self, data: Data<'_>) {
        // Nothing to do if we already have a body, or already generated an error. This includes
        // the case where the content type does not indicate a form, as the error is silent until a
        // body is explicitely requested.
        if let Ok(None) = self.body {
            let data = data.open(2.mebibytes()).into_string().await;
            match serde_urlencoded::from_str(&data.unwrap()) {
                Ok(query) => self.body = Ok(Some(query)),
                Err(_) => self.body = Err(WebError::Encoding),
            }
        }
    }
}

impl<'r> OAuthResponse<'r> {
    /// Create a new `OAuthResponse<'r>`
    #[must_use]
    pub fn new() -> Self {
        OAuthResponse::default()
    }

    /// Create a new `OAuthResponse<'r>` from an existing `rocket::Response<'r>`
    pub fn from_response(response: Response<'r>) -> Self {
        OAuthResponse(response)
    }
}

impl<'r> WebRequest for OAuthRequest<'r> {
    type Error = WebError;
    type Response = OAuthResponse<'r>;

    fn query(&mut self) -> Result<Cow<dyn QueryParameter + 'static>, Self::Error> {
        match self.query.as_ref() {
            Ok(query) => Ok(Cow::Borrowed(query as &dyn QueryParameter)),
            Err(err) => Err(*err),
        }
    }

    fn urlbody(&mut self) -> Result<Cow<dyn QueryParameter + 'static>, Self::Error> {
        match self.body.as_ref() {
            Ok(None) => Err(WebError::BodyNeeded),
            Ok(Some(body)) => Ok(Cow::Borrowed(body as &dyn QueryParameter)),
            Err(err) => Err(*err),
        }
    }

    fn authheader(&mut self) -> Result<Option<Cow<str>>, Self::Error> {
        Ok(self.auth.as_deref().map(Cow::Borrowed))
    }
}

impl<'r> WebResponse for OAuthResponse<'r> {
    type Error = WebError;

    fn ok(&mut self) -> Result<(), Self::Error> {
        self.0.set_status(Status::Ok);
        Ok(())
    }

    fn redirect(&mut self, url: Url) -> Result<(), Self::Error> {
        self.0.set_status(Status::Found);
        self.0.set_header(Header::new("location", url.to_string()));
        Ok(())
    }

    fn client_error(&mut self) -> Result<(), Self::Error> {
        self.0.set_status(Status::BadRequest);
        Ok(())
    }

    fn unauthorized(&mut self, kind: &str) -> Result<(), Self::Error> {
        self.0.set_status(Status::Unauthorized);
        self.0.set_raw_header("WWW-Authenticate", kind.to_owned());
        Ok(())
    }

    fn body_text(&mut self, text: &str) -> Result<(), Self::Error> {
        self.0
            .set_sized_body(text.len(), Cursor::new(text.to_owned()));
        self.0.set_header(ContentType::Plain);
        Ok(())
    }

    fn body_json(&mut self, data: &str) -> Result<(), Self::Error> {
        self.0
            .set_sized_body(data.len(), Cursor::new(data.to_owned()));
        self.0.set_header(ContentType::JSON);
        Ok(())
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OAuthRequest<'r> {
    type Error = NoError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, (Status, Self::Error), ()> {
        Outcome::Success(Self::new(request))
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for OAuthResponse<'o> {
    fn respond_to(self, _: &Request) -> response::Result<'o> {
        Ok(self.0)
    }
}

impl<'r> Responder<'r, 'static> for WebError {
    fn respond_to(self, _: &Request) -> response::Result<'static> {
        match self {
            WebError::Encoding | WebError::NotAForm => Err(Status::BadRequest),
            WebError::BodyNeeded => Err(Status::InternalServerError),
        }
    }
}

impl<'r> From<Response<'r>> for OAuthResponse<'r> {
    fn from(r: Response<'r>) -> Self {
        OAuthResponse::from_response(r)
    }
}

impl<'r> From<OAuthResponse<'r>> for Response<'r> {
    fn from(o: OAuthResponse<'r>) -> Self {
        o.0
    }
}
