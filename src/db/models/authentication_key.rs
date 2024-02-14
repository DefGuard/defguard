use chrono::{NaiveDateTime, Utc};
use sqlx::{
    prelude::{FromRow, Type},
    query, query_as, Encode, Error as SqlxError, PgExecutor,
};
use strum::{AsRefStr, EnumString};

use crate::db::User;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Type, EnumString, AsRefStr, Copy)]
#[sqlx(type_name = "authentication_key_type", rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum AuthenticationKeyType {
    SSH,
    GPG,
}

#[derive(Clone, Deserialize, Serialize, Debug, FromRow, Encode)]
pub struct AuthenticationKey {
    id: Option<i64>,
    pub yubikey_id: Option<i64>,
    pub name: Option<String>,
    pub user_id: i64,
    pub key: String,
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

    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        match self.id {
            Some(_) => {
                query!(
                    "UPDATE \"authentication_key\" SET \
                yubikey_id = $1, \
                name = $2, \
                user_id = $3, \
                key = $4, \
                key_type = $5, \
                created = $6 \
                WHERE id = $7;
                ",
                    self.yubikey_id,
                    self.name,
                    self.user_id,
                    self.key,
                    self.key_type.clone() as AuthenticationKeyType,
                    self.created,
                    self.id
                )
                .execute(executor)
                .await?;
                Ok(())
            }
            None => {
                let res = query!(
                    "INSERT INTO \
                    authentication_key (yubikey_id, name, user_id, key, key_type, created) \
                    VALUES ($1,$2,$3,$4,$5,$6) \
                    RETURNING id;",
                    self.yubikey_id,
                    self.name,
                    self.user_id,
                    self.key,
                    self.key_type.clone() as AuthenticationKeyType,
                    self.created
                )
                .fetch_one(executor)
                .await?;
                self.id = Some(res.id);
                Ok(())
            }
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
                query_as(
                    "SELECT id \"id?\", user_id, yubikey_id \"yubikey_id?\", \
                    key, name, key_type \"key_type: AuthenticationKeyType\", created \
                    FROM \"authentication_key\"
                    WHERE user_id = $1 AND key_type = $2",
                )
                .bind(user_id)
                .bind(key_type.as_ref())
                .fetch_all(executor)
                .await
            }
            None => {
                query_as!(
                    Self,
                    "SELECT id \"id?\", user_id, yubikey_id \"yubikey_id?\", key, name, \
                    key_type \"key_type: AuthenticationKeyType\", created \
                    FROM \"authentication_key\"
                    WHERE user_id = $1",
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

    pub async fn find_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        Ok(query_as!(
            Self,
            "SELECT id, user_id, yubikey_id, key, name, \
            key_type \"key_type: _\", created FROM \"authentication_key\" \
            WHERE id = $1",
            id
        )
        .fetch_optional(executor)
        .await?)
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

    pub async fn delete<'e, E>(self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        match self.id {
            Some(id) => Self::delete_by_id(executor, id).await,
            None => Err(SqlxError::RowNotFound),
        }
    }
}
