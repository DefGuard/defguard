use crate::{
    auth::{Claims, SESSION_TIMEOUT},
    db::DbPool,
};
use chrono::{Duration, TimeZone, Utc};
use oxide_auth::primitives::{
    generator::{RandomGenerator, TagGrant},
    grant::{Extensions, Grant},
    issuer::{IssuedToken, RefreshedToken, TokenType},
};
use sqlx::{query, query_as, Error as SqlxError};

pub struct OAuth2Client {
    /// user ID
    pub user: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scope: String,
}

impl OAuth2Client {
    /// Find client by ID.
    pub async fn find_client_id(pool: &DbPool, client_id: &str) -> Option<Self> {
        query_as!(
            Self,
            "SELECT \"user\", client_id, client_secret, redirect_uri, scope \
            FROM oauth2client WHERE client_id = $1",
            client_id
        )
        .fetch_one(pool)
        .await
        .ok()
    }

    /// Store data in the database.
    pub async fn save(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "INSERT INTO oauth2client (\"user\", client_id, client_secret, redirect_uri, scope) \
            VALUES ($1, $2, $3, $4, $5)",
            self.user,
            self.client_id,
            self.client_secret,
            self.redirect_uri,
            self.scope
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

pub struct OAuth2Token {
    pub access_token: String,
    pub refresh_token: String,
    pub redirect_uri: String,
    pub scope: String,
    pub expires_in: i64,
}

impl OAuth2Token {
    /// Generate new access token, scratching the old one. Changes are reflected in the database.
    pub async fn refresh_and_save(
        &mut self,
        pool: &DbPool,
        grant: &Grant,
    ) -> Result<(), SqlxError> {
        let claims = Claims::new(
            grant.owner_id.clone(),
            grant.client_id.clone(),
            SESSION_TIMEOUT,
        );
        let new_access_token = claims.to_jwt().unwrap();
        let mut rnd = RandomGenerator::new(16);
        let new_refresh_token = rnd.tag(1, grant).unwrap();

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
        let claims = Claims::from_jwt(&token.access_token).unwrap();
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
            until: Utc.timestamp(token.expires_in, 0),
            token_type: TokenType::Bearer,
        }
    }
}

impl From<OAuth2Token> for RefreshedToken {
    fn from(token: OAuth2Token) -> Self {
        Self {
            token: token.access_token,
            refresh: Some(token.refresh_token),
            until: Utc.timestamp(token.expires_in, 0),
            token_type: TokenType::Bearer,
        }
    }
}

pub struct AuthorizationCode {
    /// user ID
    pub user: String,
    pub client_id: String,
    pub code: String,
    pub redirect_uri: String,
    pub scope: String,
    pub auth_time: i64,
}

impl AuthorizationCode {
    /// Store data in the database.
    pub async fn save(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "INSERT INTO authorization_code \
            (\"user\", client_id, code, redirect_uri, scope, auth_time) \
            VALUES ($1, $2, $3, $4, $5, $6)",
            self.user,
            self.client_id,
            self.code,
            self.redirect_uri,
            self.scope,
            self.auth_time
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete from the database.
    pub async fn delete(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "DELETE FROM authorization_code WHERE client_id = $1 AND code = $2",
            self.client_id,
            self.code,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find by code.
    pub async fn find_code(pool: &DbPool, code: &str) -> Option<Self> {
        query_as!(
            Self,
            "SELECT \"user\", client_id, code, redirect_uri, scope, auth_time \
            FROM authorization_code WHERE code = $1",
            code
        )
        .fetch_one(pool)
        .await
        .ok()
    }
}

impl From<Grant> for AuthorizationCode {
    fn from(grant: Grant) -> Self {
        let mut rnd = RandomGenerator::new(16);
        let code = rnd.tag(2, &grant).unwrap();
        Self {
            user: grant.owner_id,
            client_id: grant.client_id,
            code,
            redirect_uri: grant.redirect_uri.to_string(),
            scope: grant.scope.to_string(),
            auth_time: Utc::now().timestamp(),
        }
    }
}

impl From<AuthorizationCode> for Grant {
    fn from(code: AuthorizationCode) -> Self {
        Self {
            owner_id: code.user,
            client_id: code.client_id,
            scope: code.scope.parse().unwrap(),
            redirect_uri: code.redirect_uri.parse().unwrap(),
            until: Utc::now() + Duration::minutes(1),
            extensions: Extensions::new(),
        }
    }
}
