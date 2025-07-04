use model_derive::Model;
use sqlx::{PgExecutor, query, query_as};

use crate::db::{Id, NoId};

#[derive(Deserialize, Model, Serialize)]
pub struct YubiKey<I = NoId> {
    pub id: I,
    pub name: String,
    pub serial: String,
    pub user_id: Id,
}

impl YubiKey {
    #[must_use]
    pub fn new(name: String, serial: String, user_id: Id) -> Self {
        Self {
            id: NoId,
            name,
            serial,
            user_id,
        }
    }
}

impl YubiKey<Id> {
    pub async fn find_by_user_id<'e, E>(executor: E, user_id: Id) -> Result<Vec<Self>, sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, name, serial, user_id FROM \"yubikey\" WHERE user_id = $1",
            user_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn delete_by_id<'e, E>(executor: E, id: Id) -> Result<(), sqlx::Error>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM \"yubikey\" WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(())
    }
}
