use chrono::Utc;
use defguard_common::db::{Id, setup_pool};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    query, query_scalar,
};

use crate::common::{attach_device_to_location, create_device, create_location, create_user};

const ACTIVE_SESSION_UNIQUE_INDEX: &str = "vpn_client_session_active_location_device_unique";

async fn insert_session(
    pool: &sqlx::PgPool,
    location_id: Id,
    user_id: Id,
    device_id: Id,
    state: &str,
) -> Result<Id, sqlx::Error> {
    let connected_at = (state == "connected").then(|| Utc::now().naive_utc());

    query_scalar(
        "INSERT INTO vpn_client_session (location_id, user_id, device_id, connected_at, mfa_method, state, preshared_key) \
         VALUES ($1, $2, $3, $4, NULL, $5::vpn_client_session_state, NULL) \
         RETURNING id",
    )
    .bind(location_id)
    .bind(user_id)
    .bind(device_id)
    .bind(connected_at)
    .bind(state)
    .fetch_one(pool)
    .await
}

async fn count_active_sessions(pool: &sqlx::PgPool, location_id: Id, device_id: Id) -> i64 {
    query_scalar(
        "SELECT COUNT(*) FROM vpn_client_session \
         WHERE location_id = $1 AND device_id = $2 AND state IN ('new', 'connected')",
    )
    .bind(location_id)
    .bind(device_id)
    .fetch_one(pool)
    .await
    .expect("failed to count active sessions")
}

#[sqlx::test]
async fn test_db_rejects_second_active_session_for_same_device_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    insert_session(&pool, location.id, user.id, device.id, "new")
        .await
        .expect("failed to create first active session");

    let error = insert_session(&pool, location.id, user.id, device.id, "connected")
        .await
        .expect_err("expected unique index to reject duplicate active session");

    match error {
        sqlx::Error::Database(database_error) => {
            assert_eq!(database_error.code().as_deref(), Some("23505"));
            assert_eq!(
                database_error.constraint(),
                Some(ACTIVE_SESSION_UNIQUE_INDEX)
            );
        }
        other => panic!("expected database uniqueness error, got {other:?}"),
    }
}

#[sqlx::test]
async fn test_db_allows_new_active_session_after_previous_session_disconnects(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let disconnected_session_id =
        insert_session(&pool, location.id, user.id, device.id, "connected")
            .await
            .expect("failed to create initial active session");

    query(
        "UPDATE vpn_client_session \
         SET state = 'disconnected', disconnected_at = NOW() \
         WHERE id = $1",
    )
    .bind(disconnected_session_id)
    .execute(&pool)
    .await
    .expect("failed to disconnect initial session");

    let new_session_id = insert_session(&pool, location.id, user.id, device.id, "new")
        .await
        .expect("disconnected session should not block new active session");

    assert_eq!(
        count_active_sessions(&pool, location.id, device.id).await,
        1
    );

    let active_session_id: Id = query_scalar(
        "SELECT id FROM vpn_client_session \
         WHERE location_id = $1 AND device_id = $2 AND state IN ('new', 'connected') \
         ORDER BY created_at DESC, id DESC \
         LIMIT 1",
    )
    .bind(location.id)
    .bind(device.id)
    .fetch_one(&pool)
    .await
    .expect("failed to fetch remaining active session");

    assert_eq!(active_session_id, new_session_id);
}
