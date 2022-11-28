use crate::{auth::SESSION_TIMEOUT, db::DbPool, random::gen_alphanumeric};
use chrono::{Duration, Utc};
use sqlx::{query, query_as, Error as SqlxError};

pub struct OAuth2Token {
    pub(crate) user_id: i64,
    pub access_token: String,
    pub refresh_token: String,
    pub redirect_uri: String,
    pub scope: String,
    pub expires_in: i64,
}

impl OAuth2Token {
    #[must_use]
    pub fn new(user_id: i64, redirect_uri: String, scope: String) -> Self {
        let expiration = Utc::now() + Duration::seconds(SESSION_TIMEOUT as i64);
        Self {
            user_id,
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
            "INSERT INTO oauth2token (user_id, access_token, refresh_token, redirect_uri, scope, expires_in) \
            VALUES ($1, $2, $3, $4, $5, $6)",
            self.user_id,
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
            "SELECT user_id, access_token, refresh_token, redirect_uri, scope, expires_in \
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
            "SELECT user_id, access_token, refresh_token, redirect_uri, scope, expires_in \
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
