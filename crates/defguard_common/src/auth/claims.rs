use std::time::{Duration, SystemTime};

use jsonwebtoken::{
    DecodingKey, EncodingKey, Header, Validation, decode, encode, errors::Error as JWTError,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::db::models::{Settings, settings::SettingsInitializationError};

pub static JWT_ISSUER: &str = "DefGuard";

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ClaimsType {
    #[default]
    Auth,
    Gateway,
    YubiBridge,
    DesktopClient,
}

#[derive(Debug, Error)]
pub enum ClaimsError {
    #[error("Failed to read JWT signing key from settings: {0}")]
    Settings(#[from] SettingsInitializationError),
    #[error("JWT processing failed: {0}")]
    Jwt(#[from] JWTError),
    #[error("JWT claims type mismatch: expected {expected:?}, got {actual:?}")]
    UnexpectedClaimsType {
        expected: ClaimsType,
        actual: ClaimsType,
    },
}

/// Standard claims: https://www.iana.org/assignments/jwt/jwt.xhtml
#[derive(Deserialize, Serialize)]
pub struct Claims {
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
    pub claims_type: ClaimsType,
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
            iss: JWT_ISSUER.to_string(),
            sub,
            client_id,
            exp,
            nbf,
            claims_type,
        }
    }

    fn encoding_key() -> Result<EncodingKey, ClaimsError> {
        let settings = Settings::get_current_settings();
        Ok(EncodingKey::from_secret(
            settings.secret_key_required()?.as_bytes(),
        ))
    }

    fn decoding_key() -> Result<DecodingKey, ClaimsError> {
        let settings = Settings::get_current_settings();
        Ok(DecodingKey::from_secret(
            settings.secret_key_required()?.as_bytes(),
        ))
    }

    /// Convert claims to JWT.
    pub fn to_jwt(&self) -> Result<String, ClaimsError> {
        let encoding_key = Self::encoding_key()?;

        encode(&Header::default(), self, &encoding_key).map_err(ClaimsError::from)
    }

    /// Verify JWT and, if successful, convert it to claims.
    pub fn from_jwt(expected_claims_type: ClaimsType, token: &str) -> Result<Self, ClaimsError> {
        let decoding_key = Self::decoding_key()?;
        let mut validation = Validation::default();
        validation.validate_nbf = true;
        validation.set_issuer(&[JWT_ISSUER]);
        validation.set_required_spec_claims(&["iss", "sub", "exp", "nbf"]);
        let claims = decode::<Self>(token, &decoding_key, &validation).map(|data| data.claims)?;

        if claims.claims_type != expected_claims_type {
            return Err(ClaimsError::UnexpectedClaimsType {
                expected: expected_claims_type,
                actual: claims.claims_type,
            });
        }

        Ok(claims)
    }
}
