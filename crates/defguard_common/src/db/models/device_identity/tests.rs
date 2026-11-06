use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::{
    DeviceAttestationChallenge, DeviceAttestationState, DeviceAttestationStatus,
    DeviceIdentityBackend, DeviceIdentityChallenge, DeviceIdentityChallengePurpose,
    DevicePossessionState,
};
use crate::db::{
    Id, NoId,
    models::{Device, DeviceType, User, WireguardNetwork},
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
}

#[sqlx::test]
async fn test_device_identity_challenge_roundtrip_and_device_binding(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let row = device(&pool, "challenge").await;
    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/30")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    for (purpose, location_id) in [
        (DeviceIdentityChallengePurpose::Polling, None),
        (
            DeviceIdentityChallengePurpose::PostureConnection,
            Some(network.id),
        ),
    ] {
        let challenge = DeviceIdentityChallenge {
            id: NoId,
            purpose,
            device_id: row.id,
            location_id,
            data: vec![0, 255, 128],
            created_at: row.created,
            expires_at: row.created + chrono::TimeDelta::minutes(5),
            consumed_at: None,
        }
        .save(&pool)
        .await
        .unwrap();
        assert!(
            DeviceIdentityChallenge::find_by_id(&pool, challenge.id)
                .await
                .unwrap()
                == Some(challenge.clone())
        );
        let error = sqlx::query("UPDATE device_identity_challenge SET purpose = CASE WHEN purpose = 'polling' THEN 'posture_connection'::challenge_purpose ELSE 'polling'::challenge_purpose END WHERE id = $1")
            .bind(challenge.id).execute(&pool).await.unwrap_err();
        assert_constraint(error, "device_identity_challenge_location");
        for (statement, constraint) in [
            (
                "UPDATE device_identity_challenge SET data = '\\x' WHERE id = $1",
                "device_identity_challenge_data",
            ),
            (
                "UPDATE device_identity_challenge SET expires_at = created_at WHERE id = $1",
                "device_identity_challenge_expiry",
            ),
        ] {
            assert_constraint(
                sqlx::query(statement)
                    .bind(challenge.id)
                    .execute(&pool)
                    .await
                    .unwrap_err(),
                constraint,
            );
        }
    }
    row.delete(&pool).await.unwrap();
    assert_eq!(DeviceIdentityChallenge::count(&pool).await.unwrap(), 0);
}

#[test]
fn test_device_identity_enrollment_state_roundtrip() {
    for backend in [
        DeviceAttestationChallenge::Tpm {
            ak_name: vec![0, 255],
            ek_fingerprint: [1; 32],
            activation_secret_hash: [2; 32],
        },
        DeviceAttestationChallenge::AppleAppAttest {
            key_id: "apple-key".into(),
        },
    ] {
        let state = DeviceAttestationState {
            challenge_id: "challenge".into(),
            data: vec![0, 255, 128],
            wireguard_key: "wg-key".into(),
            backend,
        };
        let encoded = serde_cbor::to_vec(&state).unwrap();
        let decoded: DeviceAttestationState = serde_cbor::from_slice(&encoded).unwrap();
        assert!(decoded == state);
    }
    for state in [
        DevicePossessionState::Pending {
            challenge_id: "challenge".into(),
            data: vec![0, 255, 128],
        },
        DevicePossessionState::Verified {
            key_fingerprint: [3; 32],
        },
    ] {
        let encoded = serde_json::to_vec(&state).unwrap();
        assert_eq!(
            serde_json::from_slice::<DevicePossessionState>(&encoded).unwrap(),
            state
        );
    }
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

#[sqlx::test]
async fn test_device_identity_models_crud(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let device = device(&pool, "crud").await;
    let original = DeviceIdentityChallenge {
        id: NoId,
        purpose: DeviceIdentityChallengePurpose::Polling,
        device_id: device.id,
        location_id: None,
        data: vec![0, 255, 128],
        created_at: device.created,
        expires_at: device.created + chrono::TimeDelta::minutes(5),
        consumed_at: None,
    }
    .save(&pool)
    .await
    .unwrap();
    let saved = original.clone().as_noid();
    original.clone().delete(&pool).await.unwrap();
    let mut row = saved.save(&pool).await.unwrap();
    assert!(row.id > original.id);
    row = DeviceIdentityChallenge::consume(
        &pool,
        row.id,
        device.id,
        DeviceIdentityChallengePurpose::Polling,
        None,
    )
    .await
    .unwrap()
    .unwrap();
    assert!(
        DeviceIdentityChallenge::consume(
            &pool,
            row.id,
            device.id,
            DeviceIdentityChallengePurpose::Polling,
            None
        )
        .await
        .unwrap()
        .is_none()
    );
    assert!(DeviceIdentityChallenge::all(&pool).await.unwrap() == vec![row.clone()]);
    assert!(
        DeviceIdentityChallenge::all_paginated(&pool, 1, 0)
            .await
            .unwrap()
            == vec![row.clone()]
    );
    row.delete(&pool).await.unwrap();
    assert_eq!(DeviceIdentityChallenge::count(&pool).await.unwrap(), 0);
}
