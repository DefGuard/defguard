use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError, PgPool};

use crate::{
    db::{Id, NoId},
    secret::SecretString,
};

#[derive(Deserialize, Model, Serialize)]
pub struct OpenIdProvider<I = NoId> {
    pub id: I,
    pub name: String,
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub display_name: Option<String>,
    // Specific stuff for Google
    pub google_service_account_key: Option<String>,
    pub google_service_account_email: Option<String>,
    pub admin_email: Option<String>,
}

impl OpenIdProvider {
    #[must_use]
    pub fn new<S: Into<String>>(
        name: S,
        base_url: S,
        client_id: S,
        client_secret: S,
        display_name: Option<String>,
        google_service_account_key: Option<String>,
        google_service_account_email: Option<String>,
        admin_email: Option<String>,
    ) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            base_url: base_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            display_name,
            google_service_account_key,
            google_service_account_email,
            admin_email,
        }
    }

    pub async fn upsert(self, pool: &PgPool) -> Result<OpenIdProvider<Id>, SqlxError> {
        if let Some(provider) = OpenIdProvider::<Id>::get_current(pool).await? {
            query!(
                "UPDATE openidprovider SET name = $1, \
                base_url = $2, client_id = $3, client_secret = $4, \
                display_name = $5, google_service_account_key = $6, google_service_account_email = $7, admin_email = $8 \
                WHERE id = $9",
                self.name,
                self.base_url,
                self.client_id,
                self.client_secret,
                self.display_name,
                self.google_service_account_key,
                self.google_service_account_email,
                self.admin_email,
                provider.id,
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
            "SELECT id, name, base_url, client_id, client_secret, display_name, \
            google_service_account_key, google_service_account_email, admin_email \
            FROM openidprovider WHERE name = $1",
            name
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn get_current(pool: &PgPool) -> Result<Option<Self>, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id, name, base_url, client_id, client_secret, display_name, \
            google_service_account_key, google_service_account_email, admin_email \
            FROM openidprovider LIMIT 1"
        )
        .fetch_optional(pool)
        .await
    }
}
