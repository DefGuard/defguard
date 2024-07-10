use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError};

use crate::db::DbPool;

// TODO(jck): maybe rename OpenIdProvider
#[derive(Deserialize, Model, Serialize)]
pub struct OpenIdProvider {
    pub id: Option<i64>,
    pub name: String,
    pub provider_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: bool,
    // pub client_id: String, // unique
    // // TODO(jck): maybe remove since we get the id_token in the first reponse?
    // pub client_secret: String,
    // pub auth_url: String,
    // TODO(jck): provider image?

    // // TODO(jck): do we need this?
    // #[model(ref)]
    // pub redirect_uri: Vec<String>,
    // // TODO(jck): can we assume constant scope ahead of time?
    // #[model(ref)]
    // pub scope: Vec<String>,
    // // TODO(jck): remove?
    // // informational
    // pub name: String,
    // pub enabled: bool,
}

impl OpenIdProvider {
    #[must_use]
    pub fn new<S: Into<String>>(name: S, provider_url: S, client_id: S, client_secret: S) -> Self {
        Self {
            id: None,
            name: name.into(),
            provider_url: provider_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            enabled: false,
        }
    }

    pub async fn find_by_name(pool: &DbPool, name: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id \"id?\", name, provider_url, client_id, client_secret, enabled FROM openidprovider WHERE name = $1",
            name
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn exists(pool: &DbPool, provider: &OpenIdProvider) -> Result<bool, SqlxError> {
        query!(
            "SELECT EXISTS(SELECT 1 FROM openidprovider WHERE name = $1 OR client_id = $2 OR client_secret = $3)",
            provider.name,
            provider.client_id,
            provider.client_secret
        )
        .fetch_one(pool)
        .await?
        .exists
        .ok_or_else(|| SqlxError::RowNotFound)
    }

    // TODO(aleksander): there may be more than one active provider
    pub async fn get_enabled(pool: &DbPool) -> Result<Self, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id \"id?\", name, provider_url, client_id, client_secret, enabled FROM openidprovider WHERE enabled = true limit 1"
        )
        .fetch_one(pool)
        .await
    }
}
