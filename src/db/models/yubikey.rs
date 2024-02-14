use sqlx::{query, query_as, Error as SqlxError, PgExecutor};

#[derive(Debug, Deserialize, Serialize, Clone)]
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

    pub async fn save<'e, E>(&mut self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        match self.id {
            Some(id) => {
                query!(
                    "UPDATE \"yubikey\" \
                SET name = $1, \
                serial = $2, \
                user_id = $3 \
                WHERE id = $4;
                ",
                    self.name,
                    self.serial,
                    self.user_id,
                    id
                )
                .execute(executor)
                .await?;
                Ok(())
            }
            None => {
                let q_res = query!(
                    "INSERT INTO \"yubikey\" (name, serial, user_id) \
                VALUES ($1,$2,$3) \
                RETURNING id;",
                    self.name,
                    self.serial,
                    self.user_id
                )
                .fetch_one(executor)
                .await?;
                self.id = Some(q_res.id);
                Ok(())
            }
        }
    }

    pub async fn find_by_id<'e, E>(executor: E, id: i64) -> Result<Self, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(Self, "SELECT * FROM \"yubikey\" WHERE id = $1", id)
            .fetch_one(executor)
            .await
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
