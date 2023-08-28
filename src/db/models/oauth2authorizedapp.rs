use super::DbPool;
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError};

#[derive(Model)]
pub struct OAuth2AuthorizedApp {
    pub id: Option<i64>,
    pub user_id: i64,
    pub oauth2client_id: i64,
}

impl OAuth2AuthorizedApp {
    #[must_use]
    pub fn new(user_id: i64, oauth2client_id: i64) -> Self {
        Self {
            id: None,
            user_id,
            oauth2client_id,
        }
    }
    pub async fn find_by_user_and_oauth2client_id(
        pool: &DbPool,
        user_id: i64,
        oauth2client_id: i64,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, oauth2client_id \
            FROM oauth2authorizedapp WHERE user_id = $1 AND oauth2client_id = $2",
            user_id,
            oauth2client_id
        )
        .fetch_optional(pool)
        .await
    }
}
