use chrono::NaiveDateTime;
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError, PgExecutor};

use crate::db::{Id, NoId};

#[derive(Deserialize, Model, Serialize)]
#[table(api_token)]
pub(crate) struct ApiToken<I = NoId> {
    id: I,
    pub user_id: Id,
    pub created_at: NaiveDateTime,
    pub name: String,
    pub token_hash: String,
}

impl ApiToken {
    #[must_use]
    pub fn new(user_id: Id, created_at: NaiveDateTime, name: String, token_hash: String) -> Self {
        Self {
            id: NoId,
            user_id,
            created_at,
            name,
            token_hash,
        }
    }
}

impl ApiToken<Id> {
    pub async fn find_by_user_id<'e, E>(executor: E, user_id: Id) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, user_id, created_at, name, token_hash \
                    FROM api_token WHERE user_id = $1",
            user_id
        )
        .fetch_all(executor)
        .await
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct ApiTokenInfo {
    id: Id,
    name: String,
    created_at: NaiveDateTime,
}

impl From<ApiToken<Id>> for ApiTokenInfo {
    fn from(token: ApiToken<Id>) -> Self {
        Self {
            id: token.id,
            name: token.name,
            created_at: token.created_at,
        }
    }
}
