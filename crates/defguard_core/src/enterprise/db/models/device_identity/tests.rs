use chrono::SubsecRound;
use defguard_common::db::{
    Id, NoId,
    models::{Device, DeviceType, User, WireguardNetwork},
    setup_pool,
};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::{
    DeviceAttestationChallenge, DeviceAttestationPayload, DeviceAttestationState,
    DeviceIdentityChallenge, DeviceIdentityChallengePurpose,
};
use crate::db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token};

fn assert_constraint(error: sqlx::Error, name: &str) {
    assert_eq!(error.as_database_error().unwrap().constraint(), Some(name));
}

#[test]
fn test_attestation_payload_roundtrip() {
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
        let state = DeviceAttestationPayload {
            data: vec![0, 255, 128],
            backend,
        };
        let encoded = serde_cbor::to_vec(&state).unwrap();
        let decoded: DeviceAttestationPayload = serde_cbor::from_slice(&encoded).unwrap();
        assert!(decoded == state);
    }
}

async fn enrollment_attestation(
    pool: &PgPool,
    name: &str,
    backend: DeviceAttestationChallenge,
) -> DeviceAttestationState {
    let user = User::new(
        name,
        None,
        "First",
        "Last",
        &format!("{name}@example.com"),
        None,
    )
    .save(pool)
    .await
    .unwrap();
    let now = chrono::Utc::now().naive_utc().trunc_subsecs(6);
    let mut token = Token::new(
        user.id,
        None,
        None,
        3600,
        Some(ENROLLMENT_TOKEN_TYPE.into()),
    );
    token.used_at = Some(now);
    token.save(pool).await.unwrap();
    DeviceAttestationState {
        challenge_id: format!("challenge-{name}"),
        token_id: token.id,
        wireguard_key: format!("key-{name}"),
        created_at: now,
        expires_at: now + chrono::TimeDelta::minutes(10),
        payload: DeviceAttestationPayload {
            data: vec![0, 255, 128],
            backend,
        },
    }
}

fn tpm_challenge() -> DeviceAttestationChallenge {
    DeviceAttestationChallenge::Tpm {
        ak_name: vec![0, 255],
        ek_fingerprint: [1; 32],
        activation_secret_hash: [2; 32],
    }
}

#[sqlx::test]
async fn test_attestation_state_roundtrips_backend_payloads(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    for (name, backend) in [
        ("attestation-tpm", tpm_challenge()),
        (
            "attestation-apple",
            DeviceAttestationChallenge::AppleAppAttest {
                key_id: "apple-key".into(),
            },
        ),
    ] {
        let state = enrollment_attestation(&pool, name, backend).await;
        state.save(&pool).await.unwrap();
        assert!(
            DeviceAttestationState::find_by_token_id(&pool, &state.token_id)
                .await
                .unwrap()
                == Some(state)
        );
    }
}

#[sqlx::test]
async fn test_attestation_state_replacement_and_binding(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let original = enrollment_attestation(&pool, "replacement", tpm_challenge()).await;
    original.save(&pool).await.unwrap();
    let other = enrollment_attestation(&pool, "other-session", tpm_challenge()).await;
    other.save(&pool).await.unwrap();
    let mut current = original.clone();
    current.challenge_id = "new-challenge".into();
    current.save(&pool).await.unwrap();
    assert!(!original.consume(&pool).await.unwrap());
    let previous = current.clone();
    current.payload.data.push(1);
    current.save(&pool).await.unwrap();
    assert!(!previous.consume(&pool).await.unwrap());
    let mut wrong_token = current.clone();
    wrong_token.token_id.clone_from(&other.token_id);
    assert!(!wrong_token.consume(&pool).await.unwrap());
    let mut wrong_key = current.clone();
    wrong_key.wireguard_key = "another-wg-key".into();
    assert!(!wrong_key.consume(&pool).await.unwrap());
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &original.token_id)
            .await
            .unwrap()
            == Some(current.clone())
    );
    assert!(current.consume(&pool).await.unwrap());
    assert!(!current.consume(&pool).await.unwrap());
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &original.token_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &other.token_id)
            .await
            .unwrap()
            == Some(other)
    );
}

#[sqlx::test]
async fn test_attestation_state_expiry_rollback_and_concurrent_consumption(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let mut state = enrollment_attestation(&pool, "expiry", tpm_challenge()).await;
    state.created_at -= chrono::TimeDelta::minutes(20);
    state.expires_at = state.created_at + chrono::TimeDelta::minutes(10);
    state.save(&pool).await.unwrap();
    assert!(!state.consume(&pool).await.unwrap());
    state.created_at = chrono::Utc::now().naive_utc().trunc_subsecs(6);
    state.expires_at = state.created_at + chrono::TimeDelta::minutes(10);
    state.save(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    assert!(state.consume(&mut *tx).await.unwrap());
    tx.rollback().await.unwrap();
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &state.token_id)
            .await
            .unwrap()
            == Some(state.clone())
    );
    let (first, second) = tokio::join!(state.consume(&pool), state.consume(&pool));
    assert!(matches!(
        (first.unwrap(), second.unwrap()),
        (true, false) | (false, true)
    ));
}

#[sqlx::test]
async fn test_attestation_state_constraints_and_token_cleanup(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let state = enrollment_attestation(&pool, "constraints-state", tpm_challenge()).await;
    state.save(&pool).await.unwrap();
    for (statement, constraint) in [
        (
            "UPDATE device_attestation_state SET challenge_id = '' WHERE token_id = $1",
            "device_attestation_state_challenge_id",
        ),
        (
            "UPDATE device_attestation_state SET wireguard_key = '' WHERE token_id = $1",
            "device_attestation_state_wireguard_key",
        ),
        (
            "UPDATE device_attestation_state SET payload = '\\x' WHERE token_id = $1",
            "device_attestation_state_payload",
        ),
        (
            "UPDATE device_attestation_state SET expires_at = created_at WHERE token_id = $1",
            "device_attestation_state_expiry",
        ),
    ] {
        assert_constraint(
            sqlx::query(statement)
                .bind(&state.token_id)
                .execute(&pool)
                .await
                .unwrap_err(),
            constraint,
        );
    }
    let mut missing_token = state.clone();
    missing_token.challenge_id = "orphan".into();
    missing_token.token_id = "missing-token".into();
    assert_constraint(
        missing_token.save(&pool).await.unwrap_err(),
        "device_attestation_state_token_id_fkey",
    );
    sqlx::query("UPDATE device_attestation_state SET payload = $1 WHERE token_id = $2")
        .bind(vec![1u8])
        .bind(&state.token_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        DeviceAttestationState::find_by_token_id(&pool, &state.token_id)
            .await
            .err(),
        Some(sqlx::Error::ColumnDecode { .. })
    ));
    sqlx::query("DELETE FROM token WHERE id = $1")
        .bind(&state.token_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &state.token_id)
            .await
            .unwrap()
            .is_none()
    );
}

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
