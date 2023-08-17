use crate::{appstate::AppState, db::Session, error::OriWebError, SERVER_CONFIG};
use reqwest::Url;
use rocket::{
    http::{CookieJar, Status},
    request::{FromRequest, Outcome, Request},
    response::{self, Redirect, Responder},
    State,
};

/* Header Names */
static FORWARDED_HOST: &str = "X-Forwarded-Host";
static FORWARDED_PROTO: &str = "X-Forwarded-Proto";
static FORWARDED_URI: &str = "X-Forwarded-Uri";

pub enum ForwardAuthResponse {
    Accept,
    Redirect(String),
}

impl<'r> Responder<'r, 'static> for ForwardAuthResponse {
    fn respond_to(self, request: &'r Request) -> response::Result<'static> {
        match self {
            Self::Accept => ().respond_to(request),
            Self::Redirect(location) => Redirect::temporary(location).respond_to(request),
        }
    }
}

pub struct ForwardAuthHeaders {
    pub forwarded_host: Option<String>,
    pub forwarded_proto: Option<String>,
    pub forwarded_uri: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ForwardAuthHeaders {
    type Error = OriWebError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let headers = request.headers();
        let forwarded_host = headers.get_one(FORWARDED_HOST).map(String::from);
        let forwarded_proto = headers.get_one(FORWARDED_PROTO).map(String::from);
        let forwarded_uri = headers.get_one(FORWARDED_URI).map(String::from);

        Outcome::Success(Self {
            forwarded_host,
            forwarded_proto,
            forwarded_uri,
        })
    }
}

#[get("/forward_auth")]
pub async fn forward_auth(
    appstate: &State<AppState>,
    cookies: &CookieJar<'_>,
    headers: ForwardAuthHeaders,
) -> Result<ForwardAuthResponse, OriWebError> {
    // check if session cookie is present
    if let Some(session_cookie) = cookies.get("defguard_session") {
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

async fn login_redirect(headers: ForwardAuthHeaders) -> Result<ForwardAuthResponse, OriWebError> {
    let server_url = &SERVER_CONFIG
        .get()
        .ok_or(OriWebError::ServerConfigMissing)?
        .url;
    // prepare redirect URL for login page
    let mut location = server_url.join("/auth/login").map_err(|err| {
        error!("Failed to prepare redirect URL: {err}");
        OriWebError::Http(Status::InternalServerError)
    })?;
    if let Some(host) = headers.forwarded_host {
        if host != server_url.to_string() {
            let mut referral_url =
                Url::parse(format!("http://{}", host).as_str()).map_err(|_| {
                    error!("Failed to parse forwarded host as URL: {host}");
                    OriWebError::Http(Status::InternalServerError)
                })?;
            if let Some(proto) = headers.forwarded_proto {
                if let Err(_e) = referral_url.set_scheme(&proto) {
                    warn!("Failed setting protocol for referral url to {proto}");
                }
            }
            if let Some(uri) = headers.forwarded_uri {
                referral_url.set_path(&uri);
            }
            location.set_query(Some(format!("r={}", referral_url).as_str()));
        }
    }
    debug!("Redirecting to login page at {location}");
    Ok(ForwardAuthResponse::Redirect(location.to_string()))
}
