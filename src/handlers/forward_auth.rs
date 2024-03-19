use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{header::HeaderValue, request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use reqwest::Url;

use super::SESSION_COOKIE_NAME;
use crate::{appstate::AppState, db::Session, error::WebError, server_config};

// Header names
static FORWARDED_HOST: &str = "x-forwarded-host";
static FORWARDED_PROTO: &str = "x-forwarded-proto";
static FORWARDED_URI: &str = "x-forwarded-uri";

pub enum ForwardAuthResponse {
    Accept,
    Redirect(String),
}

impl IntoResponse for ForwardAuthResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Accept => ().into_response(),
            Self::Redirect(location) => Redirect::temporary(&location).into_response(),
        }
    }
}

pub struct ForwardAuthHeaders {
    pub forwarded_host: Option<String>,
    pub forwarded_proto: Option<String>,
    pub forwarded_uri: Option<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for ForwardAuthHeaders {
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        fn header_to_string(header: &HeaderValue) -> Option<String> {
            header.to_str().ok().map(String::from)
        }

        let forwarded_host = parts.headers.get(FORWARDED_HOST).and_then(header_to_string);
        let forwarded_proto = parts
            .headers
            .get(FORWARDED_PROTO)
            .and_then(header_to_string);
        let forwarded_uri = parts.headers.get(FORWARDED_URI).and_then(header_to_string);

        Ok(Self {
            forwarded_host,
            forwarded_proto,
            forwarded_uri,
        })
    }
}

pub async fn forward_auth(
    State(appstate): State<AppState>,
    cookies: CookieJar,
    headers: ForwardAuthHeaders,
) -> Result<ForwardAuthResponse, WebError> {
    // check if session cookie is present
    if let Some(session_cookie) = cookies.get(SESSION_COOKIE_NAME) {
        // check if session is found in DB
        if let Ok(Some(session)) = Session::find_by_id(&appstate.pool, session_cookie.value()).await
        {
            // check if session is expired
            if session.expired() {
                info!(
                    "Session {} for user id {} has expired, redirecting to login",
                    session.id, session.user_id
                );
                let _result = session.delete(&appstate.pool).await;
            } else {
                // If session is verified return 200 response
                return Ok(ForwardAuthResponse::Accept);
            }
        }
    }
    // If no session cookie provided redirect to login
    info!("Valid session not found, redirecting to login page");
    login_redirect(headers).await
}

async fn login_redirect(headers: ForwardAuthHeaders) -> Result<ForwardAuthResponse, WebError> {
    let server_url = &server_config().url; // prepare redirect URL for login page
    let mut location = server_url.join("/auth/login").map_err(|err| {
        error!("Failed to prepare redirect URL: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    if let Some(host) = headers.forwarded_host {
        if host != server_url.as_str() {
            let mut referral_url = Url::parse(format!("http://{host}").as_str()).map_err(|_| {
                error!("Failed to parse forwarded host as URL: {host}");
                WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
            })?;
            if let Some(proto) = headers.forwarded_proto {
                if let Err(_e) = referral_url.set_scheme(&proto) {
                    warn!("Failed setting protocol for referral url to {proto}");
                }
            }
            if let Some(uri) = headers.forwarded_uri {
                referral_url.set_path(&uri);
            }
            location.set_query(Some(format!("r={referral_url}").as_str()));
        }
    }
    debug!("Redirecting to login page at {location}");
    Ok(ForwardAuthResponse::Redirect(location.to_string()))
}
