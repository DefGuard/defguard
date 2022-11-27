use crate::{
    appstate::AppState,
    db::{Session, SessionState, User},
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

#[derive(Deserialize, PartialEq, Serialize)]
pub enum ClaimRole {
    Admin,
}

#[derive(Clone)]
pub enum ClaimsType {
    Auth,
    Gateway,
    YubiBridge,
}

impl Default for ClaimsType {
    fn default() -> Self {
        ClaimsType::Auth
    }
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
    // TODO: aud https://openid.net/specs/openid-connect-core-1_0.html
    // client identifier
    pub client_id: String,
    // expiration time
    pub exp: u64,
    // not before
    pub nbf: u64,
    // roles
    #[serde(default)]
    pub roles: Vec<ClaimRole>,
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
            roles: Vec::new(),
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

    #[must_use]
    pub fn is_admin(&self) -> bool {
        self.roles.contains(&ClaimRole::Admin)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = OriWebError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(state) = request.rocket().state::<AppState>() {
            let cookies = request.cookies();
            if let Some(session_cookie) = cookies.get("session") {
                return {
                    match Session::find_by_id(&state.pool, session_cookie.value()).await {
                        Ok(Some(session)) => {
                            if session.expired() {
                                let _result = session.delete(&state.pool).await;
                                cookies.remove(Cookie::named("session"));
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

pub struct SessionInfo {
    pub user: User,
    pub is_admin: bool,
}

impl SessionInfo {
    #[must_use]
    pub fn new(user: User, is_admin: bool) -> Self {
        Self { user, is_admin }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SessionInfo {
    type Error = OriWebError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(state) = request.rocket().state::<AppState>() {
            let user = {
                if let Some(token) = request
                    .headers()
                    .get_one("Authorization")
                    .and_then(|value| {
                        if value.starts_with("Bearer ") {
                            value.get(7..)
                        } else {
                            None
                        }
                    })
                {
                    match Claims::from_jwt(ClaimsType::Auth, token) {
                        Ok(claims) => User::find_by_username(&state.pool, &claims.sub).await,
                        Err(_) => {
                            return Outcome::Failure((
                                Status::Unauthorized,
                                OriWebError::Authorization("Invalid token".into()),
                            ));
                        }
                    }
                } else {
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
                    user
                }
            };

            return match user {
                Ok(Some(user)) => {
                    let is_admin = match user.member_of(&state.pool).await {
                        Ok(groups) => groups.contains(&state.config.admin_groupname),
                        _ => false,
                    };
                    Outcome::Success(SessionInfo::new(user, is_admin))
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
