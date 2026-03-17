use std::net::SocketAddr;

use chrono::{TimeDelta, Utc};
use defguard_common::db::{
    models::{
        device::WireguardNetworkDevice,
        vpn_client_session::{VpnClientMfaMethod, VpnClientSession, VpnClientSessionState},
        vpn_session_stats::VpnSessionStats,
        wireguard::LocationMfaMode,
    },
    setup_pool,
};
use defguard_core::grpc::GatewayEvent;
use defguard_session_manager::events::SessionManagerEventType;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::{Duration, timeout};

use crate::common::{
    SessionManagerHarness, assert_no_gateway_events, assert_no_session_manager_events,
    attach_device_to_location, authorize_device_in_location, build_stats_update,
    count_session_stats, count_stats_for_device_location, create_device, create_gateway,
    create_location_with_mfa_mode, create_session, create_session_stats, create_user,
    set_session_created_at, stale_session_timestamp, truncate_timestamp,
};

const RECEIVE_TIMEOUT: Duration = Duration::from_secs(1);

#[sqlx::test]
async fn test_mfa_location_stats_do_not_create_missing_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location_with_mfa_mode(&pool, LocationMfaMode::Internal).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let timestamp = Utc::now().naive_utc();
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        timestamp,
        endpoint,
        100,
        200,
        timestamp,
    ));

    let _ = harness.run_iteration().await;

    assert!(
        VpnClientSession::try_get_active_session(&pool, location.id, device.id)
            .await
            .expect("failed to query active session")
            .is_none()
    );
    assert_eq!(
        count_stats_for_device_location(&pool, device.id, location.id).await,
        0
    );
}

#[sqlx::test]
async fn test_mfa_new_session_upgrades_to_connected_on_stats(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location_with_mfa_mode(&pool, LocationMfaMode::Internal).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        None,
        Some(VpnClientMfaMethod::Totp),
    )
    .await;

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let handshake = truncate_timestamp(Utc::now().naive_utc());
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        handshake,
        endpoint,
        100,
        200,
        handshake,
    ));

    let _ = harness.run_iteration().await;

    let refreshed_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(refreshed_session.state, VpnClientSessionState::Connected);
    assert_eq!(refreshed_session.connected_at, Some(handshake));
    assert_eq!(
        count_stats_for_device_location(&pool, device.id, location.id).await,
        1
    );

    let connected_event = timeout(RECEIVE_TIMEOUT, harness.event_rx.recv())
        .await
        .expect("timed out waiting for MfaClientConnected event")
        .expect("session manager event channel closed");
    assert!(matches!(
        connected_event.event,
        SessionManagerEventType::MfaClientConnected
    ));
    assert_eq!(connected_event.context.location.id, location.id);
    assert_eq!(connected_event.context.user.id, user.id);
    assert_eq!(connected_event.context.device.id, device.id);
    assert_eq!(connected_event.context.public_ip, endpoint.ip());

    let second_collected_at = handshake + TimeDelta::seconds(30);
    let second_handshake = handshake + TimeDelta::seconds(25);
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        second_collected_at,
        endpoint,
        160,
        280,
        second_handshake,
    ));

    let _ = harness.run_iteration().await;

    let updated_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(updated_session.state, VpnClientSessionState::Connected);
    assert_eq!(updated_session.connected_at, Some(handshake));

    let active_sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query active sessions");
    assert_eq!(active_sessions.len(), 1);
    assert_eq!(active_sessions[0].id, session.id);

    assert_eq!(count_session_stats(&pool, session.id).await, 2);
    assert_eq!(
        count_stats_for_device_location(&pool, device.id, location.id).await,
        2
    );

    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, session.id);
    assert_eq!(latest_stats.total_upload, 160);
    assert_eq!(latest_stats.total_download, 280);
    assert_eq!(latest_stats.upload_diff, 60);
    assert_eq!(latest_stats.download_diff, 80);

    assert_no_session_manager_events(&mut harness);
    assert_no_gateway_events(&mut harness);
}

#[sqlx::test]
async fn test_duplicate_first_stats_on_mfa_new_session_are_idempotent(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location_with_mfa_mode(&pool, LocationMfaMode::Internal).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        None,
        Some(VpnClientMfaMethod::Totp),
    )
    .await;

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let handshake = truncate_timestamp(Utc::now().naive_utc());
    let duplicate_update = || {
        build_stats_update(
            location.id,
            gateway.id,
            &device.wireguard_pubkey,
            handshake,
            endpoint,
            100,
            200,
            handshake,
        )
    };

    harness.send_stats(duplicate_update());
    harness.send_stats(duplicate_update());

    let _ = harness.run_iteration().await;

    let refreshed_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(refreshed_session.state, VpnClientSessionState::Connected);
    assert_eq!(refreshed_session.connected_at, Some(handshake));

    let active_sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query active sessions");
    assert_eq!(active_sessions.len(), 1);
    assert_eq!(active_sessions[0].id, session.id);

    assert_eq!(count_session_stats(&pool, session.id).await, 2);

    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, session.id);
    assert_eq!(latest_stats.upload_diff, 0);
    assert_eq!(latest_stats.download_diff, 0);

    let connected_event = timeout(RECEIVE_TIMEOUT, harness.event_rx.recv())
        .await
        .expect("timed out waiting for MfaClientConnected event in duplicate first-stats test")
        .expect("session manager event channel closed");
    assert!(matches!(
        connected_event.event,
        SessionManagerEventType::MfaClientConnected
    ));
    assert_eq!(connected_event.context.location.id, location.id);
    assert_eq!(connected_event.context.user.id, user.id);
    assert_eq!(connected_event.context.device.id, device.id);
    assert_eq!(connected_event.context.public_ip, endpoint.ip());

    assert_no_session_manager_events(&mut harness);
    assert_no_gateway_events(&mut harness);
}

#[sqlx::test]
async fn test_repeated_later_stats_on_mfa_session_remain_idempotent(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location_with_mfa_mode(&pool, LocationMfaMode::Internal).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        None,
        Some(VpnClientMfaMethod::Totp),
    )
    .await;

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let first_handshake = truncate_timestamp(Utc::now().naive_utc() - TimeDelta::seconds(30));
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        first_handshake,
        endpoint,
        100,
        200,
        first_handshake,
    ));

    let _ = harness.run_iteration().await;

    let connected_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(connected_session.state, VpnClientSessionState::Connected);
    assert_eq!(connected_session.connected_at, Some(first_handshake));

    let connected_event = timeout(RECEIVE_TIMEOUT, harness.event_rx.recv())
        .await
        .expect("timed out waiting for MfaClientConnected event in repeated-stats test")
        .expect("session manager event channel closed");
    assert!(matches!(
        connected_event.event,
        SessionManagerEventType::MfaClientConnected
    ));
    assert_eq!(connected_event.context.location.id, location.id);
    assert_eq!(connected_event.context.user.id, user.id);
    assert_eq!(connected_event.context.device.id, device.id);
    assert_eq!(connected_event.context.public_ip, endpoint.ip());

    assert_no_session_manager_events(&mut harness);
    assert_no_gateway_events(&mut harness);

    let later_collected_at = first_handshake + TimeDelta::seconds(30);
    let later_handshake = first_handshake + TimeDelta::seconds(20);
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        later_collected_at,
        endpoint,
        100,
        200,
        later_handshake,
    ));

    let _ = harness.run_iteration().await;

    let refreshed_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(refreshed_session.state, VpnClientSessionState::Connected);
    assert_eq!(refreshed_session.connected_at, Some(first_handshake));

    let active_sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query active sessions");
    assert_eq!(active_sessions.len(), 1);
    assert_eq!(active_sessions[0].id, session.id);

    assert_eq!(count_session_stats(&pool, session.id).await, 2);

    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, session.id);
    assert_eq!(latest_stats.upload_diff, 0);
    assert_eq!(latest_stats.download_diff, 0);

    assert_no_session_manager_events(&mut harness);
    assert_no_gateway_events(&mut harness);
}

#[sqlx::test]
async fn test_closed_event_channel_keeps_mfa_first_stats_upgrade_idempotent(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location_with_mfa_mode(&pool, LocationMfaMode::Internal).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        None,
        Some(VpnClientMfaMethod::Totp),
    )
    .await;

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let first_handshake = truncate_timestamp(Utc::now().naive_utc() - TimeDelta::seconds(30));
    let second_collected_at = first_handshake + TimeDelta::seconds(30);
    let second_handshake = first_handshake + TimeDelta::seconds(20);

    harness.close_event_channel();
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        first_handshake,
        endpoint,
        100,
        200,
        first_handshake,
    ));
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        second_collected_at,
        endpoint,
        160,
        280,
        second_handshake,
    ));

    let _ = harness.run_iteration().await;

    let refreshed_session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(refreshed_session.state, VpnClientSessionState::Connected);
    assert_eq!(refreshed_session.connected_at, Some(first_handshake));

    assert_eq!(count_session_stats(&pool, session.id).await, 1);

    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, session.id);
    assert_eq!(latest_stats.total_upload, 160);
    assert_eq!(latest_stats.total_download, 280);
    assert_eq!(latest_stats.upload_diff, 0);
    assert_eq!(latest_stats.download_diff, 0);

    assert_no_gateway_events(&mut harness);
}

#[sqlx::test]
async fn test_inactive_mfa_connected_sessions_disconnect_and_clear_authorization(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location_with_mfa_mode(&pool, LocationMfaMode::Internal).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    authorize_device_in_location(&pool, location.id, device.id, "psk-before-disconnect").await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let stale_handshake = stale_session_timestamp(&location);
    let session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        Some(stale_handshake),
        Some(VpnClientMfaMethod::Totp),
    )
    .await;
    create_session_stats(
        &pool,
        session.id,
        gateway.id,
        stale_handshake,
        stale_handshake,
        "203.0.113.10:51820".parse().unwrap(),
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

    let network_device = WireguardNetworkDevice::find(&pool, device.id, location.id)
        .await
        .expect("failed to query network device")
        .expect("expected network device");
    assert!(!network_device.is_authorized);
    assert_eq!(network_device.preshared_key, None);

    let gateway_event = timeout(RECEIVE_TIMEOUT, harness.gateway_rx.recv())
        .await
        .expect("timed out waiting for MFA disconnect gateway event")
        .expect("gateway event channel closed");
    match gateway_event {
        GatewayEvent::MfaSessionDisconnected(location_id, disconnected_device) => {
            assert_eq!(location_id, location.id);
            assert_eq!(disconnected_device.id, device.id);
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }

    let disconnected_event = timeout(RECEIVE_TIMEOUT, harness.event_rx.recv())
        .await
        .expect("timed out waiting for MfaClientDisconnected event")
        .expect("session manager event channel closed");
    assert!(matches!(
        disconnected_event.event,
        SessionManagerEventType::MfaClientDisconnected
    ));
    assert_eq!(disconnected_event.context.location.id, location.id);
    assert_eq!(disconnected_event.context.user.id, user.id);
    assert_eq!(disconnected_event.context.device.id, device.id);
}

#[sqlx::test]
async fn test_never_connected_mfa_new_sessions_disconnect_after_threshold(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location_with_mfa_mode(&pool, LocationMfaMode::Internal).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    authorize_device_in_location(&pool, location.id, device.id, "psk-before-timeout").await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        None,
        Some(VpnClientMfaMethod::Totp),
    )
    .await;
    set_session_created_at(&pool, session.id, stale_session_timestamp(&location)).await;

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

    let network_device = WireguardNetworkDevice::find(&pool, device.id, location.id)
        .await
        .expect("failed to query network device")
        .expect("expected network device");
    assert!(!network_device.is_authorized);
    assert_eq!(network_device.preshared_key, None);

    let gateway_event = timeout(RECEIVE_TIMEOUT, harness.gateway_rx.recv())
        .await
        .expect("timed out waiting for MFA disconnect gateway event for new session")
        .expect("gateway event channel closed");
    match gateway_event {
        GatewayEvent::MfaSessionDisconnected(location_id, disconnected_device) => {
            assert_eq!(location_id, location.id);
            assert_eq!(disconnected_device.id, device.id);
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }

    assert_no_session_manager_events(&mut harness);
}
