use super::NewOpenIDClient;
use crate::{
    db::{DbPool, User},
    random::gen_alphanumeric,
};
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError};

#[derive(Deserialize, Model, Serialize)]
pub struct OAuth2Client {
    #[serde(skip)]
    pub(crate) id: Option<i64>,
    #[serde(skip)]
    pub(crate) user_id: i64,
    pub client_id: String, // unique
    pub client_secret: String,
    pub redirect_uri: String, // TODO: Vec<String>
    #[model(ref)]
    pub scope: Vec<String>,
    // informational
    pub name: String,
    pub enabled: bool,
}

impl OAuth2Client {
    #[must_use]
    pub fn new(user_id: i64, redirect_uri: String, scope: Vec<String>, name: String) -> Self {
        let client_id = gen_alphanumeric(16);
        let client_secret = gen_alphanumeric(32);
        Self {
            id: None,
            user_id,
            client_id,
            client_secret,
            redirect_uri,
            scope,
            name,
            enabled: true,
        }
    }

    #[must_use]
    pub fn from_new(new: NewOpenIDClient, user_id: i64) -> Self {
        let client_id = gen_alphanumeric(16);
        let client_secret = gen_alphanumeric(32);
        Self {
            id: None,
            user_id,
            client_id,
            client_secret,
            redirect_uri: new.redirect_uri,
            scope: new.scope,
            name: new.name,
            enabled: new.enabled,
        }
    }

    /// All by `user_id`.
    pub async fn all_for_user(pool: &DbPool, user_id: i64) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, client_id, client_secret, redirect_uri, scope, name, enabled \
            FROM oauth2client WHERE user_id = $1",
            user_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find client by 'client_id`.
    pub async fn find_by_client_id(
        pool: &DbPool,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, client_id, client_secret, redirect_uri, scope, name, enabled \
            FROM oauth2client WHERE client_id = $1",
            client_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find enabled client by `client_id`.
    pub async fn find_enabled_for_client_id(
        pool: &DbPool,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, client_id, client_secret, redirect_uri, scope, name, enabled \
            FROM oauth2client WHERE client_id = $1 AND enabled",
            client_id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn set_for_user(&mut self, pool: &DbPool, user: &User) -> Result<bool, SqlxError> {
        if let Some(id) = user.id {
            self.user_id = id;
            self.save(pool).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
