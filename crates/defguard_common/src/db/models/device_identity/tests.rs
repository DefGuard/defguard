use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::DeviceAttestationStatus;
use crate::db::{
    Id,
    models::{Device, DeviceType, User},
    setup_pool,
};

async fn device(pool: &PgPool, name: &str) -> Device<Id> {
    let user = User::new(
        name,
        None,
        "Last",
        "First",
        &format!("{name}@example.com"),
        None,
    )
    .save(pool)
    .await
    .unwrap();
    Device::new(
        name.into(),
        format!("key-{name}"),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn test_device_attestation_status_roundtrip(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut row = device(&pool, "status").await;
    assert_eq!(row.attestation_status, DeviceAttestationStatus::NotAttested);
    assert!(row.attested_at.is_none());
    row.attestation_status = DeviceAttestationStatus::AttestationFailed;
    row.save(&pool).await.unwrap();
    assert_eq!(Device::find_by_id(&pool, row.id).await.unwrap(), Some(row));
    assert_eq!(
        serde_json::to_value(DeviceAttestationStatus::AttestationFailed).unwrap(),
        "attestation_failed"
    );
}

#[sqlx::test]
async fn test_device_identity_migration_is_reversible(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let mut row = device(&pool, "migration").await;
    row.attestation_status = DeviceAttestationStatus::Unrecognized;
    row.attested_at = Some(row.created);
    row.save(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/20261110111514_[2.3.0]_device_identity_verification.down.sql"
    )))
    .execute(&mut *tx)
    .await
    .unwrap();
    let legacy_fields: (String, Option<chrono::NaiveDateTime>) =
        sqlx::query_as("SELECT attestation_status::text, attested_at FROM device WHERE id = $1")
            .bind(row.id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(legacy_fields, ("unrecognized".into(), row.attested_at));
    let table: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('device_identity_challenge')::text")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(table.is_none());
    let table: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('device_attestation_state')::text")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert!(table.is_none());
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/20261110111514_[2.3.0]_device_identity_verification.up.sql"
    )))
    .execute(&mut *tx)
    .await
    .unwrap();
    assert_eq!(
        Device::find_by_id(&mut *tx, row.id).await.unwrap(),
        Some(row)
    );
    let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('device_identity')::text")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert!(table.is_some());
    tx.rollback().await.unwrap();
}
