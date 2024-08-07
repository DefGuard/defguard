use crate::enterprise::license::{validate_license, License};

pub mod openid_login;
pub mod openid_providers;

use std::{
    env,
    time::{Duration, SystemTime},
};

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::cookie::CookieJar;
use jsonwebtoken::{
    decode, encode, errors::Error as JWTError, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};

use crate::{
    appstate::AppState,
    db::{Group, OAuth2AuthorizedApp, OAuth2Token, Session, SessionState, User},
    error::WebError,
    handlers::SESSION_COOKIE_NAME,
    server_config,
};

pub struct LicenseInfo {
    pub valid: bool,
}

#[async_trait]
impl<S> FromRequestParts<S> for LicenseInfo
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);

        let license = appstate
            .license
            .lock()
            .expect("Failed to acquire lock on the license.");

        match validate_license((*license).as_ref()) {
            // Useless struct, but may come in handy later
            Ok(_) => Ok(LicenseInfo { valid: true }),
            Err(e) => Err(WebError::Forbidden(e.to_string())),
        }
    }
}
