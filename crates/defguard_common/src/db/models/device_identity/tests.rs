use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::{DeviceAttestationStatus, DeviceIdentityBackend};
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

fn assert_constraint(error: sqlx::Error, name: &str) {
    assert_eq!(error.as_database_error().unwrap().constraint(), Some(name));
}

#[sqlx::test]
async fn test_device_identity_defaults_and_backend_roundtrips(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    for (name, backend) in [
        ("tpm", DeviceIdentityBackend::Tpm),
        ("apple", DeviceIdentityBackend::AppleAppAttest),
    ] {
        let mut row = device(&pool, name).await;
        assert_eq!(row.attestation_status, DeviceAttestationStatus::NotAttested);
        assert!(row.identity_backend.is_none());
        assert!(row.identity_public_key_der.is_none());
        assert!(row.attested_at.is_none());
        assert!(row.tpm_ek_public.is_none());
        assert!(row.tpm_ek_certificate_chain_der.is_none());
        assert!(row.apple_key_id.is_none());
        row.attestation_status = DeviceAttestationStatus::AttestationFailed;
        row.save(&pool).await.unwrap();
        assert_eq!(
            Device::find_by_id(&pool, row.id)
                .await
                .unwrap()
                .unwrap()
                .attestation_status,
            DeviceAttestationStatus::AttestationFailed
        );
        assert_eq!(
            serde_json::to_value(row.attestation_status).unwrap(),
            "attestation_failed"
        );
        row.identity_backend = Some(backend);
        row.identity_public_key_der = Some(vec![0, 255, 128]);
        row.attestation_status = DeviceAttestationStatus::Attested;
        row.attested_at = Some(row.created);
        row.apple_sign_count =
            (backend == DeviceIdentityBackend::AppleAppAttest).then_some(4_294_967_295);
        match backend {
            DeviceIdentityBackend::Tpm => {
                row.tpm_ek_public = Some(vec![0, 255, 128]);
                row.tpm_ek_certificate_chain_der = Some(vec![vec![0, 255], vec![128, 1]]);
            }
            DeviceIdentityBackend::AppleAppAttest => {
                row.apple_key_id = Some("apple-key-id".into());
            }
        }
        row.save(&pool).await.unwrap();
        assert_eq!(Device::find_by_id(&pool, row.id).await.unwrap(), Some(row));
    }
}

#[sqlx::test]
async fn test_device_identity_constraints(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let row = device(&pool, "constraints").await;
    for (statement, constraint) in [
        (
            "UPDATE device SET attestation_status = 'attested' WHERE id = $1",
            "device_identity_attested_key",
        ),
        (
            "UPDATE device SET identity_public_key_der = '\\x01' WHERE id = $1",
            "device_identity_backend_fields",
        ),
        (
            "UPDATE device SET identity_backend = 'tpm', apple_sign_count = 1 WHERE id = $1",
            "device_identity_backend_fields",
        ),
        (
            "UPDATE device SET identity_backend = 'tpm', apple_key_id = 'apple-key' WHERE id = $1",
            "device_identity_backend_fields",
        ),
        (
            "UPDATE device SET identity_backend = 'apple_app_attest', tpm_ek_public = '\\x01' WHERE id = $1",
            "device_identity_backend_fields",
        ),
        (
            "UPDATE device SET identity_backend = 'apple_app_attest', tpm_ek_certificate_chain_der = ARRAY['\\x01'::bytea] WHERE id = $1",
            "device_identity_backend_fields",
        ),
        (
            "UPDATE device SET identity_backend = 'tpm', identity_public_key_der = '\\x01', attestation_status = 'attested', attested_at = created WHERE id = $1",
            "device_identity_attested_key",
        ),
        (
            "UPDATE device SET identity_backend = 'apple_app_attest', identity_public_key_der = '\\x01', attestation_status = 'attested', attested_at = created WHERE id = $1",
            "device_identity_attested_key",
        ),
        (
            "UPDATE device SET identity_backend = 'apple_app_attest', apple_sign_count = -1 WHERE id = $1",
            "device_identity_apple_counter_range",
        ),
        (
            "UPDATE device SET identity_backend = 'apple_app_attest', apple_sign_count = 4294967296 WHERE id = $1",
            "device_identity_apple_counter_range",
        ),
    ] {
        let error = sqlx::query(statement)
            .bind(row.id)
            .execute(&pool)
            .await
            .unwrap_err();
        assert_constraint(error, constraint);
    }
    let mut manually_attested = row;
    manually_attested.identity_backend = Some(DeviceIdentityBackend::Tpm);
    manually_attested.identity_public_key_der = Some(vec![1]);
    manually_attested.attestation_status = DeviceAttestationStatus::AttestedManually;
    manually_attested.attested_at = Some(manually_attested.created);
    manually_attested.save(&pool).await.unwrap();
    assert_eq!(
        Device::find_by_id(&pool, manually_attested.id)
            .await
            .unwrap(),
        Some(manually_attested)
    );
}

#[sqlx::test]
async fn test_device_identity_migration_is_reversible(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let row = device(&pool, "migration").await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/20261030123121_[2.3.0]_device_identity.down.sql"
    )))
    .execute(&mut *tx)
    .await
    .unwrap();
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
        "/../../migrations/20261030123121_[2.3.0]_device_identity.up.sql"
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
    assert!(table.is_none());
    tx.rollback().await.unwrap();
}
