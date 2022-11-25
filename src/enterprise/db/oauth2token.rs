use crate::{
    auth::{Claims, ClaimsType, SESSION_TIMEOUT},
    db::DbPool,
    random::gen_alphanumeric,
};
use chrono::{Duration, TimeZone, Utc};
use oxide_auth::primitives::{
    generator::{RandomGenerator, TagGrant},
    grant::{Extensions, Grant},
    issuer::{IssuedToken, RefreshedToken, TokenType},
};
use sqlx::{query, query_as, Error as SqlxError};

pub struct OAuth2Token {
    pub access_token: String,
    pub refresh_token: String,
    pub redirect_uri: String,
    pub scope: String,
    pub expires_in: i64,
}

impl OAuth2Token {
    #[must_use]
    pub fn new(redirect_uri: String, scope: String) -> Self {
        let expiration = Utc::now() + Duration::seconds(SESSION_TIMEOUT as i64);
        Self {
            access_token: gen_alphanumeric(24),
            refresh_token: gen_alphanumeric(24),
            redirect_uri,
            scope,
            expires_in: expiration.timestamp(),
        }
    }

    /// Generate new access token, scratching the old one. Changes are reflected in the database.
    pub async fn refresh_and_save(&mut self, pool: &DbPool) -> Result<(), SqlxError> {
        let new_access_token = gen_alphanumeric(24);
        let new_refresh_token = gen_alphanumeric(24);
        let expiration = Utc::now() + Duration::seconds(SESSION_TIMEOUT as i64);
        self.expires_in = expiration.timestamp();

        query!(
            "UPDATE oauth2token SET access_token = $2, refresh_token = $3, expires_in = $4 \
            WHERE access_token = $1",
            self.access_token,
            new_access_token,
            new_refresh_token,
            self.expires_in
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Check if token has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_in < Utc::now().timestamp()
    }

    /// Store data in the database.
    pub async fn save(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "INSERT INTO oauth2token (access_token, refresh_token, redirect_uri, scope, expires_in) \
            VALUES ($1, $2, $3, $4, $5)",
            self.access_token,
            self.refresh_token,
            self.redirect_uri,
            self.scope,
            self.expires_in)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete token from the database.
    pub async fn delete(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "DELETE FROM oauth2token WHERE access_token = $1 AND refresh_token = $2",
            self.access_token,
            self.refresh_token
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find by access token.
    pub async fn find_access_token(pool: &DbPool, access_token: &str) -> Option<Self> {
        match query_as!(
            Self,
            "SELECT access_token, refresh_token, redirect_uri, scope, expires_in \
            FROM oauth2token WHERE access_token = $1",
            access_token
        )
        .fetch_one(pool)
        .await
        {
            Ok(token) => {
                if token.is_expired() {
                    let _result = token.delete(pool).await;
                    None
                } else {
                    Some(token)
                }
            }
            Err(_) => None,
        }
    }

    /// Find by refresh token.
    pub async fn find_refresh_token(pool: &DbPool, refresh_token: &str) -> Option<Self> {
        match query_as!(
            Self,
            "SELECT access_token, refresh_token, redirect_uri, scope, expires_in \
            FROM oauth2token WHERE refresh_token = $1",
            refresh_token
        )
        .fetch_one(pool)
        .await
        {
            Ok(token) => {
                if token.is_expired() {
                    let _result = token.delete(pool).await;
                    None
                } else {
                    Some(token)
                }
            }
            Err(_) => None,
        }
    }
}

impl From<Grant> for OAuth2Token {
    fn from(grant: Grant) -> Self {
        let claims = Claims::new(
            ClaimsType::Auth,
            grant.owner_id.clone(),
            grant.client_id.clone(),
            SESSION_TIMEOUT,
        );
        let mut rnd = RandomGenerator::new(16);
        let refresh_token = rnd.tag(1, &grant).unwrap();
        Self {
            access_token: claims.to_jwt().unwrap(),
            refresh_token,
            redirect_uri: grant.redirect_uri.to_string(),
            scope: grant.scope.to_string(),
            expires_in: claims.exp as i64,
        }
    }
}

impl From<OAuth2Token> for Grant {
    fn from(token: OAuth2Token) -> Self {
        let claims = Claims::from_jwt(ClaimsType::Auth, &token.access_token).unwrap();
        Self {
            owner_id: claims.sub,
            client_id: claims.client_id,
            scope: token.scope.parse().unwrap(),
            redirect_uri: token.redirect_uri.parse().unwrap(),
            until: Utc::now() + Duration::minutes(1),
            extensions: Extensions::new(),
        }
    }
}

impl From<OAuth2Token> for IssuedToken {
    fn from(token: OAuth2Token) -> Self {
        Self {
            token: token.access_token,
            refresh: Some(token.refresh_token),
            until: Utc
                .timestamp_opt(token.expires_in, 0)
                .earliest()
                .unwrap_or_default(),
            token_type: TokenType::Bearer,
        }
    }
}

impl From<OAuth2Token> for RefreshedToken {
    fn from(token: OAuth2Token) -> Self {
        Self {
            token: token.access_token,
            refresh: Some(token.refresh_token),
            until: Utc
                .timestamp_opt(token.expires_in, 0)
                .earliest()
                .unwrap_or_default(),
            token_type: TokenType::Bearer,
        }
    }
}
