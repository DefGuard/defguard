use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::db::{
    Id,
    models::{
        device::{Device, DeviceType},
        user::User,
        vpn_client_session::VpnClientMfaMethod,
        wireguard::WireguardNetwork,
    },
    setup_pool,
};

async fn create_location(pool: &sqlx::PgPool) -> WireguardNetwork<Id> {
    WireguardNetwork::default()
        .try_set_address("10.0.6.1/24")
        .unwrap()
        .save(pool)
        .await
        .unwrap()
}

async fn create_user(pool: &sqlx::PgPool) -> User<Id> {
    User::new("mfa-session-user", None, "Ln", "Fn", "m@t.com", None)
        .save(pool)
        .await
        .unwrap()
}

async fn create_device(pool: &sqlx::PgPool, user_id: Id) -> Device<Id> {
    Device::new(
        "mfa-session-device".into(),
        "device-pubkey".into(),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn test_start_supersedes_existing_session(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    let steps = vec![vec![VpnClientMfaMethod::Totp]];

    let mut tx = pool.begin().await.unwrap();
    let (first, first_outcome) = VpnClientMfaSession::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps.clone(),
        Duration::from_mins(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(first.current_step_methods(), [VpnClientMfaMethod::Totp]);
    // The raw token is never stored; only its hash is.
    assert_eq!(first.token_hash, token_hash(&first_outcome.token));
    assert_ne!(first.token_hash, first_outcome.token);
    assert!(
        VpnClientMfaSession::find_active_by_token(&pool, &first_outcome.token)
            .await
            .is_some()
    );

    let mut tx = pool.begin().await.unwrap();
    let (_second, second_outcome) = VpnClientMfaSession::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps,
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
        VpnClientMfaSession::find_active_by_token(&pool, &first_outcome.token)
            .await
            .is_none()
    );
    assert!(
        VpnClientMfaSession::find_active_by_token(&pool, &second_outcome.token)
            .await
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
    let (first, _) = VpnClientMfaSession::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps.clone(),
        Duration::from_mins(10),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let (_, outcome) = VpnClientMfaSession::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        steps,
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

#[sqlx::test]
async fn test_find_active_by_token_rejects_expired(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;

    let mut tx = pool.begin().await.unwrap();
    let (_session, outcome) = VpnClientMfaSession::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        vec![vec![VpnClientMfaMethod::Totp]],
        Duration::ZERO,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert!(
        VpnClientMfaSession::find_active_by_token(&pool, &outcome.token)
            .await
            .is_none()
    );
}

#[sqlx::test]
async fn test_find_active_by_token_rejects_unknown(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    assert!(
        VpnClientMfaSession::find_active_by_token(&pool, "nonexistent-token")
            .await
            .is_none()
    );
}
