use std::fmt::{self, Display, Formatter};

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::{query_as, Error as SqlxError};

use crate::db::DbPool;

#[derive(Clone, Deserialize, Model, Serialize, Debug)]
#[table(authentication_key)]
pub struct AuthenticationKey {
    id: Option<i64>,
    pub user_id: i64,
    pub key: String,
    pub name: String,
    pub key_type: String,
    pub created: NaiveDateTime,
}

impl Display for AuthenticationKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.id {
            Some(id) => write!(f, "[ID {}] {}", id, self.name),
            None => write!(f, "{}", self.name),
        }
    }
}

impl AuthenticationKey {
    #[must_use]
    pub fn new(user_id: i64, key: String, name: String, key_type: String) -> Self {
        Self {
            id: None,
            user_id,
            key,
            name,
            key_type,
            created: Utc::now().naive_utc(),
        }
    }

    pub async fn fetch_user_authentication_keys(
        pool: &DbPool,
        user_id: i64,
    ) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, key, name, key_type, created
            FROM authentication_key WHERE user_id = $1",
            user_id,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn fetch_user_authentication_keys_by_type(
        pool: &DbPool,
        user_id: i64,
        key_type: &str,
    ) -> Result<Vec<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, key, name, key_type, created
            FROM authentication_key WHERE user_id = $1 AND key_type = $2",
            user_id,
            key_type,
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_user(
        pool: &DbPool,
        user_id: i64,
        key: String,
    ) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, key, name, key_type, created
            FROM authentication_key WHERE user_id = $1 AND key = $2",
            user_id,
            key,
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_authentication_key(&self, pool: &DbPool) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id \"id?\", user_id, key, name, key_type, created
            FROM authentication_key WHERE user_id = $1 AND key = $2",
            self.user_id,
            self.key,
        )
        .fetch_optional(pool)
        .await
    }
}
