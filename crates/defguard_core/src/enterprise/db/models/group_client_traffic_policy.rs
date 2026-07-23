use defguard_common::db::Id;
use sqlx::{FromRow, PgExecutor, query, query_as};

use super::enterprise_settings::ClientTrafficPolicy;

#[derive(Clone, Debug, FromRow, PartialEq)]
pub struct GroupClientTrafficPolicy {
    pub group_id: Id,
    pub client_traffic_policy: ClientTrafficPolicy,
}

impl GroupClientTrafficPolicy {
    pub async fn find_by_user_id<'e, E>(executor: E, user_id: Id) -> sqlx::Result<Vec<Self>>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT gctp.group_id, \
                gctp.client_traffic_policy \"client_traffic_policy: ClientTrafficPolicy\" \
             FROM group_client_traffic_policy gctp \
             JOIN group_user gu ON gu.group_id = gctp.group_id \
             WHERE gu.user_id = $1",
            user_id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn upsert<'e, E>(
        executor: E,
        group_id: Id,
        policy: ClientTrafficPolicy,
    ) -> sqlx::Result<Self>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "INSERT INTO group_client_traffic_policy \
                (group_id, client_traffic_policy) \
             VALUES ($1, $2) \
             ON CONFLICT (group_id) DO UPDATE SET \
                client_traffic_policy = EXCLUDED.client_traffic_policy \
             RETURNING group_id, \
                client_traffic_policy \"client_traffic_policy: ClientTrafficPolicy\"",
            group_id,
            policy as ClientTrafficPolicy
        )
        .fetch_one(executor)
        .await
    }

    pub async fn delete<'e, E>(executor: E, group_id: Id) -> sqlx::Result<()>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "DELETE FROM group_client_traffic_policy WHERE group_id = $1",
            group_id
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}
