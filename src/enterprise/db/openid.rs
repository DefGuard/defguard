use crate::db::{DbPool, User};
use model_derive::Model;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sqlx::{query_as, Error as SqlxError};

#[derive(Deserialize, Model, Serialize)]
pub struct OpenIDClient {
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    pub home_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub enabled: bool,
}

impl OpenIDClient {
    /// Find client by `client_id`.
    pub async fn find_enabled_for_client_id(
        pool: &DbPool,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
                Self,
                "SELECT id \"id?\", name, home_url, client_id, client_secret, redirect_uri, description, enabled \
                FROM openidclient WHERE client_id = $1 AND enabled", client_id)
                .fetch_optional(pool)
                .await
    }
}

#[derive(Deserialize, Serialize)]
pub struct NewOpenIDClient {
    pub name: String,
    pub description: String,
    pub home_url: String,
    pub redirect_uri: String,
    pub enabled: bool,
}

impl From<NewOpenIDClient> for OpenIDClient {
    fn from(new: NewOpenIDClient) -> Self {
        let client_id = thread_rng()
            .sample_iter(Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        let client_secret = thread_rng()
            .sample_iter(Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        Self {
            id: None,
            name: new.name,
            description: new.description,
            home_url: new.home_url,
            client_id,
            client_secret,
            redirect_uri: new.redirect_uri,
            enabled: new.enabled,
        }
    }
}

#[derive(Deserialize, Model, Serialize)]
#[table(openidclientauthcode)]
pub struct OpenIDClientAuth {
    #[serde(default)]
    id: Option<i64>,
    /// User ID
    pub user: String,
    pub code: String,
    pub client_id: String,
    pub state: String,
    pub scope: String,
    pub redirect_uri: String,
    pub nonce: Option<String>,
}

impl OpenIDClientAuth {
    #[must_use]
    pub fn new(
        user: String,
        code: String,
        client_id: String,
        state: String,
        redirect_uri: String,
        scope: String,
        nonce: Option<String>,
    ) -> Self {
        Self {
            id: None,
            user,
            code,
            client_id,
            state,
            scope,
            redirect_uri,
            nonce,
        }
    }

    /// Get client by code
    pub async fn find_by_code(pool: &DbPool, code: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", \"user\", code, client_id, state, scope, redirect_uri, nonce \
            FROM openidclientauthcode WHERE code = $1",
            code
        )
        .fetch_optional(pool)
        .await
    }
}

#[derive(Deserialize, Model, PartialEq, Serialize)]
#[table(authorizedapps)]
pub struct AuthorizedApp {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub user_id: i64,
    pub client_id: String,
    pub home_url: String,
    pub date: String, // TODO: NaiveDateTime %d-%m-%Y %H:%M
    pub name: String,
}

impl AuthorizedApp {
    #[must_use]
    pub fn new(
        user_id: i64,
        client_id: String,
        home_url: String,
        date: String,
        name: String,
    ) -> Self {
        Self {
            id: None,
            user_id,
            client_id,
            home_url,
            date,
            name,
        }
    }

    pub async fn find_by_user_and_client_id(
        pool: &DbPool,
        user_id: i64,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, client_id, home_url, date, name \
            FROM authorizedapps WHERE user_id = $1 AND client_id = $2",
            user_id,
            client_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn all_for_user(pool: &DbPool, user: &User) -> Result<Vec<Self>, SqlxError> {
        if let Some(id) = user.id {
            query_as!(
                Self,
                "SELECT id \"id?\", user_id, client_id, home_url, date, name \
                FROM authorizedapps WHERE user_id = $1",
                id
            )
            .fetch_all(pool)
            .await
        } else {
            Ok(Vec::new())
        }
    }
}
