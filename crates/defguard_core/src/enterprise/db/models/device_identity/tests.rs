use chrono::SubsecRound;
use defguard_common::db::{
    Id, NoId,
    models::{
        Device, DeviceType, User, WireguardNetwork,
        device_identity::DeviceAttestationStatus,
        vpn_client_mfa_session::{VpnClientMfaSession, VpnMfaFlowKind},
        vpn_client_session::VpnClientMfaMethod,
    },
    setup_pool,
};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::{
    DeviceAttestationChallenge, DeviceAttestationPayload, DeviceAttestationState, DeviceIdentity,
    DeviceIdentityBackend, DeviceIdentityChallenge, DeviceIdentityChallengePurpose,
    DeviceIdentityError,
};
use crate::db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token};

fn assert_constraint(error: sqlx::Error, name: &str) {
    assert_eq!(error.as_database_error().unwrap().constraint(), Some(name));
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
                == Some(state.clone())
        );
        sqlx::query("UPDATE device_attestation_state SET payload = $1 WHERE token_id = $2")
            .bind(vec![1u8])
            .bind(&state.token_id)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            DeviceAttestationState::find_by_token_id(&pool, &state.token_id)
                .await
                .is_err()
        );
    }
}

#[sqlx::test]
async fn test_attestation_state_replacement_preserves_other_sessions(
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
    let mut wrong_token = current.clone();
    wrong_token.token_id.clone_from(&other.token_id);
    assert!(!wrong_token.consume(&pool).await.unwrap());
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
async fn test_expired_attestation_state_cannot_be_consumed(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let mut state = enrollment_attestation(&pool, "expiry", tpm_challenge()).await;
    state.created_at -= chrono::TimeDelta::minutes(20);
    state.expires_at = state.created_at + chrono::TimeDelta::minutes(10);
    state.save(&pool).await.unwrap();
    assert!(!state.consume(&pool).await.unwrap());
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &state.token_id)
            .await
            .unwrap()
            == Some(state)
    );
}

#[sqlx::test]
async fn test_attestation_state_consumption_is_rolled_back(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let state = enrollment_attestation(&pool, "rollback", tpm_challenge()).await;
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
    assert!(state.consume(&pool).await.unwrap());
}

#[sqlx::test]
async fn test_attestation_state_is_consumed_once_under_concurrency(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let state = enrollment_attestation(&pool, "concurrent", tpm_challenge()).await;
    state.save(&pool).await.unwrap();
    let (first, second) = tokio::join!(state.consume(&pool), state.consume(&pool));
    assert!(matches!(
        (first.unwrap(), second.unwrap()),
        (true, false) | (false, true)
    ));
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &state.token_id)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn test_attestation_state_is_removed_with_its_enrollment_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let state = enrollment_attestation(&pool, "token-cleanup", tpm_challenge()).await;
    state.save(&pool).await.unwrap();
    let token = Token::find_by_id(&pool, &state.token_id).await.unwrap();
    Token::delete_user_tokens_of_type(&pool, token.user_id, ENROLLMENT_TOKEN_TYPE)
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
async fn test_polling_and_posture_challenges_enforce_scope_expiry_and_single_use(
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
            mfa_session_id: None,
            mfa_session_token_hash: None,
            data: vec![0, 255, 128],
            created_at: row.created - chrono::TimeDelta::minutes(10),
            expires_at: row.created + chrono::TimeDelta::minutes(5),
            consumed_at: None,
        }
        .save(&pool)
        .await
        .unwrap();
        let other_purpose = match purpose {
            DeviceIdentityChallengePurpose::Polling => {
                DeviceIdentityChallengePurpose::PostureConnection
            }
            _ => DeviceIdentityChallengePurpose::Polling,
        };
        for (device_id, expected_purpose, expected_location) in [
            (row.id + 1, purpose, location_id),
            (row.id, other_purpose, location_id),
            (row.id, purpose, Some(network.id + 1)),
        ] {
            assert!(
                DeviceIdentityChallenge::consume(
                    &pool,
                    challenge.id,
                    device_id,
                    expected_purpose,
                    expected_location,
                    None
                )
                .await
                .unwrap()
                .is_none()
            );
        }
        let mut expired = challenge.clone();
        expired.expires_at = row.created - chrono::TimeDelta::minutes(1);
        expired.save(&pool).await.unwrap();
        assert!(
            DeviceIdentityChallenge::consume(
                &pool,
                challenge.id,
                row.id,
                purpose,
                location_id,
                None
            )
            .await
            .unwrap()
            .is_none()
        );
        challenge.clone().save(&pool).await.unwrap();
        let (first, second) = tokio::join!(
            DeviceIdentityChallenge::consume(
                &pool,
                challenge.id,
                row.id,
                purpose,
                location_id,
                None
            ),
            DeviceIdentityChallenge::consume(
                &pool,
                challenge.id,
                row.id,
                purpose,
                location_id,
                None
            ),
        );
        let consumed = match (first.unwrap(), second.unwrap()) {
            (Some(consumed), None) | (None, Some(consumed)) => consumed,
            _ => panic!("expected exactly one successful consumption"),
        };
        assert_eq!(consumed.data, vec![0, 255, 128]);
        assert!(consumed.consumed_at.is_some());
        assert!(
            DeviceIdentityChallenge::consume(
                &pool,
                challenge.id,
                row.id,
                purpose,
                location_id,
                None
            )
            .await
            .unwrap()
            .is_none()
        );
    }
    row.delete(&pool).await.unwrap();
    assert_eq!(DeviceIdentityChallenge::count(&pool).await.unwrap(), 0);
}

async fn mfa_session(pool: &PgPool, name: &str) -> (VpnClientMfaSession<Id>, String) {
    let owner = device(pool, name).await;
    let location = WireguardNetwork::default()
        .try_set_address("10.9.1.1/24")
        .unwrap()
        .save(pool)
        .await
        .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    let (session, outcome) = VpnClientMfaSession::start(
        &mut connection,
        location.id,
        owner.id,
        owner.user_id,
        1,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
        VpnMfaFlowKind::MultiStep,
        VpnClientMfaMethod::Totp,
        None,
        std::time::Duration::from_mins(10),
    )
    .await
    .unwrap();
    (session, outcome.token)
}

#[sqlx::test]
async fn test_mfa_possession_records_verification_for_the_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (session, token) = mfa_session(&pool, "mfa-proof").await;
    assert!(!session.possession_verified);
    let challenge = DeviceIdentityChallenge::for_mfa(&session, vec![0, 255, 128])
        .save(&pool)
        .await
        .unwrap();
    assert_eq!(
        challenge.purpose,
        DeviceIdentityChallengePurpose::MfaConnection
    );
    assert_eq!(challenge.mfa_session_id, Some(session.id));
    assert_eq!(
        challenge.mfa_session_token_hash.as_deref(),
        Some(session.token_hash.as_str())
    );
    assert_eq!(challenge.expires_at, session.expires_at);
    let mut connection = pool.acquire().await.unwrap();
    assert!(
        DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            challenge.id,
            &session
        )
        .await
        .unwrap()
    );
    assert!(
        !DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            challenge.id,
            &session
        )
        .await
        .unwrap()
    );
    let verified = VpnClientMfaSession::find_active_by_token(&pool, &token)
        .await
        .unwrap()
        .unwrap();
    assert!(verified.possession_verified);
    assert!(
        verified
            .advance(&mut connection, 0, None, VpnClientMfaMethod::Totp, None)
            .await
            .unwrap()
            .is_some()
    );
    let next_step = VpnClientMfaSession::find_active_by_token(&pool, &token)
        .await
        .unwrap()
        .unwrap();
    assert!(next_step.possession_verified);
    assert!(
        DeviceIdentityChallenge::find_by_id(&pool, challenge.id)
            .await
            .unwrap()
            .unwrap()
            .consumed_at
            .is_some()
    );
}

#[sqlx::test]
async fn test_mfa_possession_rejects_wrong_context(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (session, _) = mfa_session(&pool, "mfa-scope").await;
    let challenge = DeviceIdentityChallenge::for_mfa(&session, vec![1])
        .save(&pool)
        .await
        .unwrap();
    for (device_id, location_id, session_id) in [
        (
            session.device_id + 1,
            Some(session.location_id),
            Some(session.id),
        ),
        (
            session.device_id,
            Some(session.location_id + 1),
            Some(session.id),
        ),
        (
            session.device_id,
            Some(session.location_id),
            Some(session.id + 1),
        ),
        (session.device_id, Some(session.location_id), None),
    ] {
        assert!(
            DeviceIdentityChallenge::consume(
                &pool,
                challenge.id,
                device_id,
                DeviceIdentityChallengePurpose::MfaConnection,
                location_id,
                session_id
            )
            .await
            .unwrap()
            .is_none()
        );
    }
    let mut connection = pool.acquire().await.unwrap();
    assert!(
        !DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            challenge.id + 1,
            &session
        )
        .await
        .unwrap()
    );
    let stored = VpnClientMfaSession::find_by_id(&pool, session.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!stored.possession_verified);
    let other_device = device(&pool, "other-mfa-device").await;
    let mut mismatched = DeviceIdentityChallenge::for_mfa(&session, vec![2]);
    mismatched.device_id = other_device.id;
    assert_constraint(
        mismatched.save(&pool).await.err().unwrap(),
        "device_identity_challenge_mfa_session",
    );
    let mut missing_session = DeviceIdentityChallenge::for_mfa(&session, vec![3]);
    missing_session.mfa_session_id = None;
    assert_constraint(
        missing_session.save(&pool).await.err().unwrap(),
        "device_identity_challenge_location",
    );
}

#[sqlx::test]
async fn test_mfa_possession_rejects_expired_challenge_or_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (session, _) = mfa_session(&pool, "mfa-expiry").await;
    let mut expired = DeviceIdentityChallenge::for_mfa(&session, vec![1]);
    expired.created_at -= chrono::TimeDelta::minutes(20);
    expired.expires_at = expired.created_at + chrono::TimeDelta::minutes(1);
    let expired = expired.save(&pool).await.unwrap();
    let mut connection = pool.acquire().await.unwrap();
    assert!(
        !DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            expired.id,
            &session
        )
        .await
        .unwrap()
    );
    let mut too_long = DeviceIdentityChallenge::for_mfa(&session, vec![2]);
    too_long.expires_at += chrono::TimeDelta::minutes(1);
    let too_long = too_long.save(&pool).await.unwrap();
    assert!(
        !DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            too_long.id,
            &session
        )
        .await
        .unwrap()
    );
    let challenge = DeviceIdentityChallenge::for_mfa(&session, vec![3])
        .save(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE vpn_client_mfa_session SET expires_at = $1 WHERE id = $2")
        .bind(chrono::Utc::now().naive_utc() - chrono::TimeDelta::seconds(1))
        .bind(session.id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            challenge.id,
            &session
        )
        .await
        .unwrap()
    );
    assert!(
        !VpnClientMfaSession::find_by_id(&pool, session.id)
            .await
            .unwrap()
            .unwrap()
            .possession_verified
    );
}

#[sqlx::test]
async fn test_mfa_possession_verification_is_rolled_back(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (session, _) = mfa_session(&pool, "mfa-rollback").await;
    let challenge = DeviceIdentityChallenge::for_mfa(&session, vec![1])
        .save(&pool)
        .await
        .unwrap();
    let mut transaction = pool.begin().await.unwrap();
    assert!(
        DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut transaction,
            challenge.id,
            &session
        )
        .await
        .unwrap()
    );
    transaction.rollback().await.unwrap();
    let restored = VpnClientMfaSession::find_by_id(&pool, session.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!restored.possession_verified);
    assert!(
        DeviceIdentityChallenge::find_by_id(&pool, challenge.id)
            .await
            .unwrap()
            .unwrap()
            .consumed_at
            .is_none()
    );
}

#[sqlx::test]
async fn test_mfa_possession_deadline_applies_inside_transaction(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (session, _) = mfa_session(&pool, "mfa-transaction-deadline").await;
    let challenge = DeviceIdentityChallenge::for_mfa(&session, vec![1])
        .save(&pool)
        .await
        .unwrap();
    for expire_session in [false, true] {
        let mut delayed_transaction = pool.begin().await.unwrap();
        let deadline: chrono::NaiveDateTime =
            sqlx::query_scalar("SELECT (now() AT TIME ZONE 'UTC') + interval '50 milliseconds'")
                .fetch_one(&mut *delayed_transaction)
                .await
                .unwrap();
        if expire_session {
            sqlx::query("UPDATE vpn_client_mfa_session SET expires_at = $1 WHERE id = $2")
                .bind(deadline)
                .bind(session.id)
                .execute(&mut *delayed_transaction)
                .await
                .unwrap();
        }
        sqlx::query("UPDATE device_identity_challenge SET expires_at = $1 WHERE id = $2")
            .bind(deadline)
            .bind(challenge.id)
            .execute(&mut *delayed_transaction)
            .await
            .unwrap();
        sqlx::query("SELECT pg_sleep(0.1)")
            .execute(&mut *delayed_transaction)
            .await
            .unwrap();
        assert!(
            !DeviceIdentityChallenge::record_verified_mfa_possession(
                &mut delayed_transaction,
                challenge.id,
                &session,
            )
            .await
            .unwrap()
        );
        delayed_transaction.rollback().await.unwrap();
    }
}

#[sqlx::test]
async fn test_mfa_possession_is_recorded_once_under_concurrency(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (session, _) = mfa_session(&pool, "mfa-concurrent").await;
    let challenge = DeviceIdentityChallenge::for_mfa(&session, vec![1])
        .save(&pool)
        .await
        .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    let mut second_connection = pool.acquire().await.unwrap();
    let (first, second) = tokio::join!(
        DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            challenge.id,
            &session
        ),
        DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut second_connection,
            challenge.id,
            &session
        ),
    );
    assert!(matches!(
        (first.unwrap(), second.unwrap()),
        (true, false) | (false, true)
    ));
    assert!(
        VpnClientMfaSession::find_by_id(&pool, session.id)
            .await
            .unwrap()
            .unwrap()
            .possession_verified
    );
}

#[sqlx::test]
async fn test_mfa_possession_is_bound_to_session_generation_and_cascades_on_restart(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (old_session, _) = mfa_session(&pool, "mfa-generation").await;
    let old_challenge = DeviceIdentityChallenge::for_mfa(&old_session, vec![1])
        .save(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE vpn_client_mfa_session SET token_hash = $1 WHERE id = $2")
        .bind("replacement-session-hash")
        .bind(old_session.id)
        .execute(&pool)
        .await
        .unwrap();
    let current = VpnClientMfaSession::find_by_id(&pool, old_session.id)
        .await
        .unwrap()
        .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    assert!(
        !DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            old_challenge.id,
            &old_session
        )
        .await
        .unwrap()
    );
    assert!(
        !DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            old_challenge.id,
            &current
        )
        .await
        .unwrap()
    );
    let fresh = DeviceIdentityChallenge::for_mfa(&current, vec![2])
        .save(&pool)
        .await
        .unwrap();
    assert!(
        DeviceIdentityChallenge::record_verified_mfa_possession(
            &mut connection,
            fresh.id,
            &current
        )
        .await
        .unwrap()
    );
    let (replacement, _) = VpnClientMfaSession::start(
        &mut connection,
        current.location_id,
        current.device_id,
        current.user_id,
        1,
        vec![vec![VpnClientMfaMethod::Totp]],
        VpnMfaFlowKind::Legacy,
        VpnClientMfaMethod::Totp,
        None,
        std::time::Duration::from_mins(10),
    )
    .await
    .unwrap();
    assert!(!replacement.possession_verified);
    assert!(
        DeviceIdentityChallenge::find_by_id(&pool, old_challenge.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        DeviceIdentityChallenge::find_by_id(&pool, fresh.id)
            .await
            .unwrap()
            .is_none()
    );
}

fn identity_for_device(device_id: Id, backend: DeviceIdentityBackend) -> DeviceIdentity {
    DeviceIdentity {
        device_id,
        backend,
        public_key_der: vec![0, 255, 128],
        apple_sign_count: (backend == DeviceIdentityBackend::AppleAppAttest).then_some(0),
        tpm_ek_public: (backend == DeviceIdentityBackend::Tpm).then_some(vec![1, 2, 3]),
        tpm_ek_certificate_chain_der: (backend == DeviceIdentityBackend::Tpm)
            .then_some(vec![vec![4, 5, 6]]),
        apple_key_id: (backend == DeviceIdentityBackend::AppleAppAttest)
            .then_some("apple-key-id".into()),
    }
}

#[sqlx::test]
async fn test_device_identity_roundtrip_status_and_cascade(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    for (name, backend) in [
        ("identity-tpm", DeviceIdentityBackend::Tpm),
        ("identity-apple", DeviceIdentityBackend::AppleAppAttest),
    ] {
        let row = device(&pool, name).await;
        assert_eq!(row.attestation_status, DeviceAttestationStatus::NotAttested);
        assert!(row.attested_at.is_none());
        assert!(
            DeviceIdentity::find_by_device_id(&pool, row.id)
                .await
                .unwrap()
                .is_none()
        );
        let mut identity = identity_for_device(row.id, backend);
        let mut connection = pool.acquire().await.unwrap();
        let saved_device = identity
            .save_verified(
                &mut connection,
                DeviceAttestationStatus::Attested,
                row.created,
            )
            .await
            .unwrap();
        assert!(
            DeviceIdentity::find_by_device_id(&pool, row.id)
                .await
                .unwrap()
                == Some(identity.clone())
        );
        let updated = Device::find_by_id(&pool, row.id).await.unwrap().unwrap();
        assert_eq!(saved_device, updated);
        assert_eq!(
            updated.attestation_status,
            DeviceAttestationStatus::Attested
        );
        assert_eq!(updated.attested_at, Some(row.created));
        if backend == DeviceIdentityBackend::AppleAppAttest {
            identity.apple_sign_count = Some(4_294_967_295);
            identity
                .save_verified(
                    &mut connection,
                    DeviceAttestationStatus::Attested,
                    row.created,
                )
                .await
                .unwrap();
            assert!(
                DeviceIdentity::find_by_device_id(&pool, row.id)
                    .await
                    .unwrap()
                    == Some(identity)
            );
            for count in [-1i64, 4_294_967_296] {
                assert_constraint(
                    sqlx::query(
                        "UPDATE device_identity SET apple_sign_count = $1 WHERE device_id = $2",
                    )
                    .bind(count)
                    .bind(row.id)
                    .execute(&pool)
                    .await
                    .unwrap_err(),
                    "device_identity_apple_counter_range",
                );
            }
        } else {
            assert_constraint(
                sqlx::query(
                    "UPDATE device_identity SET apple_key_id = 'apple-key' WHERE device_id = $1",
                )
                .bind(row.id)
                .execute(&pool)
                .await
                .unwrap_err(),
                "device_identity_backend_fields",
            );
        }
        row.delete(&pool).await.unwrap();
        assert!(
            DeviceIdentity::find_by_device_id(&pool, updated.id)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[sqlx::test]
async fn test_device_identity_validation_preserves_device_status(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let row = device(&pool, "identity-validation").await;
    let mut invalid = vec![];
    let mut empty_key = identity_for_device(row.id, DeviceIdentityBackend::Tpm);
    empty_key.public_key_der.clear();
    invalid.push(("empty public key", empty_key));
    let mut missing_ek = identity_for_device(row.id, DeviceIdentityBackend::Tpm);
    missing_ek.tpm_ek_public = None;
    invalid.push(("missing EK", missing_ek));
    let mut missing_chain = identity_for_device(row.id, DeviceIdentityBackend::Tpm);
    missing_chain.tpm_ek_certificate_chain_der = None;
    invalid.push(("missing EK chain", missing_chain));
    let mut mixed = identity_for_device(row.id, DeviceIdentityBackend::Tpm);
    mixed.apple_key_id = Some("apple-key".into());
    invalid.push(("TPM with Apple provenance", mixed));
    let mut missing_apple_key = identity_for_device(row.id, DeviceIdentityBackend::AppleAppAttest);
    missing_apple_key.apple_key_id = None;
    invalid.push(("missing Apple key ID", missing_apple_key));
    let mut mixed = identity_for_device(row.id, DeviceIdentityBackend::AppleAppAttest);
    mixed.tpm_ek_public = Some(vec![1]);
    invalid.push(("Apple with TPM provenance", mixed));
    for count in [-1i64, 4_294_967_296] {
        let mut apple = identity_for_device(row.id, DeviceIdentityBackend::AppleAppAttest);
        apple.apple_sign_count = Some(count);
        invalid.push(("Apple counter out of range", apple));
    }
    let mut connection = pool.acquire().await.unwrap();
    for (case, identity) in invalid {
        assert!(
            matches!(
                identity
                    .save_verified(
                        &mut connection,
                        DeviceAttestationStatus::Attested,
                        row.created
                    )
                    .await,
                Err(DeviceIdentityError::InvalidState(_))
            ),
            "unexpected result for {case}"
        );
        assert_eq!(
            Device::find_by_id(&pool, row.id).await.unwrap(),
            Some(row.clone()),
            "device changed for {case}"
        );
        assert!(
            DeviceIdentity::find_by_device_id(&pool, row.id)
                .await
                .unwrap()
                .is_none(),
            "identity stored for {case}"
        );
    }
    let identity = identity_for_device(row.id, DeviceIdentityBackend::Tpm);
    assert!(matches!(
        identity
            .save_verified(
                &mut connection,
                DeviceAttestationStatus::NotAttested,
                row.created
            )
            .await,
        Err(DeviceIdentityError::InvalidState(_))
    ));
    assert_eq!(
        Device::find_by_id(&pool, row.id).await.unwrap(),
        Some(row.clone())
    );
    assert!(
        DeviceIdentity::find_by_device_id(&pool, row.id)
            .await
            .unwrap()
            .is_none()
    );
    let orphan = identity_for_device(row.id + 1, DeviceIdentityBackend::Tpm);
    assert!(matches!(
        orphan
            .save_verified(
                &mut connection,
                DeviceAttestationStatus::Attested,
                row.created
            )
            .await,
        Err(DeviceIdentityError::DeviceNotFound)
    ));
}

#[sqlx::test]
async fn test_device_identity_manual_approval_and_transaction_rollback(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let row = device(&pool, "identity-transaction").await;
    let mut identity = identity_for_device(row.id, DeviceIdentityBackend::Tpm);
    identity.tpm_ek_public = None;
    identity.tpm_ek_certificate_chain_der = None;
    let mut transaction = pool.begin().await.unwrap();
    identity
        .save_verified(
            &mut transaction,
            DeviceAttestationStatus::AttestedManually,
            row.created,
        )
        .await
        .unwrap();
    assert!(
        DeviceIdentity::find_by_device_id(&mut *transaction, row.id)
            .await
            .unwrap()
            == Some(identity.clone())
    );
    transaction.rollback().await.unwrap();
    assert!(
        DeviceIdentity::find_by_device_id(&pool, row.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        Device::find_by_id(&pool, row.id).await.unwrap(),
        Some(row.clone())
    );
    let mut connection = pool.acquire().await.unwrap();
    identity
        .save_verified(
            &mut connection,
            DeviceAttestationStatus::AttestedManually,
            row.created,
        )
        .await
        .unwrap();
    let updated = Device::find_by_id(&pool, row.id).await.unwrap().unwrap();
    assert_eq!(
        updated.attestation_status,
        DeviceAttestationStatus::AttestedManually
    );
    assert_eq!(updated.attested_at, Some(row.created));
    assert!(
        DeviceIdentity::find_by_device_id(&pool, row.id)
            .await
            .unwrap()
            == Some(identity)
    );
}

#[sqlx::test]
async fn test_reap_expired_device_identity_state(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (session, _) = mfa_session(&pool, "reap-challenges").await;
    let now = chrono::Utc::now().naive_utc().trunc_subsecs(6);
    let mut expired_ids = Vec::new();
    let mut active_ids = Vec::new();
    for (purpose, expired, consumed) in [
        (DeviceIdentityChallengePurpose::Polling, true, false),
        (
            DeviceIdentityChallengePurpose::PostureConnection,
            true,
            true,
        ),
        (DeviceIdentityChallengePurpose::MfaConnection, true, false),
        (DeviceIdentityChallengePurpose::Polling, false, false),
        (DeviceIdentityChallengePurpose::Polling, false, true),
    ] {
        let challenge = DeviceIdentityChallenge {
            id: NoId,
            purpose,
            device_id: session.device_id,
            location_id: (purpose != DeviceIdentityChallengePurpose::Polling)
                .then_some(session.location_id),
            mfa_session_id: (purpose == DeviceIdentityChallengePurpose::MfaConnection)
                .then_some(session.id),
            mfa_session_token_hash: (purpose == DeviceIdentityChallengePurpose::MfaConnection)
                .then_some(session.token_hash.clone()),
            data: vec![1],
            created_at: now - chrono::TimeDelta::minutes(10),
            expires_at: if expired {
                now - chrono::TimeDelta::minutes(5)
            } else {
                now + chrono::TimeDelta::minutes(5)
            },
            consumed_at: consumed.then_some(now - chrono::TimeDelta::minutes(9)),
        }
        .save(&pool)
        .await
        .unwrap();
        if expired {
            expired_ids.push(challenge.id);
        } else {
            active_ids.push(challenge.id);
        }
    }
    let mut expired_attestation =
        enrollment_attestation(&pool, "expired-attestation", tpm_challenge()).await;
    expired_attestation.created_at = now - chrono::TimeDelta::minutes(10);
    expired_attestation.expires_at = now - chrono::TimeDelta::minutes(5);
    expired_attestation.save(&pool).await.unwrap();
    let active_attestation =
        enrollment_attestation(&pool, "active-attestation", tpm_challenge()).await;
    active_attestation.save(&pool).await.unwrap();
    assert_eq!(super::reap_expired(&pool).await.unwrap(), 4);
    for id in expired_ids {
        assert!(
            DeviceIdentityChallenge::find_by_id(&pool, id)
                .await
                .unwrap()
                .is_none()
        );
    }
    for id in active_ids {
        assert!(
            DeviceIdentityChallenge::find_by_id(&pool, id)
                .await
                .unwrap()
                .is_some()
        );
    }
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &expired_attestation.token_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        DeviceAttestationState::find_by_token_id(&pool, &active_attestation.token_id)
            .await
            .unwrap()
            == Some(active_attestation)
    );
    assert!(
        Token::find_by_id(&pool, &expired_attestation.token_id)
            .await
            .is_ok()
    );
    assert!(
        VpnClientMfaSession::find_by_id(&pool, session.id)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(super::reap_expired(&pool).await.unwrap(), 0);
}

#[sqlx::test]
async fn test_identity_save_rolls_back_when_device_update_fails(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    sqlx::query(
        "ALTER TABLE device ADD CONSTRAINT test_reject_later_attestation \
         CHECK (attested_at IS NULL OR attested_at <= created)",
    )
    .execute(&pool)
    .await
    .unwrap();
    for (name, existing_identity) in [("insert-failure", false), ("update-failure", true)] {
        let row = device(&pool, name).await;
        let mut connection = pool.acquire().await.unwrap();
        let previous_identity = if existing_identity {
            let identity = identity_for_device(row.id, DeviceIdentityBackend::Tpm);
            identity
                .save_verified(
                    &mut connection,
                    DeviceAttestationStatus::AttestedManually,
                    row.created,
                )
                .await
                .unwrap();
            Some(identity)
        } else {
            None
        };
        let previous_device = Device::find_by_id(&mut *connection, row.id).await.unwrap();
        let candidate = identity_for_device(row.id, DeviceIdentityBackend::AppleAppAttest);
        let result = candidate
            .save_verified(
                &mut connection,
                DeviceAttestationStatus::Attested,
                row.created + chrono::TimeDelta::seconds(1),
            )
            .await;
        let Err(DeviceIdentityError::Database(error)) = result else {
            panic!("expected the device update constraint to reject the save");
        };
        assert_constraint(error, "test_reject_later_attestation");
        assert_eq!(
            Device::find_by_id(&mut *connection, row.id).await.unwrap(),
            previous_device
        );
        assert!(
            DeviceIdentity::find_by_device_id(&mut *connection, row.id)
                .await
                .unwrap()
                == previous_identity
        );

        candidate
            .save_verified(
                &mut connection,
                DeviceAttestationStatus::Attested,
                row.created,
            )
            .await
            .unwrap();
        assert!(
            DeviceIdentity::find_by_device_id(&mut *connection, row.id)
                .await
                .unwrap()
                == Some(candidate)
        );
    }
}
