use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::db::{
    Id,
    models::{
        biometric_auth::BiometricChallenge,
        device::{Device, DeviceType},
        user::User,
        vpn_client_session::VpnClientMfaMethod,
        wireguard::WireguardNetwork,
    },
    setup_pool,
};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_suffix() -> String {
    COUNTER.fetch_add(1, Ordering::Relaxed).to_string()
}

async fn create_location(pool: &sqlx::PgPool) -> WireguardNetwork<Id> {
    create_location_with_address(pool, "10.0.6.1/24").await
}

async fn create_location_with_address(pool: &sqlx::PgPool, address: &str) -> WireguardNetwork<Id> {
    WireguardNetwork::default()
        .try_set_address(address)
        .unwrap()
        .save(pool)
        .await
        .unwrap()
}

async fn create_user(pool: &sqlx::PgPool) -> User<Id> {
    let suffix = next_suffix();
    User::new(
        format!("mfa-session-user-{suffix}"),
        None,
        "Ln".to_string(),
        "Fn".to_string(),
        format!("mfa-{suffix}@t.com"),
        None,
    )
    .save(pool)
    .await
    .unwrap()
}

async fn create_device(pool: &sqlx::PgPool, user_id: Id) -> Device<Id> {
    let suffix = next_suffix();
    Device::new(
        format!("mfa-session-device-{suffix}"),
        format!("device-pubkey-{suffix}"),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap()
}

async fn start_session(pool: &sqlx::PgPool) -> (VpnClientMfaSession<Id>, StartOutcome) {
    start_session_with_ttl(pool, Duration::from_mins(10)).await
}

async fn start_session_with_ttl(
    pool: &sqlx::PgPool,
    ttl: Duration,
) -> (VpnClientMfaSession<Id>, StartOutcome) {
    let location = create_location(pool).await;
    let user = create_user(pool).await;
    let device = create_device(pool, user.id).await;
    let mut tx = pool.begin().await.unwrap();
    let result = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
        VpnClientMfaMethod::Totp,
        None,
        ttl,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    result
}

async fn refetch(pool: &sqlx::PgPool, token: &str) -> VpnClientMfaSession<Id> {
    VpnClientMfaSession::<Id>::find_active_by_token(pool, token)
        .await
        .unwrap()
        .expect("expected active session")
}

#[sqlx::test]
async fn test_start_supersedes_existing_session(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    let steps = vec![vec![VpnClientMfaMethod::Totp]];

    let mut tx = pool.begin().await.unwrap();
    let (first, first_outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps.clone(),
        VpnClientMfaMethod::Totp,
        None,
        Duration::from_mins(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(first.current_step_methods(), [VpnClientMfaMethod::Totp]);
    // The raw token is never stored; only its hash is.
    assert_eq!(first.token_hash, hash_token(&first_outcome.token));
    assert_ne!(first.token_hash, first_outcome.token);
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &first_outcome.token)
            .await
            .unwrap()
            .is_some()
    );

    let mut tx = pool.begin().await.unwrap();
    let (_second, second_outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps,
        VpnClientMfaMethod::Totp,
        None,
        Duration::from_mins(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(
        second_outcome.superseded_token_hash.as_deref(),
        Some(first.token_hash.as_str())
    );
    // The superseded token no longer validates; the new one does.
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &first_outcome.token)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &second_outcome.token)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test]
async fn test_start_returns_superseded_token_hash(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    let steps = vec![vec![VpnClientMfaMethod::Totp]];

    let mut tx = pool.begin().await.unwrap();
    let (first, _) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps.clone(),
        VpnClientMfaMethod::Totp,
        None,
        Duration::from_mins(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let (_, outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps,
        VpnClientMfaMethod::Totp,
        None,
        Duration::from_mins(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(
        outcome.superseded_token_hash.as_deref(),
        Some(first.token_hash.as_str())
    );
}

/// `start` must commit the first attempt with the row it mints. If the attempt were a second
/// write, a concurrent `start` taking the `ON CONFLICT DO UPDATE` branch (which preserves the
/// row id) could have the losing caller's attempt land on the winner's row.
#[sqlx::test]
async fn test_start_mints_first_attempt_with_row(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    let steps = vec![vec![VpnClientMfaMethod::MobileApprove]];
    let challenge = BiometricChallenge::new();

    let mut tx = pool.begin().await.unwrap();
    let (session, outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps.clone(),
        VpnClientMfaMethod::MobileApprove,
        Some(challenge.clone()),
        Duration::from_mins(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // The row comes back already initialized: no follow-up write is needed to begin the attempt.
    let state = session
        .ephemeral_state
        .clone()
        .expect("start must return a session carrying its first attempt")
        .0;
    assert_eq!(state.step_attempt_id, outcome.step_attempt_id);
    assert_eq!(state.selected_method, VpnClientMfaMethod::MobileApprove);
    assert_eq!(
        state.biometric_challenge.as_ref().map(|c| &c.challenge),
        Some(&challenge.challenge)
    );
    assert!(!state.openid_auth_completed);
    assert!(!state.mobile_approved);

    // What was returned is what was persisted.
    let persisted = refetch(&pool, &outcome.token)
        .await
        .ephemeral_state
        .unwrap()
        .0;
    assert_eq!(persisted, state);

    // Superseding rewrites the attempt in that same write, so the surviving token is always
    // paired with the attempt its own `start` minted.
    let mut tx = pool.begin().await.unwrap();
    let (_second, second_outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps,
        VpnClientMfaMethod::Totp,
        None,
        Duration::from_mins(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let persisted = refetch(&pool, &second_outcome.token)
        .await
        .ephemeral_state
        .unwrap()
        .0;
    assert_ne!(second_outcome.step_attempt_id, outcome.step_attempt_id);
    assert_eq!(persisted.step_attempt_id, second_outcome.step_attempt_id);
    assert_eq!(persisted.selected_method, VpnClientMfaMethod::Totp);
    assert!(persisted.biometric_challenge.is_none());
}

#[sqlx::test]
async fn test_find_active_by_token_rejects_expired(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;

    let mut tx = pool.begin().await.unwrap();
    let (_session, outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        vec![vec![VpnClientMfaMethod::Totp]],
        VpnClientMfaMethod::Totp,
        None,
        Duration::ZERO,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &outcome.token)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn test_find_active_by_token_rejects_unknown(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, "nonexistent-token")
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn test_advance_clears_ephemeral_state(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (session, outcome) = start_session(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    session
        .begin_attempt(&mut tx, VpnClientMfaMethod::Totp, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(
        refetch(&pool, &outcome.token)
            .await
            .ephemeral_state
            .is_some()
    );

    let session = refetch(&pool, &outcome.token).await;
    let mut tx = pool.begin().await.unwrap();
    let (result, _) = session
        .advance(
            &mut tx,
            session.current_step,
            None,
            VpnClientMfaMethod::Totp,
            None,
        )
        .await
        .unwrap()
        .expect("advance should match the current step");
    tx.commit().await.unwrap();
    assert_eq!(result, StepOutcome::Advanced { next_step: 1 });

    let session = refetch(&pool, &outcome.token).await;
    assert!(session.ephemeral_state.is_none());
    assert_eq!(session.current_step, 1);
    assert_eq!(session.failed_attempts, 0);
}

#[sqlx::test]
async fn test_advance_records_satisfied_method(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (session, outcome) = start_session(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    session
        .begin_attempt(&mut tx, VpnClientMfaMethod::MobileApprove, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let session = refetch(&pool, &outcome.token).await;
    let mut tx = pool.begin().await.unwrap();
    let (result, _) = session
        .advance(
            &mut tx,
            session.current_step,
            None,
            VpnClientMfaMethod::MobileApprove,
            Some("phone"),
        )
        .await
        .unwrap()
        .expect("advance should match the current step");
    tx.commit().await.unwrap();
    assert_eq!(result, StepOutcome::Advanced { next_step: 1 });

    let snapshot = &refetch(&pool, &outcome.token).await.steps_snapshot.0;
    assert_eq!(
        snapshot.steps[0].satisfied,
        Some(VpnClientMfaMethod::MobileApprove)
    );
    assert_eq!(
        snapshot.steps[0].mobile_auth_device_name.as_deref(),
        Some("phone")
    );
    assert_eq!(snapshot.steps[1].satisfied, None);
    assert_eq!(snapshot.steps[1].mobile_auth_device_name, None);
}

#[sqlx::test]
async fn test_advance_does_not_extend_expiry(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (session, outcome) = start_session(&pool).await;
    let original_expiry = session.expires_at;

    let mut tx = pool.begin().await.unwrap();
    session
        .advance(
            &mut tx,
            session.current_step,
            None,
            VpnClientMfaMethod::Totp,
            None,
        )
        .await
        .unwrap()
        .expect("advance should match the current step");
    tx.commit().await.unwrap();

    assert_eq!(
        refetch(&pool, &outcome.token).await.expires_at,
        original_expiry
    );
}

#[sqlx::test]
async fn test_advance_guards_against_stale_step(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (session, outcome) = start_session(&pool).await;

    // A wrong attempt id matches zero rows, so a proof bound to a superseded attempt cannot
    // advance the step.
    let mut tx = pool.begin().await.unwrap();
    let wrong_attempt = session
        .advance(
            &mut tx,
            session.current_step,
            Some("not-the-attempt-id"),
            VpnClientMfaMethod::Totp,
            None,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(wrong_attempt.is_none());

    // The correct attempt id advances the step.
    let mut tx = pool.begin().await.unwrap();
    let advanced = session
        .advance(
            &mut tx,
            session.current_step,
            Some(&outcome.step_attempt_id),
            VpnClientMfaMethod::Totp,
            None,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(advanced.is_some());

    // A second advance from the now-stale step matches zero rows, so a duplicate proof cannot
    // skip a step.
    let mut tx = pool.begin().await.unwrap();
    let stale = session
        .advance(
            &mut tx,
            session.current_step,
            None,
            VpnClientMfaMethod::Totp,
            None,
        )
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(stale.is_none());
}

#[sqlx::test]
async fn test_increment_failed_attempts_caps_at_five(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (session, outcome) = start_session(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    let mut at_cap = false;
    for i in 0..MFA_FAILED_ATTEMPT_CAP {
        at_cap = session.increment_failed_attempts(&mut tx).await.unwrap();
        if i + 1 < MFA_FAILED_ATTEMPT_CAP {
            assert!(!at_cap);
        }
    }
    tx.commit().await.unwrap();
    assert!(at_cap);

    assert_eq!(
        refetch(&pool, &outcome.token).await.failed_attempts,
        MFA_FAILED_ATTEMPT_CAP
    );
}

#[sqlx::test]
async fn test_mark_oidc_completed_ignores_stale_attempt(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (session, outcome) = start_session(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    let attempt_id = session
        .begin_attempt(&mut tx, VpnClientMfaMethod::Oidc, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let session = refetch(&pool, &outcome.token).await;
    let mut tx = pool.begin().await.unwrap();
    assert!(
        !session
            .mark_oidc_completed(&mut tx, "stale-id")
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();
    assert!(
        !refetch(&pool, &outcome.token)
            .await
            .ephemeral_state
            .unwrap()
            .openid_auth_completed
    );

    let session = refetch(&pool, &outcome.token).await;
    let mut tx = pool.begin().await.unwrap();
    assert!(
        session
            .mark_oidc_completed(&mut tx, &attempt_id)
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();
    assert!(
        refetch(&pool, &outcome.token)
            .await
            .ephemeral_state
            .unwrap()
            .openid_auth_completed
    );
}

#[sqlx::test]
async fn test_mark_mobile_approved_ignores_stale_attempt(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (session, outcome) = start_session(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    let attempt_id = session
        .begin_attempt(&mut tx, VpnClientMfaMethod::MobileApprove, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let session = refetch(&pool, &outcome.token).await;
    let mut tx = pool.begin().await.unwrap();
    assert!(
        !session
            .mark_mobile_approved(&mut tx, "stale-id", Some("phone"))
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();
    assert!(
        !refetch(&pool, &outcome.token)
            .await
            .ephemeral_state
            .unwrap()
            .mobile_approved
    );

    let session = refetch(&pool, &outcome.token).await;
    let mut tx = pool.begin().await.unwrap();
    assert!(
        session
            .mark_mobile_approved(&mut tx, &attempt_id, Some("phone"))
            .await
            .unwrap()
    );
    tx.commit().await.unwrap();
    let state = refetch(&pool, &outcome.token)
        .await
        .ephemeral_state
        .unwrap();
    assert!(state.mobile_approved);
    assert_eq!(state.mobile_auth_device_name.as_deref(), Some("phone"));
}

#[sqlx::test]
async fn test_begin_attempt_replaces_prior_attempt(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (session, outcome) = start_session(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    let first = session
        .begin_attempt(&mut tx, VpnClientMfaMethod::Totp, None)
        .await
        .unwrap();
    let second = session
        .begin_attempt(&mut tx, VpnClientMfaMethod::Email, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_ne!(first, second);

    let state = refetch(&pool, &outcome.token)
        .await
        .ephemeral_state
        .unwrap();
    assert_eq!(state.step_attempt_id, second);
    assert_eq!(state.selected_method, VpnClientMfaMethod::Email);
}

#[sqlx::test]
async fn test_reap_expired_deletes_only_expired(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (_expired, expired_outcome) = start_session_with_ttl(&pool, Duration::ZERO).await;
    let (_active, active_outcome) = start_session_with_ttl(&pool, Duration::from_mins(10)).await;

    let reaped = reap_expired(&pool).await.unwrap();
    assert_eq!(reaped, 1);
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &active_outcome.token)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &expired_outcome.token)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn test_concurrent_starts_leave_single_row(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    let steps = vec![vec![VpnClientMfaMethod::Totp]];

    let mut conn_a = pool.acquire().await.unwrap();
    let mut conn_b = pool.acquire().await.unwrap();

    let (a, b) = tokio::join!(
        VpnClientMfaSession::<Id>::start(
            &mut conn_a,
            location.id,
            device.id,
            user.id,
            1,
            steps.clone(),
            VpnClientMfaMethod::Totp,
            None,
            Duration::from_mins(10),
        ),
        VpnClientMfaSession::<Id>::start(
            &mut conn_b,
            location.id,
            device.id,
            user.id,
            1,
            steps.clone(),
            VpnClientMfaMethod::Totp,
            None,
            Duration::from_mins(10),
        ),
    );
    let ((_, a_outcome), (_, b_outcome)) = (a.unwrap(), b.unwrap());

    // Exactly one live row for this (location, device), regardless of interleaving.
    let count = sqlx::query_scalar!(
        "SELECT count(*) FROM vpn_client_mfa_session WHERE location_id = $1 AND device_id = $2",
        location.id,
        device.id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, Some(1));

    // Exactly one of the two minted tokens survives; the other was superseded.
    let a_live = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &a_outcome.token)
        .await
        .unwrap()
        .is_some();
    let b_live = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &b_outcome.token)
        .await
        .unwrap()
        .is_some();
    assert_ne!(
        a_live, b_live,
        "exactly one token must survive concurrent start"
    );
}

#[sqlx::test]
async fn test_same_device_two_locations_both_live(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    let location_a = create_location_with_address(&pool, "10.0.6.1/24").await;
    let location_b = create_location_with_address(&pool, "10.0.7.1/24").await;
    let steps = vec![vec![VpnClientMfaMethod::Totp]];

    let mut conn_a = pool.acquire().await.unwrap();
    let mut conn_b = pool.acquire().await.unwrap();

    let (a, b) = tokio::join!(
        VpnClientMfaSession::<Id>::start(
            &mut conn_a,
            location_a.id,
            device.id,
            user.id,
            1,
            steps.clone(),
            VpnClientMfaMethod::Totp,
            None,
            Duration::from_mins(10),
        ),
        VpnClientMfaSession::<Id>::start(
            &mut conn_b,
            location_b.id,
            device.id,
            user.id,
            1,
            steps.clone(),
            VpnClientMfaMethod::Totp,
            None,
            Duration::from_mins(10),
        ),
    );
    let ((_, a_outcome), (_, b_outcome)) = (a.unwrap(), b.unwrap());

    // Uniqueness is per (location, device), so both rows stay live.
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &a_outcome.token)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &b_outcome.token)
            .await
            .unwrap()
            .is_some()
    );
}
