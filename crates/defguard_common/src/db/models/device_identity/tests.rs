use serde_json::json;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use uuid::Uuid;

use super::{
    DeviceAttestationStatus, DeviceIdentity, DeviceIdentityBackend, DeviceIdentityChallenge,
    DeviceIdentityChallengePurpose, DeviceIdentityErrorCode,
};
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

async fn identity(pool: &PgPool, device_id: Id, backend: DeviceIdentityBackend) -> DeviceIdentity {
    sqlx::query_as(
        "INSERT INTO device_identity (id, device_id, backend) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(Uuid::new_v4())
    .bind(device_id)
    .bind(backend)
    .fetch_one(pool)
    .await
    .unwrap()
}

fn assert_constraint(error: sqlx::Error, name: &str) {
    let database = error
        .as_database_error()
        .expect("expected a PostgreSQL constraint violation");
    assert_eq!(database.constraint(), Some(name));
}

#[sqlx::test]
async fn test_device_identity_defaults_and_backend_roundtrips(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let tpm_device = device(&pool, "tpm").await;
    let apple_device = device(&pool, "apple").await;
    for (device, backend) in [
        (tpm_device, DeviceIdentityBackend::Tpm),
        (apple_device, DeviceIdentityBackend::AppleAppAttest),
    ] {
        let row = identity(&pool, device.id, backend).await;
        assert_eq!(row.backend, backend);
        assert_eq!(row.attestation_status, DeviceAttestationStatus::NotAttested);
        assert!(row.public_key_der.is_none());
        assert!(row.attested_at.is_none());
        assert!(row.last_possession_verified_at.is_none());
        assert!(row.error_code.is_none());

        let updated: DeviceIdentity = sqlx::query_as(
            "UPDATE device_identity SET public_key_der = $2, key_fingerprint = $3, \
             capabilities = $4, attestation_evidence = $5, attestation_status = $6, \
             attested_at = created_at, error_code = $7, \
             apple_key_id = $8, apple_sign_count = $9, ek_fingerprint = $10 \
             WHERE id = $1 RETURNING *",
        )
        .bind(row.id)
        .bind(vec![0u8, 255])
        .bind(vec![1u8; 32])
        .bind(json!({"algorithms": [{"id": 65535, "flags": 2147483648u32}]}))
        .bind(b"encrypted-evidence-fixture".as_slice())
        .bind(DeviceAttestationStatus::Attested)
        .bind(Some(DeviceIdentityErrorCode::TemporarilyUnavailable))
        .bind((backend == DeviceIdentityBackend::AppleAppAttest).then_some("apple-key"))
        .bind((backend == DeviceIdentityBackend::AppleAppAttest).then_some(4_294_967_295i64))
        .bind((backend == DeviceIdentityBackend::Tpm).then_some(vec![2u8; 32]))
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(updated.public_key_der, Some(vec![0, 255]));
        assert_eq!(
            updated.attestation_status,
            DeviceAttestationStatus::Attested
        );
        assert_eq!(
            updated.error_code,
            Some(DeviceIdentityErrorCode::TemporarilyUnavailable)
        );
        assert_eq!(updated.capabilities.unwrap()["algorithms"][0]["id"], 65535);
        assert_eq!(
            updated.attestation_evidence.as_deref(),
            Some(b"encrypted-evidence-fixture".as_slice())
        );
    }
}

#[sqlx::test]
async fn test_device_identity_constraints(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let target_device = device(&pool, "constraints").await;
    let row = identity(&pool, target_device.id, DeviceIdentityBackend::Tpm).await;
    let duplicate =
        sqlx::query("INSERT INTO device_identity (id, device_id, backend) VALUES ($1, $2, 'tpm')")
            .bind(Uuid::new_v4())
            .bind(target_device.id)
            .execute(&pool)
            .await
            .unwrap_err();
    assert_constraint(duplicate, "device_identity_device_id_key");

    for (statement, constraint) in [
        (
            "UPDATE device_identity SET public_key_der = '\\x01', key_fingerprint = '\\x01' WHERE id = $1",
            "device_identity_key_fingerprint_length",
        ),
        (
            "UPDATE device_identity SET ek_fingerprint = '\\x01' WHERE id = $1",
            "device_identity_ek_fingerprint_length",
        ),
        (
            "UPDATE device_identity SET attestation_status = 'attested' WHERE id = $1",
            "device_identity_attested_key",
        ),
        (
            "UPDATE device_identity SET apple_key_id = 'apple-key' WHERE id = $1",
            "device_identity_backend_fields",
        ),
    ] {
        let error = sqlx::query(statement)
            .bind(row.id)
            .execute(&pool)
            .await
            .unwrap_err();
        assert_constraint(error, constraint);
    }

    let apple_device = device(&pool, "apple-constraints").await;
    let apple = identity(
        &pool,
        apple_device.id,
        DeviceIdentityBackend::AppleAppAttest,
    )
    .await;
    for counter in [-1i64, 4_294_967_296] {
        let error = sqlx::query("UPDATE device_identity SET apple_sign_count = $2 WHERE id = $1")
            .bind(apple.id)
            .bind(counter)
            .execute(&pool)
            .await
            .unwrap_err();
        assert_constraint(error, "device_identity_apple_counter_range");
    }
}

#[sqlx::test]
async fn test_device_identity_challenge_roundtrip_and_device_binding(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let first = device(&pool, "first").await;
    let second = device(&pool, "second").await;
    let identity = identity(&pool, first.id, DeviceIdentityBackend::Tpm).await;
    let challenge_id = Uuid::new_v4();
    let challenge: DeviceIdentityChallenge = sqlx::query_as(
        "INSERT INTO device_identity_challenge \
         (id, purpose, device_id, identity_id, data, expires_at) \
         VALUES ($1, 'possession', $2, $3, $4, (now() AT TIME ZONE 'UTC') + interval '5 minutes') \
         RETURNING *",
    )
    .bind(challenge_id)
    .bind(first.id)
    .bind(identity.id)
    .bind(vec![0u8, 255, 128])
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        challenge.purpose,
        DeviceIdentityChallengePurpose::Possession
    );
    assert_eq!(challenge.identity_id, Some(identity.id));
    assert_eq!(challenge.data, vec![0, 255, 128]);
    assert!(challenge.consumed_at.is_none());
    assert!(challenge.activation_secret_hash.is_none());
    assert!(challenge.expires_at > challenge.created_at);

    let error = sqlx::query("UPDATE device_identity_challenge SET device_id = $2 WHERE id = $1")
        .bind(challenge_id)
        .bind(second.id)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_constraint(error, "device_identity_challenge_identity_device_fkey");

    let consumed: DeviceIdentityChallenge = sqlx::query_as(
        "UPDATE device_identity_challenge SET consumed_at = created_at WHERE id = $1 RETURNING *",
    )
    .bind(challenge_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(consumed.consumed_at, Some(consumed.created_at));

    sqlx::query("DELETE FROM device WHERE id = $1")
        .bind(first.id)
        .execute(&pool)
        .await
        .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM device_identity_challenge WHERE id = $1")
            .bind(challenge_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM device_identity WHERE id = $1")
        .bind(identity.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test]
async fn test_device_identity_enrollment_challenge_constraints(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let device = device(&pool, "enrollment").await;
    let token = "test-enrollment-token";
    sqlx::query(
        "INSERT INTO token (id, user_id, admin_id, created_at, expires_at) \
         VALUES ($1, $2, $2, now(), now() + interval '10 minutes')",
    )
    .bind(token)
    .bind(device.user_id)
    .execute(&pool)
    .await
    .unwrap();
    let challenge_id = Uuid::new_v4();
    let challenge: DeviceIdentityChallenge = sqlx::query_as(
        "INSERT INTO device_identity_challenge \
         (id, purpose, enrollment_token_id, data, credential_blob, encrypted_secret, activation_secret_hash, expires_at) \
         VALUES ($1, 'attestation', $2, $3, $4, $5, $6, (now() AT TIME ZONE 'UTC') + interval '5 minutes') \
         RETURNING *",
    ).bind(challenge_id).bind(token).bind(vec![0u8, 255]).bind(vec![1u8, 255])
        .bind(vec![2u8, 128]).bind(vec![3u8; 32]).fetch_one(&pool).await.unwrap();
    assert_eq!(
        challenge.purpose,
        DeviceIdentityChallengePurpose::Attestation
    );
    assert_eq!(challenge.enrollment_token_id.as_deref(), Some(token));
    assert_eq!(challenge.activation_secret_hash, Some(vec![3; 32]));
    assert!(challenge.device_id.is_none());

    for (statement, constraint) in [
        (
            "UPDATE device_identity_challenge SET enrollment_token_id = NULL WHERE id = $1",
            "device_identity_challenge_owner",
        ),
        (
            "UPDATE device_identity_challenge SET encrypted_secret = NULL WHERE id = $1",
            "device_identity_challenge_credential_fields",
        ),
        (
            "UPDATE device_identity_challenge SET activation_secret_hash = '\\x01' WHERE id = $1",
            "device_identity_challenge_secret_hash_length",
        ),
        (
            "UPDATE device_identity_challenge SET expires_at = created_at WHERE id = $1",
            "device_identity_challenge_expiry",
        ),
        (
            "UPDATE device_identity_challenge SET data = '\\x' WHERE id = $1",
            "device_identity_challenge_data",
        ),
    ] {
        let error = sqlx::query(statement)
            .bind(challenge_id)
            .execute(&pool)
            .await
            .unwrap_err();
        assert_constraint(error, constraint);
    }
    sqlx::query("DELETE FROM token WHERE id = $1")
        .bind(token)
        .execute(&pool)
        .await
        .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM device_identity_challenge WHERE id = $1")
            .bind(challenge_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test]
async fn test_device_identity_migration_is_reversible(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let device = device(&pool, "migration").await;
    let mut transaction = pool.begin().await.unwrap();
    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/20261030123121_[2.3.0]_device_identity.down.sql"
    )))
    .execute(&mut *transaction)
    .await
    .unwrap();
    let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('device_identity')::text")
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    assert!(table.is_none());
    let table: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('device_identity_challenge')::text")
            .fetch_one(&mut *transaction)
            .await
            .unwrap();
    assert!(table.is_none());

    sqlx::raw_sql(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../migrations/20261030123121_[2.3.0]_device_identity.up.sql"
    )))
    .execute(&mut *transaction)
    .await
    .unwrap();
    let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('device_identity')::text")
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    assert!(table.is_some());
    let preserved: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM device WHERE id = $1)")
        .bind(device.id)
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    assert!(preserved);
    transaction.rollback().await.unwrap();
}
