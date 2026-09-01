use chrono::{TimeDelta, Utc};
use sqlx::{PgPool, query, query_as};

use crate::{
    db::{Id, models::Settings},
    random::gen_alphanumeric,
};

pub struct OAuth2Token {
    pub oauth2authorizedapp_id: Id,
    pub access_token: String,
    pub refresh_token: String,
    pub redirect_uri: String,
    pub scope: String,
    pub expires_in: i64,
}

impl OAuth2Token {
    #[must_use]
    pub fn new(oauth2authorizedapp_id: Id, redirect_uri: String, scope: String) -> Self {
        let settings = Settings::get_current_settings();
        let timeout = settings.authentication_timeout();
        let expiration = Utc::now() + TimeDelta::seconds(timeout.as_secs().cast_signed());
        Self {
            oauth2authorizedapp_id,
            access_token: gen_alphanumeric(24),
            refresh_token: gen_alphanumeric(24),
            redirect_uri,
            scope,
            expires_in: expiration.timestamp(),
        }
    }

    /// Generate new access token, scratching the old one. Changes are reflected in the database.
    pub async fn refresh_and_save(&mut self, pool: &PgPool) -> sqlx::Result<bool> {
        let settings = Settings::get_current_settings();
        let timeout = settings.authentication_timeout();
        let old_refresh_token = self.refresh_token.clone();
        let new_access_token = gen_alphanumeric(24);
        let new_refresh_token = gen_alphanumeric(24);
        let expiration = Utc::now() + TimeDelta::seconds(timeout.as_secs().cast_signed());
        let expires_in = expiration.timestamp();

        let result = query!(
            "UPDATE oauth2token SET access_token = $2, refresh_token = $3, expires_in = $4 \
            WHERE refresh_token = $1",
            old_refresh_token,
            new_access_token,
            new_refresh_token,
            expires_in,
        )
        .execute(pool)
        .await?;
        if result.rows_affected() != 1 {
            return Ok(false);
        }

        self.access_token = new_access_token;
        self.refresh_token = new_refresh_token;
        self.expires_in = expires_in;
        Ok(true)
    }

    /// Check if token has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_in < Utc::now().timestamp()
    }

    /// Store data in the database.
    pub async fn save(&self, pool: &PgPool) -> sqlx::Result<()> {
        query!(
            "INSERT INTO oauth2token (oauth2authorizedapp_id, access_token, refresh_token, \
            redirect_uri, scope, expires_in) \
            VALUES ($1, $2, $3, $4, $5, $6)",
            self.oauth2authorizedapp_id,
            self.access_token,
            self.refresh_token,
            self.redirect_uri,
            self.scope,
            self.expires_in
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete token from the database.
    pub async fn delete(self, pool: &PgPool) -> sqlx::Result<()> {
        query!(
            "DELETE FROM oauth2token WHERE access_token = $1 AND refresh_token = $2",
            self.access_token,
            self.refresh_token
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete all tokens for an authorized app.
    pub async fn delete_all_by_authorized_app_id(
        pool: &PgPool,
        oauth2authorizedapp_id: Id,
    ) -> sqlx::Result<()> {
        query!(
            "DELETE FROM oauth2token WHERE oauth2authorizedapp_id = $1",
            oauth2authorizedapp_id,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find by access token.
    pub async fn find_by_access_token(
        pool: &PgPool,
        access_token: &str,
    ) -> sqlx::Result<Option<Self>> {
        match query_as!(
            Self,
            "SELECT oauth2authorizedapp_id, access_token, refresh_token, redirect_uri, scope, \
            expires_in \
            FROM oauth2token WHERE access_token = $1",
            access_token
        )
        .fetch_optional(pool)
        .await
        {
            Ok(Some(token)) => {
                if token.is_expired() {
                    token.delete(pool).await?;
                    Ok(None)
                } else {
                    Ok(Some(token))
                }
            }
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Find by refresh token. Expired tokens are removed instead of returned.
    pub async fn find_by_refresh_token(
        pool: &PgPool,
        refresh_token: &str,
    ) -> sqlx::Result<Option<Self>> {
        match query_as!(
            Self,
            "SELECT oauth2authorizedapp_id, access_token, refresh_token, redirect_uri, scope, \
            expires_in \
            FROM oauth2token WHERE refresh_token = $1",
            refresh_token
        )
        .fetch_optional(pool)
        .await
        {
            Ok(Some(token)) => {
                if token.is_expired() {
                    token.delete(pool).await?;
                    Ok(None)
                } else {
                    Ok(Some(token))
                }
            }
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Find by refresh token for a specific OAuth2 client. Expired tokens are removed instead of returned.
    pub async fn find_by_refresh_token_for_client(
        pool: &PgPool,
        refresh_token: &str,
        oauth2client_id: Id,
    ) -> sqlx::Result<Option<Self>> {
        match query_as!(
            Self,
            "SELECT t.oauth2authorizedapp_id, t.access_token, t.refresh_token, t.redirect_uri, t.scope, \
            t.expires_in \
            FROM oauth2token t \
            JOIN oauth2authorizedapp a ON a.id = t.oauth2authorizedapp_id \
            WHERE t.refresh_token = $1 AND a.oauth2client_id = $2",
            refresh_token,
            oauth2client_id,
        )
        .fetch_optional(pool)
        .await
        {
            Ok(Some(token)) => {
                if token.is_expired() {
                    token.delete(pool).await?;
                    Ok(None)
                } else {
                    Ok(Some(token))
                }
            }
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }
}
