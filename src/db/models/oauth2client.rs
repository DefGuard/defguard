use model_derive::Model;
use sqlx::{query_as, Error as SqlxError, PgPool};

use super::NewOpenIDClient;
use crate::{
    db::{Id, NoId},
    random::gen_alphanumeric,
};

#[derive(Deserialize, Model, Serialize)]
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

impl OAuth2Client {
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

    #[must_use]
    pub fn from_new(new: NewOpenIDClient) -> Self {
        let client_id = gen_alphanumeric(16);
        let client_secret = gen_alphanumeric(32);
        Self {
            id: NoId,
            client_id,
            client_secret,
            redirect_uri: new.redirect_uri,
            scope: new.scope,
            name: new.name,
            enabled: new.enabled,
        }
    }
}

impl OAuth2Client<Id> {
    /// Find client by 'client_id`.
    pub async fn find_by_client_id(
        pool: &PgPool,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, client_id, client_secret, redirect_uri, scope, name, enabled \
            FROM oauth2client WHERE client_id = $1",
            client_id
        )
        .fetch_optional(pool)
        .await
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

    /// Find enabled client by `client_id`.
    pub async fn find_enabled_for_client_id(
        pool: &PgPool,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, client_id, client_secret, redirect_uri, scope, name, enabled \
            FROM oauth2client WHERE client_id = $1 AND enabled",
            client_id
        )
        .fetch_optional(pool)
        .await
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
