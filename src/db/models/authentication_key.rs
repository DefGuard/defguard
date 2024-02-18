use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError, PgExecutor, Type};
use strum::EnumString;

use super::User;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Type, EnumString)]
#[sqlx(type_name = "authentication_key_type", rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AuthenticationKeyType {
    SSH,
    GPG,
}

#[derive(Deserialize, Model, Serialize)]
#[table(authentication_key)]
pub struct AuthenticationKey {
    id: Option<i64>,
    pub yubikey_id: Option<i64>,
    pub name: Option<String>,
    pub user_id: i64,
    pub key: String,
    #[model(enum)]
    pub key_type: AuthenticationKeyType,
    pub created: NaiveDateTime,
}

impl AuthenticationKey {
    #[must_use]
    pub fn new(
        user_id: i64,
        key: String,
        name: Option<String>,
        key_type: AuthenticationKeyType,
        yubikey_id: Option<i64>,
    ) -> Self {
        Self {
            id: None,
            yubikey_id,
            user_id,
            key,
            name,
            key_type,
            created: Utc::now().naive_utc(),
        }
    }

    pub async fn find_by_user_id<'e, E>(
        executor: E,
        user_id: i64,
        key_type: Option<AuthenticationKeyType>,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        match key_type {
            Some(key_type) => {
                query_as!(
                    Self,
                    "SELECT id \"id?\", user_id, yubikey_id \"yubikey_id?\", key, \
                    name, key_type \"key_type: AuthenticationKeyType\", created \
                    FROM authentication_key WHERE user_id = $1 AND key_type = $2",
                    user_id,
                    &key_type as &AuthenticationKeyType
                )
                .fetch_all(executor)
                .await
            }
            None => {
                query_as!(
                    Self,
                    "SELECT id \"id?\", user_id, yubikey_id \"yubikey_id?\", key, \
                    name, key_type \"key_type: AuthenticationKeyType\", created \
                    FROM authentication_key WHERE user_id = $1",
                    user_id
                )
                .fetch_all(executor)
                .await
            }
        }
    }

    pub async fn find_by_user<'e, E>(
        executor: E,
        user: User,
        key_type: Option<AuthenticationKeyType>,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        match user.id {
            Some(user_id) => Self::find_by_user_id(executor, user_id, key_type).await,
            None => Err(SqlxError::RowNotFound),
        }
    }

    pub async fn delete_by_id<'e, E>(executor: E, id: i64) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM \"authentication_key\" WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(())
    }
}
