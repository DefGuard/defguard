use axum::{
    extract::{FromRequestParts, State},
    http::{StatusCode, header::HeaderValue, request::Parts},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use defguard_common::db::models::{Session, SessionState, Settings, user::User};
use reqwest::Url;

use super::{ApiErrorResponse, SESSION_COOKIE_NAME};
use crate::{appstate::AppState, error::WebError};

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

pub(crate) struct ForwardAuthHeaders {
    pub forwarded_host: Option<String>,
    pub forwarded_proto: Option<String>,
    pub forwarded_uri: Option<String>,
}

impl<S> FromRequestParts<S> for ForwardAuthHeaders
where
    S: Send + Sync,
{
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

/// Authorize a request forwarded by a reverse proxy.
///
/// Meant to be used as a forward-auth endpoint (e.g. Traefik `forwardAuth`). The original
/// request URL is read from the `X-Forwarded-*` headers.
#[utoipa::path(
    get,
    path = "/api/v1/forward_auth",
    tag = "system",
    responses(
        (status = 200, description = "Request is authorized."),
        (status = 302, description = "User is not authenticated, redirect to the login page."),
        (status = 401, description = "Request cannot be authorized.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to authorize the request.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
)]
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
                // FIXME: This duplicates the MFA and is_active checks from
                // SessionInfo (auth/mod.rs). Extract a shared session validation
                // helper so these checks cannot drift apart again.
                match User::find_by_id(&appstate.pool, session.user_id).await {
                    Ok(Some(user)) => {
                        if user.mfa_enabled && session.state != SessionState::MultiFactorVerified {
                            info!(
                                "Session {} for user id {} MFA not completed, redirecting to login",
                                session.id, session.user_id
                            );
                        } else if !user.is_active {
                            info!(
                                "User id {} is disabled, redirecting to login",
                                session.user_id
                            );
                        } else {
                            return Ok(ForwardAuthResponse::Accept);
                        }
                    }
                    Ok(None) => {
                        info!(
                            "User id {} not found for session {}, redirecting to login",
                            session.user_id, session.id
                        );
                    }
                    Err(err) => {
                        warn!(
                            "Failed to load user id {} for session {}: {err}, redirecting to login",
                            session.user_id, session.id
                        );
                    }
                }
            }
        }
    }
    // If no session cookie provided redirect to login
    info!("Valid session not found, redirecting to login page");
    login_redirect(headers)
}

fn login_redirect(headers: ForwardAuthHeaders) -> Result<ForwardAuthResponse, WebError> {
    let server_url = Settings::url()?;
    let mut location = server_url.join("/auth/login").map_err(|err| {
        error!("Failed to prepare redirect URL: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;
    if let Some(host) = headers.forwarded_host
        && host != server_url.as_str()
    {
        let mut referral_url = Url::parse(format!("http://{host}").as_str()).map_err(|_| {
            error!("Failed to parse forwarded host as URL: {host}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
        if let Some(proto) = headers.forwarded_proto
            && let Err(_e) = referral_url.set_scheme(&proto)
        {
            warn!("Failed setting protocol for referral url to {proto}");
        }
        if let Some(uri) = headers.forwarded_uri {
            referral_url.set_path(&uri);
        }
        location.set_query(Some(format!("r={referral_url}").as_str()));
    }
    debug!("Redirecting to login page at {location}");
    Ok(ForwardAuthResponse::Redirect(location.to_string()))
}
