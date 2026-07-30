use defguard_common::db::{Id, NoId};
use sqlx::{FromRow, PgExecutor, query, query_as};

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct UserDirectoryIdentity<I = NoId> {
    pub id: I,
    pub user_id: i64,
    pub provider_id: i64,
    pub external_id: String,
}

impl UserDirectoryIdentity<Id> {
    /// Find a user by their directory identity (provider + external_id).
    pub async fn find_user_by_provider_external_id<'e, E>(
        executor: E,
        provider_id: i64,
        external_id: &str,
    ) -> sqlx::Result<Option<i64>>
    where
        E: PgExecutor<'e>,
    {
        use sqlx::Row;
        let row = query(
            "SELECT user_id FROM user_directory_identity WHERE provider_id = $1 AND external_id = $2",
        )
        .bind(provider_id)
        .bind(external_id)
        .fetch_optional(executor)
        .await?;
        Ok(row.map(|r| r.get::<i64, _>("user_id")))
    }

    /// Get the directory identity for a user and provider if it exists.
    pub async fn find_by_user_and_provider<'e, E>(
        executor: E,
        user_id: i64,
        provider_id: i64,
    ) -> sqlx::Result<Option<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            UserDirectoryIdentity,
            "SELECT id, user_id, provider_id, external_id FROM user_directory_identity \
            WHERE user_id = $1 AND provider_id = $2",
            user_id,
            provider_id
        )
        .fetch_optional(executor)
        .await
    }

    /// Create or update the directory identity for a user and provider.
    /// If the mapping already exists, it updates the external_id.
    pub async fn upsert<'e, E>(
        executor: E,
        user_id: i64,
        provider_id: i64,
        external_id: &str,
    ) -> sqlx::Result<Self>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            UserDirectoryIdentity,
            "INSERT INTO user_directory_identity (user_id, provider_id, external_id) \
            VALUES ($1, $2, $3) \
            ON CONFLICT (user_id, provider_id) DO UPDATE SET external_id = $3 \
            RETURNING id, user_id, provider_id, external_id",
            user_id,
            provider_id,
            external_id
        )
        .fetch_one(executor)
        .await
    }

    /// Delete the directory identity mapping for a user and provider.
    pub async fn delete_by_user_and_provider<'e, E>(
        executor: E,
        user_id: i64,
        provider_id: i64,
    ) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query("DELETE FROM user_directory_identity WHERE user_id = $1 AND provider_id = $2")
            .bind(user_id)
            .bind(provider_id)
            .execute(executor)
            .await?;
        Ok(())
    }
}
