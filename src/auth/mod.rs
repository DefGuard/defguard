pub mod failed_login;

use std::{
    env,
    time::{Duration, SystemTime},
};

use axum::{
    async_trait,
    extract::{FromRef, FromRequest},
    http::Request,
};
use jsonwebtoken::{
    decode, encode, errors::Error as JWTError, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

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
impl<S, B> FromRequest<S, B> for Session
where
    S: Send + Sync,
    B: Send + 'static,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request(request: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        // let cookies =

        //         if let Some(state) = request.rocket().state::<AppState>() {
        //             let cookies = request.cookies();
        //             if let Some(session_cookie) = cookies.get("defguard_session") {
        //                 return {
        //                     match Session::find_by_id(&state.pool, session_cookie.value()).await {
        //                         Ok(Some(session)) => {
        //                             if session.expired() {
        //                                 let _result = session.delete(&state.pool).await;
        //                                 cookies.remove(Cookie::named("defguard_session"));
        //                                 Outcome::Failure((
        //                                     Status::Unauthorized,
        //                                     WebError::Authorization("Session expired".into()),
        //                                 ))
        //                             } else {
        //                                 Outcome::Success(session)
        //                             }
        //                         }
        //                         Ok(None) => Outcome::Failure((
        //                             Status::Unauthorized,
        //                             WebError::Authorization("Session not found".into()),
        //                         )),
        //                         Err(err) => Outcome::Failure((StatusCode::INTERNAL_SERVER_ERROR, err.into())),
        //                     }
        //                 };
        //             }
        //         }
        //         Outcome::Failure((
        //             Status::Unauthorized,
        //             WebError::Authorization("Session is required".into()),
        //         ))
        // FIXME: dummy error
        Err(WebError::Authorization("MFA not verified".into()))
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

#[async_trait]
impl<S, B> FromRequest<S, B> for SessionInfo
where
    S: Send + Sync,
    B: Send + 'static,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request(request: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        //     let session = try_outcome!(request.guard::<Session>().await);
        //     let user = User::find_by_id(&state.pool, session.user_id).await;
        //     if let Ok(Some(user)) = &user {
        //         if user.mfa_enabled && session.state != SessionState::MultiFactorVerified {
        //             return Outcome::Failure((
        //                 Status::Unauthorized,
        //                 WebError::Authorization("MFA not verified".into()),
        //             ));
        //         }
        //     }

        //     return match user {
        //         Ok(Some(user)) => {
        //             let is_admin = match user.member_of(&state.pool).await {
        //                 Ok(groups) => groups.contains(&state.config.admin_groupname),
        //                 _ => false,
        //             };
        //             Outcome::Success(SessionInfo::new(session, user, is_admin))
        //         }
        //         _ => Outcome::Failure((
        //             Status::Unauthorized,
        //             WebError::Authorization("User not found".into()),
        //         )),
        //     };
        // }

        // Outcome::Failure((
        //     Status::Unauthorized,
        //     WebError::Authorization("Invalid session".into()),
        // ))

        // FIXME: dummy error
        Err(WebError::Authorization("MFA not verified".into()))
    }
}

pub struct AdminRole;

// #[rocket::async_trait]
// impl<'r> FromRequest<'r> for AdminRole {
//     type Error = WebError;

//     async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
//         let session_info = try_outcome!(request.guard::<SessionInfo>().await);
//         if session_info.is_admin {
//             Outcome::Success(AdminRole {})
//         } else {
//             Outcome::Failure((
//                 Status::Forbidden,
//                 WebError::Forbidden("access denied".into()),
//             ))
//         }
//     }
// }

// User authenticated by a valid access token
pub struct AccessUserInfo(pub(crate) User);

// #[rocket::async_trait]
// impl<'r> FromRequest<'r> for AccessUserInfo {
//     type Error = WebError;

//     async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
//         if let Some(state) = request.rocket().state::<AppState>() {
//             if let Some(token) = request
//                 .headers()
//                 .get_one("Authorization")
//                 .and_then(|value| {
//                     if value.to_lowercase().starts_with("bearer ") {
//                         value.get(7..)
//                     } else {
//                         None
//                     }
//                 })
//             {
//                 // TODO: #[cfg(feature = "openid")]
//                 match OAuth2Token::find_access_token(&state.pool, token).await {
//                     Ok(Some(oauth2token)) => {
//                         match OAuth2AuthorizedApp::find_by_id(
//                             &state.pool,
//                             oauth2token.oauth2authorizedapp_id,
//                         )
//                         .await
//                         {
//                             Ok(Some(authorized_app)) => {
//                                 if let Ok(Some(user)) =
//                                     User::find_by_id(&state.pool, authorized_app.user_id).await
//                                 {
//                                     return Outcome::Success(AccessUserInfo(user));
//                                 }
//                             }
//                             Ok(None) => {
//                                 return Outcome::Failure((
//                                     Status::Unauthorized,
//                                     WebError::Authorization("Authorized app not found".into()),
//                                 ));
//                             }

//                             Err(err) => {
//                                 return Outcome::Failure((StatusCode::INTERNAL_SERVER_ERROR, err.into()));
//                             }
//                         }
//                     }
//                     Ok(None) => {
//                         return Outcome::Failure((
//                             Status::Unauthorized,
//                             WebError::Authorization("Invalid token".into()),
//                         ));
//                     }
//                     Err(err) => {
//                         return Outcome::Failure((StatusCode::INTERNAL_SERVER_ERROR, err.into()));
//                     }
//                 }
//             }
//         }

//         Outcome::Failure((
//             Status::Unauthorized,
//             WebError::Authorization("Invalid session".into()),
//         ))
//     }
// }
