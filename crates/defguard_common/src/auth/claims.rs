use std::{
    env,
    sync::OnceLock,
    time::{Duration, SystemTime},
};

use jsonwebtoken::{
    decode, encode, errors::Error as JWTError, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};

pub static JWT_ISSUER: &str = "DefGuard";
pub static AUTH_SECRET_ENV: &str = "DEFGUARD_AUTH_SECRET";
pub static GATEWAY_SECRET_ENV: &str = "DEFGUARD_GATEWAY_SECRET";
pub static YUBIBRIDGE_SECRET_ENV: &str = "DEFGUARD_YUBIBRIDGE_SECRET";

static JWT_SECRET_OVERRIDES: OnceLock<JwtSecretOverrides> = OnceLock::new();

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

#[derive(Clone, Debug, PartialEq, Eq)]
struct JwtSecretOverrides {
    auth: String,
    gateway: String,
    yubibridge: String,
}

impl JwtSecretOverrides {
    fn secret_for(&self, claims_type: ClaimsType) -> &str {
        match claims_type {
            ClaimsType::Auth | ClaimsType::DesktopClient => &self.auth,
            ClaimsType::Gateway => &self.gateway,
            ClaimsType::YubiBridge => &self.yubibridge,
        }
    }
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
        if let Some(secret_overrides) = JWT_SECRET_OVERRIDES.get() {
            return secret_overrides.secret_for(claims_type).to_string();
        }

        env::var(secret_env(claims_type)).unwrap_or_default()
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

fn secret_env(claims_type: ClaimsType) -> &'static str {
    match claims_type {
        ClaimsType::Auth | ClaimsType::DesktopClient => AUTH_SECRET_ENV,
        ClaimsType::Gateway => GATEWAY_SECRET_ENV,
        ClaimsType::YubiBridge => YUBIBRIDGE_SECRET_ENV,
    }
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod test_support {
    use super::{JwtSecretOverrides, JWT_SECRET_OVERRIDES};

    pub fn initialize_jwt_secret_overrides(
        auth_secret: impl Into<String>,
        gateway_secret: impl Into<String>,
        yubibridge_secret: impl Into<String>,
    ) {
        let secret_overrides = JwtSecretOverrides {
            auth: auth_secret.into(),
            gateway: gateway_secret.into(),
            yubibridge: yubibridge_secret.into(),
        };

        if let Err(secret_overrides) = JWT_SECRET_OVERRIDES.set(secret_overrides) {
            let existing_overrides = JWT_SECRET_OVERRIDES
                .get()
                .expect("JWT secret overrides should be initialized");
            assert_eq!(
                existing_overrides, &secret_overrides,
                "JWT secret overrides already initialized with different values"
            );
        }
    }
}
