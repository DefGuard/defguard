// use model_derive::Model;
use sqlx::{query, query_as, query_scalar, PgExecutor};

pub struct NoId;
pub type Id = i64;

#[derive(Deserialize, Serialize)]
pub struct YubiKey<I> {
    pub id: I,
    pub name: String,
    pub serial: String,
    pub user_id: i64,
}

impl YubiKey<NoId> {
    #[must_use]
    pub fn new(name: String, serial: String, user_id: i64) -> Self {
        Self {
            id: NoId,
            name,
            serial,
            user_id,
        }
    }

    pub async fn save<'e, E>(self, executor: E) -> Result<YubiKey<Id>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        let id = query_scalar!(
            "INSERT INTO \"yubikey\" (name, serial, user_id) VALUES ($1, $2, $3) RETURNING id",
            self.name,
            self.serial,
            self.user_id
        )
        .fetch_one(executor)
        .await?;

        Ok(YubiKey {
            id,
            name: self.name,
            serial: self.serial,
            user_id: self.user_id,
        })
    }
}

impl YubiKey<Id> {
    pub async fn find_by_user_id<'e, E>(executor: E, user_id: i64) -> Result<Vec<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", name, serial, user_id FROM \"yubikey\" WHERE user_id = $1",
            user_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn delete_by_id<'e, E>(executor: E, id: i64) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM \"yubikey\" WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(())
    }

    // XXX: remove if Model is updated
    pub async fn find_by_id<'e, E>(executor: E, id: i64) -> Result<Option<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id \"id: _\", name, serial, user_id FROM \"yubikey\" WHERE id = $1",
            id
        )
        .fetch_optional(executor)
        .await
    }

    // XXX: remove if Model is updated
    pub async fn delete<'e, E>(self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM \"yubikey\" WHERE id = $1", self.id)
            .execute(executor)
            .await?;

        Ok(())
    }

    // XXX: remove if Model is updated
    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "UPDATE \"yubikey\" SET name = $2, serial = $3, user_id = $4 WHERE id = $1",
            self.id,
            self.name,
            self.serial,
            self.user_id
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
