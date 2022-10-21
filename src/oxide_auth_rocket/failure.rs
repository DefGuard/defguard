use self::{
    Kind::{OAuth, Web},
    OAuthError::{BadRequest, DenySilently, PrimitiveError},
};
use super::WebError;
use oxide_auth::endpoint::OAuthError;
use rocket::{
    http::Status,
    response::{Responder, Result},
    Request,
};

/// Failed handling of an oauth request, providing a response.
///
/// The error responses generated by this type are *not* part of the stable interface. To create
/// stable error pages or to build more meaningful errors, either destructure this using the
/// `oauth` and `web` method or avoid turning errors into this type by providing a custom error
/// representation.
#[derive(Clone, Debug)]
pub struct OAuthFailure {
    inner: Kind,
}

// impl OAuthFailure {
//     /// Get the `OAuthError` causing this failure.
//     pub fn oauth(&self) -> Option<OAuthError> {
//         match &self.inner {
//             OAuth(err) => Some(*err),
//             _ => None,
//         }
//     }

//     /// Get the `WebError` causing this failure.
//     pub fn web(&self) -> Option<WebError> {
//         match &self.inner {
//             Web(err) => Some(*err),
//             _ => None,
//         }
//     }
// }

#[derive(Clone, Debug)]
enum Kind {
    Web(WebError),
    OAuth(OAuthError),
}

impl<'r> Responder<'r, 'static> for OAuthFailure {
    fn respond_to(self, _: &Request) -> Result<'static> {
        match self.inner {
            Web(_) | OAuth(DenySilently | BadRequest) => Err(Status::BadRequest),
            OAuth(PrimitiveError) => Err(Status::InternalServerError),
        }
    }
}

impl From<OAuthError> for OAuthFailure {
    fn from(err: OAuthError) -> Self {
        OAuthFailure { inner: OAuth(err) }
    }
}

impl From<WebError> for OAuthFailure {
    fn from(err: WebError) -> Self {
        OAuthFailure { inner: Web(err) }
    }
}
