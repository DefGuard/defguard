use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
    query, query_scalar,
};

use super::*;
use crate::db::{
    models::{
        device::{Device, DeviceType},
        user::User,
    },
    setup_pool,
};

async fn create_user(pool: &PgPool, username: &str) -> User<Id> {
    User::new(
        username.to_owned(),
        None,
        "Ln".to_owned(),
        "Fn".to_owned(),
        format!("{username}@test.com"),
        None,
    )
    .save(pool)
    .await
    .unwrap()
}

async fn create_device(pool: &PgPool, user_id: Id) -> Device<Id> {
    Device::new(
        "laptop".to_owned(),
        "device-pubkey".to_owned(),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap()
}

async fn start(pool: &PgPool, device_id: Id, started_by: Id) -> (DeviceVerification<Id>, bool) {
    let mut conn = pool.acquire().await.unwrap();
    DeviceVerification::new(device_id, Some(started_by))
        .start_or_get(&mut conn)
        .await
        .unwrap()
}

async fn expire(pool: &PgPool, device_id: Id) {
    query!(
        "UPDATE device_verification SET expires_at = (now() AT TIME ZONE 'UTC') - interval '1 second' \
        WHERE device_id = $1",
        device_id
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn count(pool: &PgPool) -> i64 {
    query_scalar!("SELECT count(*) \"count!\" FROM device_verification")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test]
async fn test_start_creates_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let admin = create_user(&pool, "admin1").await;
    let owner = create_user(&pool, "owner1").await;
    let device = create_device(&pool, owner.id).await;

    let (verification, created) = start(&pool, device.id, admin.id).await;

    assert!(created);
    assert_eq!(verification.device_id, device.id);
    assert_eq!(verification.started_by, Some(admin.id));
    assert_eq!(verification.secret.len(), 20);
    assert_eq!(
        verification.expires_at - verification.created_at,
        DEVICE_VERIFICATION_TIMEOUT
    );
    let active = DeviceVerification::find_active_for_device(&pool, device.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.id, verification.id);
}

#[sqlx::test]
async fn test_start_returns_active_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let admin = create_user(&pool, "admin1").await;
    let other_admin = create_user(&pool, "admin2").await;
    let owner = create_user(&pool, "owner1").await;
    let device = create_device(&pool, owner.id).await;

    let (first, _) = start(&pool, device.id, admin.id).await;
    let (second, created) = start(&pool, device.id, other_admin.id).await;

    assert!(!created);
    assert_eq!(second.id, first.id);
    assert_eq!(second.secret, first.secret);
    assert_eq!(second.started_by, Some(admin.id));
    assert_eq!(second.expires_at, first.expires_at);
    assert_eq!(count(&pool).await, 1);
}

#[sqlx::test]
async fn test_start_replaces_expired_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let admin = create_user(&pool, "admin1").await;
    let other_admin = create_user(&pool, "admin2").await;
    let owner = create_user(&pool, "owner1").await;
    let device = create_device(&pool, owner.id).await;

    let (first, _) = start(&pool, device.id, admin.id).await;
    expire(&pool, device.id).await;
    assert!(
        DeviceVerification::find_active_for_device(&pool, device.id)
            .await
            .unwrap()
            .is_none()
    );

    let (second, created) = start(&pool, device.id, other_admin.id).await;

    assert!(created);
    assert_ne!(second.id, first.id);
    assert_ne!(second.secret, first.secret);
    assert_eq!(second.started_by, Some(other_admin.id));
    assert!(second.expires_at > first.expires_at);
    assert_eq!(count(&pool).await, 1);
}

#[sqlx::test]
async fn test_resolve_active_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let admin = create_user(&pool, "admin1").await;
    let owner = create_user(&pool, "owner1").await;
    let device = create_device(&pool, owner.id).await;

    assert!(
        DeviceVerification::resolve_active_for_device(
            &pool,
            device.id,
            DeviceVerificationOutcome::Cancelled
        )
        .await
        .unwrap()
        .is_none()
    );

    let (verification, _) = start(&pool, device.id, admin.id).await;
    let resolved = DeviceVerification::resolve_active_for_device(
        &pool,
        device.id,
        DeviceVerificationOutcome::Cancelled,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(resolved.id, verification.id);
    assert_eq!(resolved.outcome, Some(DeviceVerificationOutcome::Cancelled));
    assert!(resolved.resolved_at.is_some());
    assert_eq!(count(&pool).await, 1);
    assert!(
        DeviceVerification::find_active_for_device(&pool, device.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        DeviceVerification::resolve_active_for_device(
            &pool,
            device.id,
            DeviceVerificationOutcome::Verified
        )
        .await
        .unwrap()
        .is_none()
    );
    let stored = DeviceVerification::find_for_device(&pool, verification.id, device.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.outcome, Some(DeviceVerificationOutcome::Cancelled));

    let (restarted, created) = start(&pool, device.id, admin.id).await;
    assert!(created);
    assert_ne!(restarted.id, verification.id);
    assert_ne!(restarted.secret, verification.secret);
    assert_eq!(restarted.outcome, None);
    assert_eq!(count(&pool).await, 1);
    assert!(
        DeviceVerification::find_for_device(&pool, verification.id, device.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn test_resolve_ignores_expired_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let admin = create_user(&pool, "admin1").await;
    let owner = create_user(&pool, "owner1").await;
    let device = create_device(&pool, owner.id).await;

    let (verification, _) = start(&pool, device.id, admin.id).await;
    expire(&pool, device.id).await;

    assert!(
        DeviceVerification::resolve_active_for_device(
            &pool,
            device.id,
            DeviceVerificationOutcome::Blocked
        )
        .await
        .unwrap()
        .is_none()
    );
    let stored = DeviceVerification::find_for_device(&pool, verification.id, device.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.outcome, None);
}

#[sqlx::test]
async fn test_find_for_device_checks_owner(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let admin = create_user(&pool, "admin1").await;
    let owner = create_user(&pool, "owner1").await;
    let device = create_device(&pool, owner.id).await;
    let other_device = Device::new(
        "phone".to_owned(),
        "other-device-pubkey".to_owned(),
        owner.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let (verification, _) = start(&pool, device.id, admin.id).await;

    assert!(
        DeviceVerification::find_for_device(&pool, verification.id, other_device.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn test_verification_follows_device_and_user_removal(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let admin = create_user(&pool, "admin1").await;
    let owner = create_user(&pool, "owner1").await;
    let device = create_device(&pool, owner.id).await;

    let (verification, _) = start(&pool, device.id, admin.id).await;
    admin.delete(&pool).await.unwrap();
    let verification = DeviceVerification::find_by_id(&pool, verification.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(verification.started_by, None);

    device.delete(&pool).await.unwrap();
    assert_eq!(count(&pool).await, 0);
}

#[test]
fn test_code_generation() {
    let verification = DeviceVerification::new(1, None).with_id(1);
    let window_start = 1_700_000_040;
    assert_eq!(window_start % DEVICE_VERIFICATION_CODE_PERIOD, 0);

    let first = verification.code_at(window_start);
    assert_eq!(first.code.len(), DEVICE_VERIFICATION_CODE_DIGITS as usize);
    assert!(first.code.chars().all(|c| c.is_ascii_digit()));
    assert_eq!(first.expires_in, DEVICE_VERIFICATION_CODE_PERIOD);
    assert_eq!(
        first.code,
        totp_custom::<Sha1>(60, 6, &verification.secret, window_start)
    );

    let same_window = verification.code_at(window_start + 59);
    assert_eq!(same_window.code, first.code);
    assert_eq!(same_window.expires_in, 1);

    let next_window = verification.code_at(window_start + 60);
    assert_eq!(
        next_window.code,
        totp_custom::<Sha1>(60, 6, &verification.secret, window_start + 60)
    );
    assert_eq!(next_window.expires_in, DEVICE_VERIFICATION_CODE_PERIOD);
}

#[test]
fn test_code_matches_rfc6238_vector() {
    let mut verification = DeviceVerification::new(1, None).with_id(1);
    verification.secret = b"12345678901234567890".to_vec();

    // RFC 6238 SHA1 vector 94287082 is counter 1, which a 60s step reaches at T = 60..119
    assert_eq!(verification.code_at(118).code, "287082");
}

#[test]
fn test_state_at() {
    let mut verification = DeviceVerification::new(1, None).with_id(1);
    let before_expiry = verification.expires_at - TimeDelta::seconds(1);

    assert_eq!(
        verification.state_at(before_expiry),
        DeviceVerificationState::Pending
    );
    assert_eq!(
        verification.state_at(verification.expires_at),
        DeviceVerificationState::Expired
    );

    verification.outcome = Some(DeviceVerificationOutcome::Blocked);
    assert_eq!(
        verification.state_at(verification.expires_at),
        DeviceVerificationState::Resolved(DeviceVerificationOutcome::Blocked)
    );
}
