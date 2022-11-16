use crate::db::{DbPool, User};
use sqlx::{query, query_as, Error as SqlxError};

#[derive(Deserialize)]
pub struct OAuth2Client {
    #[serde(skip)]
    user_id: i64,
    client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scope: String, // TODO: Vec<String>
}

impl OAuth2Client {
    /// Find client by ID.
    pub async fn find_by_client_id(pool: &DbPool, client_id: &str) -> Option<Self> {
        query_as!(
            Self,
            "SELECT user_id, client_id, client_secret, redirect_uri, scope \
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
            "INSERT INTO oauth2client (user_id, client_id, client_secret, redirect_uri, scope) \
            VALUES ($1, $2, $3, $4, $5)",
            self.user_id,
            self.client_id,
            self.client_secret,
            self.redirect_uri,
            self.scope
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
