pub mod failed_login;

use std::{
    env,
    time::{Duration, SystemTime},
};

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use jsonwebtoken::{
    decode, encode, errors::Error as JWTError, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use tower_cookies::{Cookie, Cookies};

use crate::{
    appstate::AppState,
    db::{OAuth2AuthorizedApp, OAuth2Token, Session, SessionState, User},
    error::WebError,
};

pub static JWT_ISSUER: &str = "DefGuard";
pub static AUTH_SECRET_ENV: &str = "DEFGUARD_AUTH_SECRET";
pub static GATEWAY_SECRET_ENV: &str = "DEFGUARD_GATEWAY_SECRET";
pub static YUBIBRIDGE_SECRET_ENV: &str = "DEFGUARD_YUBIBRIDGE_SECRET";
pub const SESSION_TIMEOUT: u64 = 3600 * 24 * 7;
pub const TOTP_CODE_VALIDITY_PERIOD: u64 = 30;
pub const EMAIL_CODE_VALIDITY_PERIOD: u64 = 60 * 15;

#[derive(Clone, Copy, Default)]
pub enum ClaimsType {
    #[default]
    Auth,
    Gateway,
    YubiBridge,
}

/// Standard claims: https://www.iana.org/assignments/jwt/jwt.xhtml
#[derive(Deserialize, Serialize)]
pub struct Claims {
    #[serde(skip_serializing, skip_deserializing)]
    secret: String,
    // issuer
    pub iss: String,
    // subject
    pub sub: String,
    // client identifier
    pub client_id: String,
    // expiration time
    pub exp: u64,
    // not before
    pub nbf: u64,
}

impl Claims {
    #[must_use]
    pub fn new(claims_type: ClaimsType, sub: String, client_id: String, duration: u64) -> Self {
        let now = SystemTime::now();
        let exp = now
            .checked_add(Duration::from_secs(duration))
            .expect("valid time")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("valid timestamp")
            .as_secs();
        let nbf = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("valid timestamp")
            .as_secs();
        Self {
            secret: Self::get_secret(claims_type),
            iss: JWT_ISSUER.to_string(),
            sub,
            client_id,
            exp,
            nbf,
        }
    }

    fn get_secret(claims_type: ClaimsType) -> String {
        let env_var = match claims_type {
            ClaimsType::Auth => AUTH_SECRET_ENV,
            ClaimsType::Gateway => GATEWAY_SECRET_ENV,
            ClaimsType::YubiBridge => YUBIBRIDGE_SECRET_ENV,
        };
        env::var(env_var).unwrap_or_default()
    }

    /// Convert claims to JWT.
    pub fn to_jwt(&self) -> Result<String, JWTError> {
        encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    /// Verify JWT and, if successful, convert it to claims.
    pub fn from_jwt(claims_type: ClaimsType, token: &str) -> Result<Self, JWTError> {
        let secret = Self::get_secret(claims_type);
        let mut validation = Validation::default();
        validation.validate_nbf = true;
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_required_spec_claims(&["iss", "sub", "exp", "nbf"]);
        decode::<Self>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Session
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);
        if let Ok(cookies) = Cookies::from_request_parts(parts, state).await {
            if let Some(session_cookie) = cookies.get("defguard_session") {
                return {
                    match Session::find_by_id(&appstate.pool, session_cookie.value()).await {
                        Ok(Some(session)) => {
                            if session.expired() {
                                let _result = session.delete(&appstate.pool).await;
                                cookies.remove(Cookie::named("defguard_session"));
                                Err(WebError::Authorization("Session expired".into()))
                            } else {
                                Ok(session)
                            }
                        }
                        Ok(None) => Err(WebError::Authorization("Session not found".into())),
                        Err(err) => Err(err.into()),
                    }
                };
            }
        }
        Err(WebError::Authorization("Session is required".into()))
    }
}

// Extension of base user session that contains user data fetched from database.
// This represents a session for a user who completed the login process (including MFA, if enabled).
pub struct SessionInfo {
    pub session: Session,
    pub user: User,
    pub is_admin: bool,
}

impl SessionInfo {
    #[must_use]
    pub fn new(session: Session, user: User, is_admin: bool) -> Self {
        Self {
            session,
            user,
            is_admin,
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for SessionInfo
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);
        let session = Session::from_request_parts(parts, state).await?;
        let user = User::find_by_id(&appstate.pool, session.user_id).await;

        if let Ok(Some(user)) = user {
            if user.mfa_enabled && session.state != SessionState::MultiFactorVerified {
                return Err(WebError::Authorization("MFA not verified".into()));
            }
            let is_admin = match user.member_of(&appstate.pool).await {
                Ok(groups) => groups.contains(&appstate.config.admin_groupname),
                _ => false,
            };
            Ok(SessionInfo::new(session, user, is_admin))
        } else {
            Err(WebError::Authorization("User not found".into()))
        }
    }
}

pub struct AdminRole;

#[async_trait]
impl<S> FromRequestParts<S> for AdminRole
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session_info = SessionInfo::from_request_parts(parts, state).await?;
        if session_info.is_admin {
            Ok(AdminRole {})
        } else {
            Err(WebError::Forbidden("access denied".into()))
        }
    }
}

// User authenticated by a valid access token
pub struct AccessUserInfo(pub(crate) User);

#[async_trait]
impl<S> FromRequestParts<S> for AccessUserInfo
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);
        if let Some(token) = parts.headers.get("Authorization").and_then(|value| {
            if let Ok(value) = value.to_str() {
                if value.to_lowercase().starts_with("bearer ") {
                    value.get(7..)
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            // TODO: #[cfg(feature = "openid")]
            match OAuth2Token::find_access_token(&appstate.pool, token).await {
                Ok(Some(oauth2token)) => {
                    match OAuth2AuthorizedApp::find_by_id(
                        &appstate.pool,
                        oauth2token.oauth2authorizedapp_id,
                    )
                    .await
                    {
                        Ok(Some(authorized_app)) => {
                            if let Ok(Some(user)) =
                                User::find_by_id(&appstate.pool, authorized_app.user_id).await
                            {
                                return Ok(AccessUserInfo(user));
                            }
                        }
                        Ok(None) => {
                            return Err(WebError::Authorization("Authorized app not found".into()));
                        }

                        Err(err) => {
                            return Err(err.into());
                        }
                    }
                }
                Ok(None) => {
                    return Err(WebError::Authorization("Invalid token".into()));
                }
                Err(err) => {
                    return Err(err.into());
                }
            }
        }

        Err(WebError::Authorization("Invalid session".into()))
    }
}
