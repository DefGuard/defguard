use crate::db::{DbPool, User};
use sqlx::{query, query_as, Error as SqlxError};

#[derive(Deserialize, Serialize)]
pub struct OAuth2Client {
    #[serde(skip)]
    pub(crate) user_id: i64,
    pub client_id: String, // unique
    pub client_secret: String,
    pub redirect_uri: String, // TODO: Vec<String>
    pub scope: Vec<String>,
    // informational
    pub name: String,
    pub enabled: bool,
}

impl OAuth2Client {
    /// Find client by 'client_id`.
    pub async fn find_by_client_id(
        pool: &DbPool,
        client_id: &str,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT user_id, client_id, client_secret, redirect_uri, scope, name, enabled \
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
            "SELECT user_id, client_id, client_secret, redirect_uri, scope, name, enabled \
            FROM oauth2client WHERE client_id = $1 AND enabled",
            client_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Store data in the database.
    pub async fn save(&self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "INSERT INTO oauth2client (user_id, client_id, client_secret, redirect_uri, scope, name, enabled) \
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
            self.user_id,
            self.client_id,
            self.client_secret,
            self.redirect_uri,
            &self.scope,
            self.name,
            self.enabled,
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(self, pool: &DbPool) -> Result<(), SqlxError> {
        query!(
            "DELETE FROM oauth2client WHERE client_id = $1",
            self.client_id,
        )
        .execute(pool)
        .await?;
        Ok(())
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
