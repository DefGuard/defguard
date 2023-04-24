use crate::{
    appstate::AppState,
    db::{OAuth2AuthorizedApp, OAuth2Token, Session, SessionState, User},
    error::OriWebError,
};
use jsonwebtoken::{
    decode, encode, errors::Error as JWTError, DecodingKey, EncodingKey, Header, Validation,
};
use rocket::{
    http::{Cookie, Status},
    outcome::try_outcome,
    request::{FromRequest, Outcome, Request},
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    time::{Duration, SystemTime},
};

pub static JWT_ISSUER: &str = "DefGuard";
pub static AUTH_SECRET_ENV: &str = "DEFGUARD_AUTH_SECRET";
pub static GATEWAY_SECRET_ENV: &str = "DEFGUARD_GATEWAY_SECRET";
pub static YUBIBRIDGE_SECRET_ENV: &str = "DEFGUARD_YUBIBRIDGE_SECRET";
pub const SESSION_TIMEOUT: u64 = 3600 * 24 * 7;
pub const TOTP_CODE_VALIDITY_PERIOD: u64 = 30;

#[derive(Clone, Default)]
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
            secret: Self::get_secret(&claims_type),
            iss: JWT_ISSUER.to_string(),
            sub,
            client_id,
            exp,
            nbf,
        }
    }

    fn get_secret(claims_type: &ClaimsType) -> String {
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
        let secret = Self::get_secret(&claims_type);
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = OriWebError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(state) = request.rocket().state::<AppState>() {
            let cookies = request.cookies();
            if let Some(session_cookie) = cookies.get("defguard_session") {
                return {
                    match Session::find_by_id(&state.pool, session_cookie.value()).await {
                        Ok(Some(session)) => {
                            if session.expired() {
                                let _result = session.delete(&state.pool).await;
                                cookies.remove(Cookie::named("defguard_session"));
                                Outcome::Failure((
                                    Status::Unauthorized,
                                    OriWebError::Authorization("Session expired".into()),
                                ))
                            } else {
                                Outcome::Success(session)
                            }
                        }
                        Ok(None) => Outcome::Failure((
                            Status::Unauthorized,
                            OriWebError::Authorization("Session not found".into()),
                        )),
                        Err(err) => Outcome::Failure((Status::InternalServerError, err.into())),
                    }
                };
            }
        }
        Outcome::Failure((
            Status::Unauthorized,
            OriWebError::Authorization("Session is required".into()),
        ))
    }
}

// Extension of base user session including user data fetched from DB
// This represents a session for a user who completed the login process (including MFA if enabled)
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

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionInfo {
    type Error = OriWebError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(state) = request.rocket().state::<AppState>() {
            let session = try_outcome!(request.guard::<Session>().await);
            let user = User::find_by_id(&state.pool, session.user_id).await;
            if let Ok(Some(user)) = &user {
                if user.mfa_enabled && session.state != SessionState::MultiFactorVerified {
                    return Outcome::Failure((
                        Status::Unauthorized,
                        OriWebError::Authorization("MFA not verified".into()),
                    ));
                }
            }

            return match user {
                Ok(Some(user)) => {
                    let is_admin = match user.member_of(&state.pool).await {
                        Ok(groups) => groups.contains(&state.config.admin_groupname),
                        _ => false,
                    };
                    Outcome::Success(SessionInfo::new(session, user, is_admin))
                }
                _ => Outcome::Failure((
                    Status::Unauthorized,
                    OriWebError::Authorization("User not found".into()),
                )),
            };
        }

        Outcome::Failure((
            Status::Unauthorized,
            OriWebError::Authorization("Invalid session".into()),
        ))
    }
}

pub struct AdminRole;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminRole {
    type Error = OriWebError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let session_info = try_outcome!(request.guard::<SessionInfo>().await);
        if session_info.is_admin {
            Outcome::Success(AdminRole {})
        } else {
            Outcome::Failure((
                Status::Forbidden,
                OriWebError::Forbidden("access denied".into()),
            ))
        }
    }
}

// User authenticated by a valid access token
pub struct AccessUserInfo(pub(crate) User);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AccessUserInfo {
    type Error = OriWebError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(state) = request.rocket().state::<AppState>() {
            if let Some(token) = request
                .headers()
                .get_one("Authorization")
                .and_then(|value| {
                    if value.to_lowercase().starts_with("bearer ") {
                        value.get(7..)
                    } else {
                        None
                    }
                })
            {
                // TODO: #[cfg(feature = "openid")]
                match OAuth2Token::find_access_token(&state.pool, token).await {
                    Ok(Some(oauth2token)) => {
                        match OAuth2AuthorizedApp::find_by_id(
                            &state.pool,
                            oauth2token.oauth2authorizedapp_id,
                        )
                        .await
                        {
                            Ok(Some(authorized_app)) => {
                                if let Ok(Some(user)) =
                                    User::find_by_id(&state.pool, authorized_app.user_id).await
                                {
                                    return Outcome::Success(AccessUserInfo(user));
                                }
                            }
                            Ok(None) => {
                                return Outcome::Failure((
                                    Status::Unauthorized,
                                    OriWebError::Authorization("Authorized app not found".into()),
                                ));
                            }

                            Err(err) => {
                                return Outcome::Failure((Status::InternalServerError, err.into()));
                            }
                        }
                    }
                    Ok(None) => {
                        return Outcome::Failure((
                            Status::Unauthorized,
                            OriWebError::Authorization("Invalid token".into()),
                        ));
                    }
                    Err(err) => {
                        return Outcome::Failure((Status::InternalServerError, err.into()));
                    }
                }
            }
        }

        Outcome::Failure((
            Status::Unauthorized,
            OriWebError::Authorization("Invalid session".into()),
        ))
    }
}
