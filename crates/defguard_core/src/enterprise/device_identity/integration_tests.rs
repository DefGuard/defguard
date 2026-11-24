use chrono::{SubsecRound, TimeDelta, Utc};
use defguard_common::db::{
    Id,
    models::{Device, DeviceType, User, device_identity::DeviceAttestationStatus},
    setup_pool,
};
use defguard_proto::device_identity::{
    AttestationChallenge, AttestationChallengeSolution, AttestationIdentity, AttestationStatus,
    IdentityErrorCode, attestation_challenge, attestation_challenge_solution, attestation_identity,
};
use p256::ecdsa::SigningKey;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tonic::Code;

use super::{
    generate_attestation_challenge,
    tpm::test_support::{Fixture, activate, name, signing_public},
    verify_attestation,
};
use crate::{
    db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token},
    enterprise::db::models::device_identity::{
        DeviceAttestationState, DeviceIdentity, DeviceIdentityBackend,
    },
};

async fn session(pool: &PgPool, label: &str) -> (Token, Device<Id>) {
    defguard_common::db::models::settings::initialize_current_settings(pool)
        .await
        .unwrap();
    let user = User::new(
        label,
        None,
        "First",
        "Last",
        &format!("{label}@example.com"),
        None,
    )
    .save(pool)
    .await
    .unwrap();
    let mut token = Token::new(
        user.id,
        None,
        None,
        3600,
        Some(ENROLLMENT_TOKEN_TYPE.into()),
    );
    token.used_at = Some(Utc::now().naive_utc().trunc_subsecs(6));
    token.save(pool).await.unwrap();
    let device = Device::new(
        label.into(),
        "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap();
    (token, device)
}

async fn start(
    pool: &PgPool,
    token: &Token,
    device: &Device<Id>,
    fixture: &Fixture,
) -> AttestationChallenge {
    generate_attestation_challenge(
        pool,
        token,
        device,
        &AttestationIdentity {
            backend: Some(attestation_identity::Backend::Tpm(fixture.metadata.clone())),
        },
    )
    .await
    .unwrap()
}

fn solution(fixture: &Fixture, challenge: &AttestationChallenge) -> AttestationChallengeSolution {
    let Some(attestation_challenge::Challenge::Tpm(credential)) = &challenge.challenge else {
        panic!("expected TPM credential")
    };
    let secret = activate(fixture, credential, &name(&fixture.metadata.ak_public));
    AttestationChallengeSolution {
        session_id: challenge.session_id.clone(),
        result: Some(attestation_challenge_solution::Result::Tpm(
            fixture.response(secret, &challenge.data),
        )),
    }
}

async fn pending(pool: &PgPool, token: &Token, device: &Device<Id>) -> DeviceAttestationState {
    DeviceAttestationState::find_for_device(pool, &token.id, device.id)
        .await
        .unwrap()
        .unwrap()
}

async fn assert_not_attested(pool: &PgPool, device: &Device<Id>) {
    assert!(
        DeviceIdentity::find_by_device_id(pool, device.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        Device::find_by_id(pool, device.id).await.unwrap(),
        Some(device.clone())
    );
}

async fn assert_attestation_failed(pool: &PgPool, device: &Device<Id>) {
    assert!(
        DeviceIdentity::find_by_device_id(pool, device.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        Device::find_by_id(pool, device.id)
            .await
            .unwrap()
            .unwrap()
            .attestation_status,
        DeviceAttestationStatus::AttestationFailed
    );
}

#[sqlx::test]
async fn test_start_finish_registers_first_identity_and_consumes_challenge(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    for rsa in [false, true] {
        let (token, device) =
            session(&pool, if rsa { "ceremony-rsa" } else { "ceremony-ecc" }).await;
        let fixture = Fixture::new(rsa);
        let challenge = start(&pool, &token, &device, &fixture).await;
        let state = pending(&pool, &token, &device).await;
        assert_eq!(state.challenge_id, challenge.session_id);
        assert_eq!(state.payload.data, challenge.data);
        assert_eq!(state.expires_at.and_utc().timestamp(), challenge.expires_at);
        assert_eq!(state.wireguard_key, device.wireguard_pubkey);
        assert_not_attested(&pool, &device).await;
        let solution = solution(&fixture, &challenge);
        let result = verify_attestation(&pool, &token, &device, &solution)
            .await
            .unwrap();
        assert_eq!(result.identity_id, device.id.to_string());
        assert_eq!(
            result.attestation_status,
            i32::from(AttestationStatus::Attested)
        );
        assert_eq!(result.key_fingerprint.len(), 32);
        assert!(result.attested_at.is_some());
        assert!(result.error.is_none());
        assert!(result.last_possession_verified_at.is_none());
        let saved = DeviceIdentity::find_by_device_id(&pool, device.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.backend, DeviceIdentityBackend::Tpm);
        assert_eq!(
            saved.tpm_ek_public,
            Some(fixture.metadata.ek_public.clone())
        );
        assert_eq!(saved.tpm_ek_certificate_chain_der, Some(vec![]));
        let after = Device::find_by_id(&pool, device.id).await.unwrap().unwrap();
        assert_eq!(after.attestation_status, DeviceAttestationStatus::Attested);
        assert_eq!(
            after.attested_at.unwrap().and_utc().timestamp(),
            result.attested_at.unwrap()
        );
        assert_eq!(after.wireguard_pubkey, device.wireguard_pubkey);
        assert!(
            DeviceAttestationState::find_for_device(&pool, &token.id, device.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            Token::find_by_id(&pool, &token.id).await.unwrap().used_at,
            token.used_at
        );
        assert_eq!(
            verify_attestation(&pool, &token, &device, &solution)
                .await
                .unwrap_err()
                .code(),
            Code::FailedPrecondition
        );
        let repeat = generate_attestation_challenge(
            &pool,
            &token,
            &device,
            &AttestationIdentity {
                backend: Some(attestation_identity::Backend::Tpm(fixture.metadata.clone())),
            },
        )
        .await;
        assert_eq!(repeat.unwrap_err().code(), Code::FailedPrecondition);
        assert!(
            DeviceIdentity::find_by_device_id(&pool, device.id)
                .await
                .unwrap()
                == Some(saved)
        );
    }
}

#[sqlx::test]
async fn test_restarted_challenge_rejects_old_solution_and_preserves_other_sessions(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (token, device) = session(&pool, "ceremony-restart").await;
    let mut other = Token::new(
        token.user_id,
        None,
        None,
        3600,
        Some(ENROLLMENT_TOKEN_TYPE.into()),
    );
    other.used_at = token.used_at;
    other.save(&pool).await.unwrap();
    let fixture = Fixture::new(false);
    let first = start(&pool, &token, &device, &fixture).await;
    let old_solution = solution(&fixture, &first);
    let other_challenge = start(&pool, &other, &device, &fixture).await;
    let second = start(&pool, &token, &device, &fixture).await;
    assert_ne!(first.session_id, second.session_id);
    assert_ne!(first.data, second.data);
    assert_eq!(
        pending(&pool, &other, &device).await.challenge_id,
        other_challenge.session_id
    );
    assert_eq!(
        verify_attestation(&pool, &token, &device, &old_solution)
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    assert_eq!(
        pending(&pool, &token, &device).await.challenge_id,
        second.session_id
    );
    assert_not_attested(&pool, &device).await;
    verify_attestation(&pool, &token, &device, &solution(&fixture, &second))
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_invalid_signature_marks_device_attestation_failed(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (token, device) = session(&pool, "ceremony-invalid-signature").await;
    let fixture = Fixture::new(false);
    let challenge = start(&pool, &token, &device, &fixture).await;
    let mut bad = solution(&fixture, &challenge);
    let Some(attestation_challenge_solution::Result::Tpm(response)) = &mut bad.result else {
        unreachable!()
    };
    response
        .evidence
        .as_mut()
        .unwrap()
        .signature
        .as_mut()
        .unwrap()
        .signature[0] ^= 1;
    assert_eq!(
        verify_attestation(&pool, &token, &device, &bad)
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    assert!(
        DeviceAttestationState::find_for_device(&pool, &token.id, device.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_attestation_failed(&pool, &device).await;
}

#[sqlx::test]
async fn test_client_failure_does_not_consume_challenge(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (token, device) = session(&pool, "ceremony-client-failure").await;
    let fixture = Fixture::new(false);
    let challenge = start(&pool, &token, &device, &fixture).await;
    let original = pending(&pool, &token, &device).await;
    let failure = AttestationChallengeSolution {
        session_id: challenge.session_id,
        result: Some(attestation_challenge_solution::Result::Error(
            IdentityErrorCode::BackendFailure.into(),
        )),
    };
    assert_eq!(
        verify_attestation(&pool, &token, &device, &failure)
            .await
            .unwrap_err()
            .code(),
        Code::FailedPrecondition
    );
    assert!(pending(&pool, &token, &device).await == original);
    assert_not_attested(&pool, &device).await;
}

#[sqlx::test]
async fn test_finish_is_bound_to_session_device_and_wireguard_key(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (token, device) = session(&pool, "ceremony-binding").await;
    let (foreign_token, foreign_device) = session(&pool, "ceremony-foreign").await;
    let fixture = Fixture::new(false);
    let challenge = start(&pool, &token, &device, &fixture).await;
    let solution = solution(&fixture, &challenge);
    assert_eq!(
        verify_attestation(&pool, &foreign_token, &device, &solution)
            .await
            .unwrap_err()
            .code(),
        Code::FailedPrecondition
    );
    assert_eq!(
        verify_attestation(&pool, &token, &foreign_device, &solution)
            .await
            .unwrap_err()
            .code(),
        Code::FailedPrecondition
    );
    let mut changed = device.clone();
    changed.wireguard_pubkey = "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=".into();
    assert_eq!(
        verify_attestation(&pool, &token, &changed, &solution)
            .await
            .unwrap_err()
            .code(),
        Code::InvalidArgument
    );
    assert_not_attested(&pool, &device).await;
}

#[sqlx::test]
async fn test_finalization_rechecks_authorization_from_enrollment_snapshot(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    for mutation in [
        "disabled",
        "session_changed",
        "expired_session",
        "wireguard_changed",
        "owner_changed",
    ] {
        let (token, device) = session(&pool, &format!("ceremony-{mutation}")).await;
        let fixture = Fixture::new(false);
        let challenge = start(&pool, &token, &device, &fixture).await;
        let solution = solution(&fixture, &challenge);
        let expected = match mutation {
            "disabled" => {
                let mut user = User::find_by_id(&pool, token.user_id)
                    .await
                    .unwrap()
                    .unwrap();
                user.is_active = false;
                user.save(&pool).await.unwrap();
                Code::PermissionDenied
            }
            "session_changed" | "expired_session" => {
                let mut changed = token.clone();
                changed.used_at = Some(token.used_at.unwrap() - TimeDelta::days(1));
                changed.save(&pool).await.unwrap();
                Code::Unauthenticated
            }
            "wireguard_changed" => {
                sqlx::query("UPDATE device SET wireguard_pubkey = $1 WHERE id = $2")
                    .bind("AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=")
                    .bind(device.id)
                    .execute(&pool)
                    .await
                    .unwrap();
                Code::FailedPrecondition
            }
            "owner_changed" => {
                let (other, _) = session(&pool, "ceremony-new-owner").await;
                sqlx::query("UPDATE device SET user_id = $1 WHERE id = $2")
                    .bind(other.user_id)
                    .bind(device.id)
                    .execute(&pool)
                    .await
                    .unwrap();
                Code::PermissionDenied
            }
            _ => unreachable!(),
        };
        let current_token = if mutation == "expired_session" {
            Token::find_by_id(&pool, &token.id).await.unwrap()
        } else {
            token.clone()
        };
        assert_eq!(
            verify_attestation(&pool, &current_token, &device, &solution)
                .await
                .unwrap_err()
                .code(),
            expected,
            "{mutation}"
        );
        assert!(
            DeviceIdentity::find_by_device_id(&pool, device.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            pending(&pool, &token, &device).await.challenge_id,
            challenge.session_id
        );
    }
}

#[sqlx::test]
async fn test_only_first_parallel_identity_registration_succeeds(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (token, device) = session(&pool, "ceremony-race").await;
    let mut other = Token::new(
        token.user_id,
        None,
        None,
        3600,
        Some(ENROLLMENT_TOKEN_TYPE.into()),
    );
    other.used_at = token.used_at;
    other.save(&pool).await.unwrap();
    let fixture = Fixture::new(false);
    let mut second_fixture = Fixture::new(false);
    second_fixture.identity = SigningKey::from_slice(&[4; 32]).unwrap();
    second_fixture.identity_public = signing_public(&second_fixture.identity, false);
    let first = start(&pool, &token, &device, &fixture).await;
    let second = start(&pool, &other, &device, &second_fixture).await;
    let first_solution = solution(&fixture, &first);
    let second_solution = solution(&second_fixture, &second);
    let (first_result, second_result) = tokio::join!(
        verify_attestation(&pool, &token, &device, &first_solution),
        verify_attestation(&pool, &other, &device, &second_solution)
    );
    let (winner, loser, winner_token, loser_token) = match (first_result, second_result) {
        (Ok(winner), Err(loser)) => (winner, loser, &token, &other),
        (Err(loser), Ok(winner)) => (winner, loser, &other, &token),
        _ => panic!("exactly one registration must succeed"),
    };
    assert_eq!(loser.code(), Code::FailedPrecondition);
    assert!(
        DeviceAttestationState::find_for_device(&pool, &winner_token.id, device.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        DeviceAttestationState::find_for_device(&pool, &loser_token.id, device.id)
            .await
            .unwrap()
            .is_some()
    );
    let saved = DeviceIdentity::find_by_device_id(&pool, device.id)
        .await
        .unwrap()
        .unwrap();
    use sha2::{Digest, Sha256};
    assert_eq!(
        winner.key_fingerprint,
        Sha256::digest(&saved.public_key_der).as_slice()
    );
}

#[sqlx::test]
async fn test_identity_write_failure_rolls_back_challenge_consumption(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (token, device) = session(&pool, "ceremony-rollback").await;
    let fixture = Fixture::new(false);
    let challenge = start(&pool, &token, &device, &fixture).await;
    let original = pending(&pool, &token, &device).await;
    let solution = solution(&fixture, &challenge);
    sqlx::query("ALTER TABLE device ADD CONSTRAINT reject_attestation_write CHECK (attestation_status <> 'attested')").execute(&pool).await.unwrap();
    assert_eq!(
        verify_attestation(&pool, &token, &device, &solution)
            .await
            .unwrap_err()
            .code(),
        Code::Internal
    );
    assert!(pending(&pool, &token, &device).await == original);
    assert_not_attested(&pool, &device).await;
    sqlx::query("ALTER TABLE device DROP CONSTRAINT reject_attestation_write")
        .execute(&pool)
        .await
        .unwrap();
    verify_attestation(&pool, &token, &device, &solution)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_expired_challenge_cannot_be_finished(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (token, device) = session(&pool, "ceremony-expired").await;
    let fixture = Fixture::new(false);
    let challenge = start(&pool, &token, &device, &fixture).await;
    let solution = solution(&fixture, &challenge);
    let mut state = pending(&pool, &token, &device).await;
    state.expires_at = Utc::now().naive_utc() - TimeDelta::seconds(1);
    state.save(&pool).await.unwrap();
    assert_eq!(
        verify_attestation(&pool, &token, &device, &solution)
            .await
            .unwrap_err()
            .code(),
        Code::DeadlineExceeded
    );
    assert_not_attested(&pool, &device).await;
}
