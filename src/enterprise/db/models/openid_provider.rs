use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError};

use crate::db::DbPool;

// TODO(jck): maybe rename OpenIdProvider
#[derive(Deserialize, Model, Serialize)]
pub struct OpenIdProvider {
    pub id: Option<i64>,
    pub name: String,
    pub base_url: String,
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
    pub fn new<S: Into<String>>(name: S, base_url: S, client_id: S, client_secret: S) -> Self {
        Self {
            id: None,
            name: name.into(),
            base_url: base_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            enabled: false,
        }
    }

    pub async fn find_by_name(pool: &DbPool, name: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id \"id?\", name, base_url, client_id, client_secret, enabled FROM openidprovider WHERE name = $1",
            name
        )
        .fetch_optional(pool)
        .await
    }

    // TODO: this is a temporary method. We currently support only one provider at a time
    pub async fn upsert(&mut self, pool: &DbPool) -> Result<(), SqlxError> {
        // TODO(aleksander): do it with only one query?
        if let Some(provider) = OpenIdProvider::get_current(pool).await? {
            query!(
                "UPDATE openidprovider SET name = $1, base_url = $2, client_id = $3, client_secret = $4, enabled = $5 WHERE id = $6",
                self.name,
                self.base_url,
                self.client_id,
                self.client_secret,
                self.enabled,
                provider.id
            )
            .execute(pool)
            .await?;
        } else {
            self.save(pool).await?;
        }

        Ok(())
    }

    // TODO: this is a temporary method. We currently support only one provider at a time
    pub async fn get_current(pool: &DbPool) -> Result<Option<Self>, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id \"id?\", name, base_url, client_id, client_secret, enabled FROM openidprovider"
        )
        .fetch_optional(pool)
        .await
    }
}
