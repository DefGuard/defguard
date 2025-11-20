use crate::{
    db::{Id, NoId, models::OAuth2Token},
    random::gen_alphanumeric,
};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{Error as SqlxError, PgExecutor, PgPool, query_as};

#[derive(Clone, Debug, Deserialize, Model, Serialize, PartialEq)]
pub struct OAuth2Client<I = NoId> {
    pub id: I,
    pub client_id: String, // unique
    pub client_secret: String,
    #[model(ref)]
    pub redirect_uri: Vec<String>,
    #[model(ref)]
    pub scope: Vec<String>,
    // informational
    pub name: String,
    pub enabled: bool,
}

impl OAuth2Client<NoId> {
    #[must_use]
    pub fn new(redirect_uri: Vec<String>, scope: Vec<String>, name: String) -> Self {
        let client_id = gen_alphanumeric(16);
        let client_secret = gen_alphanumeric(32);
        Self {
            id: NoId,
            client_id,
            client_secret,
            redirect_uri,
            scope,
            name,
            enabled: true,
        }
    }
}

impl OAuth2Client<Id> {
    /// Find client by 'client_id`.
    pub async fn find_by_client_id<'e, E>(
        executor: E,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, client_id, client_secret, redirect_uri, scope, name, enabled \
            FROM oauth2client WHERE client_id = $1",
            client_id
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn clear_authorizations<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        sqlx::query!(
            "DELETE FROM oauth2authorizedapp WHERE oauth2client_id = $1",
            self.id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    /// Find using `client_id` and `client_secret`; must be `enabled`.
    pub async fn find_by_auth(
        pool: &PgPool,
        client_id: &str,
        client_secret: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, client_id, client_secret, redirect_uri, scope, name, enabled \
            FROM oauth2client WHERE client_id = $1 AND client_secret = $2 AND enabled",
            client_id,
            client_secret
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_token(
        pool: &PgPool,
        token: &OAuth2Token,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT c.id, c.client_id, c.client_secret, c.redirect_uri, c.scope, c.name, c.enabled \
            FROM oauth2client c \
            JOIN oauth2authorizedapp a ON a.oauth2client_id = c.id \
            JOIN oauth2token t ON t.oauth2authorizedapp_id = a.id \
            WHERE t.access_token = $1 OR t.refresh_token = $2",
            token.access_token,
            token.refresh_token
        )
        .fetch_optional(pool)
        .await
    }

    /// Checks if `url` matches client config (ignoring trailing slashes).
    pub fn contains_redirect_url(&self, url: &str) -> bool {
        let url_trimmed = url.trim_end_matches('/');

        for redirect in &self.redirect_uri {
            if url_trimmed == redirect.trim_end_matches('/') {
                return true;
            }
        }

        false
    }
}

// Safe to show for not privileged users
#[derive(Deserialize, Serialize)]
pub struct OAuth2ClientSafe {
    pub client_id: String,
    pub name: String,
    pub scope: Vec<String>,
}

impl From<OAuth2Client<Id>> for OAuth2ClientSafe {
    fn from(client: OAuth2Client<Id>) -> Self {
        OAuth2ClientSafe {
            client_id: client.client_id,
            name: client.name,
            scope: client.scope,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_redirect_url() {
        let oauth2client = OAuth2Client {
            id: 1,
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: vec![
                String::from("http://localhost/"),
                String::from("http://safe.net/"),
            ],
            scope: Vec::new(),
            name: String::new(),
            enabled: true,
        };
        assert!(oauth2client.contains_redirect_url("http://safe.net"));
        assert!(oauth2client.contains_redirect_url("http://localhost"));
        assert!(!oauth2client.contains_redirect_url("http://safe.net/api"));
        assert!(!oauth2client.contains_redirect_url("http://nonexistent:8000"));
    }
}
