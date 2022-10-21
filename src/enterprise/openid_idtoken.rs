use crate::{
    auth::{JWT_ISSUER, SESSION_TIMEOUT},
    db::User,
};
use jsonwebtoken::{encode, errors::Error as JWTError, EncodingKey, Header};
use std::time::{Duration, SystemTime};

// ID Token claims: https://openid.net/specs/openid-connect-core-1_0.html#IDToken
#[derive(Deserialize, Serialize)]
pub struct IDTokenClaims {
    pub iss: String,
    // User id
    pub sub: String,
    // Client id
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub phone: Option<String>,
    pub phone_verified: Option<bool>,
    pub nonce: Option<String>,
}

// Supported user claims
pub struct UserClaims {
    pub username: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub phone: Option<String>,
    pub phone_verified: Option<bool>,
}

impl IDTokenClaims {
    #[must_use]
    pub fn new(sub: String, aud: String, nonce: Option<String>, user_claims: UserClaims) -> Self {
        let now = SystemTime::now();
        let exp = now
            .checked_add(Duration::from_secs(SESSION_TIMEOUT))
            .expect("valid time")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("valid timestamp")
            .as_secs();
        let iat = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("valid timestamp")
            .as_secs();
        Self {
            iss: JWT_ISSUER.to_owned(),
            sub,
            aud,
            exp,
            iat,
            nonce,
            given_name: user_claims.given_name,
            family_name: user_claims.family_name,
            email: user_claims.email,
            email_verified: user_claims.email_verified,
            phone: user_claims.phone,
            phone_verified: user_claims.phone_verified,
        }
    }

    /// Convert claims to JWT.
    pub fn to_jwt(&self, client_secret: &str) -> Result<String, JWTError> {
        encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(client_secret.as_bytes()),
        )
    }
    // Get user data based on scopes: https://openid.net/specs/openid-connect-core-1_0.html#ScopeClaims
    // FIXME: must be better way to do this
    pub fn get_user_claims(user: User, scopes: &str) -> UserClaims {
        let mut user_claims = UserClaims {
            username: user.username,
            given_name: Some(user.first_name),
            family_name: Some(user.last_name),
            email: Some(user.email.clone()),
            email_verified: Some(true),
            phone: user.phone.clone(),
            phone_verified: Some(true),
        };
        if user.email.is_empty() {
            user_claims.email_verified = Some(false);
        }
        if user.phone.is_none() {
            user_claims.phone_verified = Some(false);
        }
        if !scopes.contains("email") {
            user_claims.email = None;
            user_claims.email_verified = None;
        }
        if !scopes.contains("profile") {
            user_claims.given_name = None;
            user_claims.family_name = None;
        }
        if !scopes.contains("phone") {
            user_claims.phone = None;
            user_claims.phone_verified = None;
        }
        user_claims
    }
}
