use std::net::SocketAddr;

use chrono::{TimeDelta, Utc};
use defguard_common::db::{
    models::vpn_client_session::{VpnClientSession, VpnClientSessionState},
    setup_pool,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::common::{
    SessionManagerHarness, attach_device_to_location, create_device, create_gateway,
    create_location, create_session, create_session_stats, create_user, stale_session_timestamp,
};

#[sqlx::test]
async fn test_inactive_connected_sessions_are_disconnected_after_threshold(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let stale_handshake = stale_session_timestamp(&location);
    let session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        Some(stale_handshake),
        None,
    )
    .await;
    create_session_stats(
        &pool,
        session.id,
        gateway.id,
        stale_handshake,
        stale_handshake,
        "203.0.113.10:51820".parse::<SocketAddr>().unwrap(),
        100,
        200,
        0,
        0,
    )
    .await;

    let _ = harness.run_idle_iteration().await;

    let disconnected_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(
        disconnected_session.state,
        VpnClientSessionState::Disconnected
    );
    assert!(disconnected_session.disconnected_at.is_some());
}

#[sqlx::test]
async fn test_recent_connected_sessions_remain_active(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let recent_handshake = Utc::now().naive_utc() - TimeDelta::seconds(30);
    let session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        Some(recent_handshake),
        None,
    )
    .await;
    create_session_stats(
        &pool,
        session.id,
        gateway.id,
        recent_handshake,
        recent_handshake,
        "203.0.113.10:51820".parse::<SocketAddr>().unwrap(),
        100,
        200,
        0,
        0,
    )
    .await;

    let _ = harness.run_idle_iteration().await;

    let refreshed_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(refreshed_session.state, VpnClientSessionState::Connected);
    assert!(refreshed_session.disconnected_at.is_none());
}
