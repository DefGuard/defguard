use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError, PgPool};

use crate::db::{Id, NoId};

#[derive(Deserialize, Model, Serialize)]
pub struct OpenIdProvider<I = NoId> {
    pub id: I,
    pub name: String,
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
}

impl OpenIdProvider {
    #[must_use]
    pub fn new<S: Into<String>>(name: S, base_url: S, client_id: S, client_secret: S) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            base_url: base_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        }
    }

    pub async fn upsert(self, pool: &PgPool) -> Result<OpenIdProvider<Id>, SqlxError> {
        if let Some(provider) = OpenIdProvider::<Id>::get_current(pool).await? {
            query!(
                "UPDATE openidprovider SET name = $1, base_url = $2, client_id = $3, client_secret = $4 WHERE id = $5",
                self.name,
                self.base_url,
                self.client_id,
                self.client_secret,
                provider.id
            )
            .execute(pool)
            .await?;

            Ok(provider)
        } else {
            self.save(pool).await
        }
    }
}

impl OpenIdProvider<Id> {
    pub async fn find_by_name(pool: &PgPool, name: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id, name, base_url, client_id, client_secret FROM openidprovider WHERE name = $1",
            name
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn get_current(pool: &PgPool) -> Result<Option<Self>, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id, name, base_url, client_id, client_secret FROM openidprovider"
        )
        .fetch_optional(pool)
        .await
    }
}
