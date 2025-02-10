use chrono::NaiveDateTime;
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError, PgExecutor};

use crate::db::{Id, NoId};

#[derive(Deserialize, Model, Serialize)]
#[table(api_token)]
pub struct ApiToken<I = NoId> {
    id: I,
    pub user_id: Id,
    pub created_at: NaiveDateTime,
    pub name: String,
    pub token_hash: String,
}

impl ApiToken {
    pub fn new(user_id: Id, created_at: NaiveDateTime, name: String, token_string: &str) -> Self {
        let token_hash = Self::hash_token(token_string);
        Self {
            id: NoId,
            user_id,
            created_at,
            name,
            token_hash,
        }
    }

    /// Generates an SHA256 hash which can be stored in a database based on a token string.
    fn hash_token(token_string: &str) -> String {
        sha256::digest(token_string)
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
                    FROM api_token WHERE user_id = $1 ORDER BY id",
            user_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn try_find_by_auth_token<'e, E>(
        executor: E,
        auth_token: &str,
    ) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let token_hash = ApiToken::hash_token(auth_token);
        let maybe_token = query_as!(
            Self,
            "SELECT id, user_id, created_at, name, token_hash \
                    FROM api_token WHERE token_hash = $1",
            token_hash
        )
        .fetch_optional(executor)
        .await?;
        Ok(maybe_token)
    }
}

#[derive(Deserialize, Serialize)]
pub struct ApiTokenInfo {
    pub id: Id,
    pub name: String,
    pub created_at: NaiveDateTime,
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
