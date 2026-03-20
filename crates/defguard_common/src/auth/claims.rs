use std::{
    env,
    time::{Duration, SystemTime},
};

use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::Error as JWTError,
};
use serde::{Deserialize, Serialize};

pub static JWT_ISSUER: &str = "DefGuard";
pub static AUTH_SECRET_ENV: &str = "DEFGUARD_AUTH_SECRET";
pub static GATEWAY_SECRET_ENV: &str = "DEFGUARD_GATEWAY_SECRET";
pub static YUBIBRIDGE_SECRET_ENV: &str = "DEFGUARD_YUBIBRIDGE_SECRET";

#[derive(Clone, Copy, Default)]
pub enum ClaimsType {
    #[default]
    Auth,
    Gateway,
    YubiBridge,
    DesktopClient,
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
            ClaimsType::Auth | ClaimsType::DesktopClient => AUTH_SECRET_ENV,
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
