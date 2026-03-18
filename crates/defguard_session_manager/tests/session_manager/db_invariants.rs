use chrono::Utc;
use defguard_common::db::{Id, setup_pool};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    query, query_scalar,
};

use crate::common::{attach_device_to_location, create_device, create_location, create_user};

const ACTIVE_SESSION_UNIQUE_INDEX: &str = "vpn_client_session_active_location_device_unique";
const PRESHARED_KEY_MIGRATION_SQL: &str = include_str!(
    "../../../../migrations/20260317120000_[2.0.0]_vpn_client_session_preshared_key.up.sql"
);

fn extract_migration_statement(start_marker: &str, end_marker: &str) -> &'static str {
    let Some(start_offset) = PRESHARED_KEY_MIGRATION_SQL.find(start_marker) else {
        panic!("migration SQL is missing expected start marker: {start_marker}");
    };

    let migration_from_start = &PRESHARED_KEY_MIGRATION_SQL[start_offset..];
    let Some(end_offset) = migration_from_start.find(end_marker) else {
        panic!("migration SQL is missing expected end marker: {end_marker}");
    };

    &migration_from_start[..end_offset + end_marker.len()]
}

fn duplicate_active_session_cleanup_sql() -> &'static str {
    extract_migration_statement(
        "WITH ranked_active_sessions AS (",
        "  AND ranked_session.rank > 1;",
    )
}

fn create_active_session_unique_index_sql() -> &'static str {
    extract_migration_statement(
        "CREATE UNIQUE INDEX vpn_client_session_active_location_device_unique",
        "    WHERE state IN ('new', 'connected');",
    )
}

fn active_mfa_session_precondition_sql() -> &'static str {
    extract_migration_statement("DO $$", "END $$;")
}

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
async fn test_migration_cleanup_keeps_newest_active_session_before_unique_index(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    query("DROP INDEX IF EXISTS vpn_client_session_active_location_device_unique")
        .execute(&pool)
        .await
        .expect("failed to drop active-session unique index");

    let older_session_id = insert_session(&pool, location.id, user.id, device.id, "new")
        .await
        .expect("failed to create older active session");
    let newer_session_id = insert_session(&pool, location.id, user.id, device.id, "connected")
        .await
        .expect("failed to create newer active session");

    query(duplicate_active_session_cleanup_sql())
        .execute(&pool)
        .await
        .expect("failed to run duplicate-session cleanup");

    query(create_active_session_unique_index_sql())
        .execute(&pool)
        .await
        .expect("failed to recreate active-session unique index");

    assert_eq!(
        count_active_sessions(&pool, location.id, device.id).await,
        1
    );

    let remaining_active_session_id: Id = query_scalar(
        "SELECT id FROM vpn_client_session
         WHERE location_id = $1 AND device_id = $2 AND state IN ('new', 'connected')
         ORDER BY created_at DESC, id DESC
         LIMIT 1",
    )
    .bind(location.id)
    .bind(device.id)
    .fetch_one(&pool)
    .await
    .expect("failed to fetch remaining active session");

    assert_eq!(remaining_active_session_id, newer_session_id);

    let disconnected_at = query_scalar::<_, Option<chrono::NaiveDateTime>>(
        "SELECT disconnected_at FROM vpn_client_session WHERE id = $1",
    )
    .bind(older_session_id)
    .fetch_one(&pool)
    .await
    .expect("failed to fetch disconnected_at for older session");

    assert!(disconnected_at.is_some());
}

#[sqlx::test]
async fn test_migration_precondition_rejects_active_mfa_sessions(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    query_scalar::<_, Id>(
        "INSERT INTO vpn_client_session (location_id, user_id, device_id, connected_at, mfa_method, state, preshared_key)
         VALUES ($1, $2, $3, NULL, 'totp'::vpn_client_mfa_method, 'new'::vpn_client_session_state, NULL)
         RETURNING id",
    )
    .bind(location.id)
    .bind(user.id)
    .bind(device.id)
    .fetch_one(&pool)
    .await
    .expect("failed to create active MFA session");

    let error = query(active_mfa_session_precondition_sql())
        .execute(&pool)
        .await
        .expect_err("expected migration precondition to reject active MFA sessions");

    match error {
        sqlx::Error::Database(database_error) => {
            assert!(
                database_error
                    .message()
                    .contains("Active MFA VPN sessions must be disconnected before migration")
            );
        }
        other => panic!("expected migration precondition error, got {other:?}"),
    }
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
