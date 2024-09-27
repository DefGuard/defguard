use model_derive::Model;
use sqlx::{query_as, Error as SqlxError, PgPool};

use crate::db::{Id, NoId};

#[derive(Model)]
pub struct OAuth2AuthorizedApp<I = NoId> {
    pub id: I,
    pub user_id: Id,
    pub oauth2client_id: Id,
}

impl OAuth2AuthorizedApp {
    #[must_use]
    pub fn new(user_id: Id, oauth2client_id: Id) -> Self {
        Self {
            id: NoId,
            user_id,
            oauth2client_id,
        }
    }
}

impl OAuth2AuthorizedApp<Id> {
    pub async fn find_by_user_and_oauth2client_id(
        pool: &PgPool,
        user_id: Id,
        oauth2client_id: Id,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id: _\", user_id, oauth2client_id \
            FROM oauth2authorizedapp WHERE user_id = $1 AND oauth2client_id = $2",
            user_id,
            oauth2client_id
        )
        .fetch_optional(pool)
        .await
    }
}
