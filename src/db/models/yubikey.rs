use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError, PgExecutor};

#[derive(Deserialize, Model, Serialize)]
pub struct YubiKey {
    pub id: Option<i64>,
    pub name: String,
    pub serial: String,
    pub user_id: i64,
}

impl YubiKey {
    #[must_use]
    pub fn new(name: String, serial: String, user_id: i64) -> Self {
        Self {
            id: None,
            name,
            serial,
            user_id,
        }
    }

    pub async fn find_by_user_id<'e, E>(executor: E, user_id: i64) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT * FROM \"yubikey\" WHERE user_id = $1",
            user_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn delete_by_id<'e, E>(executor: E, id: i64) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!("DELETE FROM \"yubikey\" WHERE id = $1", id)
            .execute(executor)
            .await?;
        Ok(())
    }
}
