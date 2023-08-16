use crate::{appstate::AppState, db::Session, error::OriWebError};
use reqwest::Url;
use rocket::{
    http::{CookieJar, Status},
    request::{FromRequest, Outcome, Request},
    response::Redirect,
    State,
};

/* Header Names */
static FORWARDED_HOST: &str = "X-Forwarded-Host";
static FORWARDED_PROTO: &str = "X-Forwarded-Proto";
static FORWARDED_URI: &str = "X-Forwarded-Uri";

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
) -> Result<Redirect, OriWebError> {
    println!("{cookies:?}");
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
                unimplemented!()
            }
        }
    }
    // If no session cookie provided redirect to login
    info!("Valid session not found, redirecting to login page");
    // prepare redirect URL for login page
    let mut location = appstate.config.url.join("/auth/login").map_err(|err| {
        error!("Failed to prepare redirect URL: {err}");
        OriWebError::Http(Status::InternalServerError)
    })?;
    if let Some(host) = headers.forwarded_host {
        if host != appstate.config.url.to_string() {
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
    Ok(Redirect::temporary(location.to_string()))
}

async fn login_redirect(
    appstate: &State<AppState>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, OriWebError> {
    // let base_url = appstate.config.url.join("api/v1/oauth/authorize").unwrap();
    // let expires = OffsetDateTime::now_utc()
    //     .checked_add(TimeDuration::minutes(10))
    //     .ok_or(OriWebError::Http(Status::InternalServerError))?;
    // let cookie = Cookie::build(
    //     "known_sign_in",
    //     format!(
    //         "{}?{}",
    //         base_url,
    //         serde_urlencoded::to_string(data).unwrap()
    //     ),
    // )
    // .secure(true)
    // .same_site(SameSite::Strict)
    // .http_only(true)
    // .expires(expires)
    // .finish();
    // cookies.add_private(cookie);
    Ok(Redirect::found("/login".to_string()))
}
