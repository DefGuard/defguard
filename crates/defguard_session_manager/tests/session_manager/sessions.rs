use std::net::SocketAddr;

use chrono::{TimeDelta, Utc};
use defguard_common::db::{
    models::{
        vpn_client_session::{VpnClientSession, VpnClientSessionState},
        vpn_session_stats::VpnSessionStats,
    },
    setup_pool,
};
use defguard_session_manager::events::SessionManagerEventType;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::{Duration, timeout};

use crate::common::{
    SessionManagerHarness, assert_no_gateway_events, assert_no_session_manager_events,
    attach_device_to_location, build_stats_update, count_session_stats,
    count_stats_for_device_location, create_device, create_device_with_pubkey, create_gateway,
    create_location, create_session, create_session_stats, create_user, stale_session_timestamp,
    truncate_timestamp,
};

const RECEIVE_TIMEOUT: Duration = Duration::from_secs(1);

#[sqlx::test]
async fn test_session_manager_creates_connected_session_from_first_stats(
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

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let handshake = truncate_timestamp(Utc::now().naive_utc() - TimeDelta::seconds(5));
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

    let session = VpnClientSession::try_get_active_session(&pool, location.id, device.id)
        .await
        .expect("failed to query active session")
        .expect("expected active session");
    assert_eq!(session.state, VpnClientSessionState::Connected);
    assert_eq!(session.connected_at, Some(handshake));
}

#[sqlx::test]
async fn test_stale_first_stats_update_does_not_create_session_or_stats(
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

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let collected_at = truncate_timestamp(Utc::now().naive_utc());
    let stale_handshake = stale_session_timestamp(&location);
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        collected_at,
        endpoint,
        100,
        200,
        stale_handshake,
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
    assert_no_session_manager_events(&mut harness);
    assert_no_gateway_events(&mut harness);
}

#[sqlx::test]
async fn test_duplicate_stats_in_same_batch_reuse_existing_session(
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

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let base_time = Utc::now().naive_utc();
    let duplicate_update = || {
        build_stats_update(
            location.id,
            gateway.id,
            &device.wireguard_pubkey,
            base_time,
            endpoint,
            100,
            200,
            base_time,
        )
    };
    harness.send_stats(duplicate_update());
    harness.send_stats(duplicate_update());

    let _ = harness.run_iteration().await;

    let connected_event = timeout(RECEIVE_TIMEOUT, harness.event_rx.recv())
        .await
        .expect("timed out waiting for ClientConnected event in duplicate same-batch stats test")
        .expect("session manager event channel closed while waiting for duplicate same-batch stats event");
    assert!(matches!(
        connected_event.event,
        SessionManagerEventType::ClientConnected
    ));
    assert_eq!(connected_event.context.location.id, location.id);
    assert_eq!(connected_event.context.user.id, user.id);
    assert_eq!(connected_event.context.device.id, device.id);
    assert_eq!(connected_event.context.public_ip, endpoint.ip());
    assert_no_session_manager_events(&mut harness);
    assert_no_gateway_events(&mut harness);

    let active_session = VpnClientSession::try_get_active_session(&pool, location.id, device.id)
        .await
        .expect("failed to query active session")
        .expect("expected active session");

    let sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query active sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, active_session.id);

    let session = sessions.first().expect("expected active session");
    assert_eq!(count_session_stats(&pool, session.id).await, 2);

    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, session.id);
    assert_eq!(latest_stats.upload_diff, 0);
    assert_eq!(latest_stats.download_diff, 0);
}

#[sqlx::test]
async fn test_duplicate_stats_across_iterations_reuse_existing_session(
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

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let base_time = Utc::now().naive_utc();
    let update = build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time,
        endpoint,
        100,
        200,
        base_time,
    );

    harness.send_stats(update);
    let _ = harness.run_iteration().await;

    let first_session = VpnClientSession::try_get_active_session(&pool, location.id, device.id)
        .await
        .expect("failed to query active session")
        .expect("expected active session");

    let connected_event = timeout(RECEIVE_TIMEOUT, harness.event_rx.recv())
        .await
        .expect(
            "timed out waiting for ClientConnected event in duplicate cross-iteration stats test",
        )
        .expect(
            "session manager event channel closed while waiting for duplicate cross-iteration stats event",
        );
    assert!(matches!(
        connected_event.event,
        SessionManagerEventType::ClientConnected
    ));
    assert_eq!(connected_event.context.location.id, location.id);
    assert_eq!(connected_event.context.user.id, user.id);
    assert_eq!(connected_event.context.device.id, device.id);
    assert_eq!(connected_event.context.public_ip, endpoint.ip());
    assert_no_session_manager_events(&mut harness);
    assert_no_gateway_events(&mut harness);

    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time,
        endpoint,
        100,
        200,
        base_time,
    ));
    let _ = harness.run_iteration().await;

    assert_no_session_manager_events(&mut harness);
    assert_no_gateway_events(&mut harness);

    let sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query active sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, first_session.id);
    assert_eq!(count_session_stats(&pool, first_session.id).await, 2);

    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, first_session.id);
    assert_eq!(latest_stats.upload_diff, 0);
    assert_eq!(latest_stats.download_diff, 0);
}

#[sqlx::test]
async fn test_existing_new_session_becomes_connected_on_stats(
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

    let existing_session =
        create_session(&pool, location.id, user.id, device.id, None, None, None).await;
    assert_eq!(existing_session.state, VpnClientSessionState::New);

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

    let updated_session = VpnClientSession::find_by_id(&pool, existing_session.id)
        .await
        .expect("failed to query session")
        .expect("expected session");
    assert_eq!(updated_session.state, VpnClientSessionState::Connected);
    assert_eq!(updated_session.connected_at, Some(handshake));
    assert_eq!(count_session_stats(&pool, updated_session.id).await, 1);
}

#[sqlx::test]
async fn test_invalid_device_pubkey_updates_are_discarded(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device_with_pubkey(&pool, user.id, "device-pubkey-valid").await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let timestamp = Utc::now().naive_utc();
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        "missing-pubkey",
        timestamp,
        endpoint,
        100,
        200,
        timestamp,
    ));

    let _ = harness.run_iteration().await;

    let maybe_session = VpnClientSession::try_get_active_session(&pool, location.id, device.id)
        .await
        .expect("failed to query active session");
    assert!(maybe_session.is_none());
    assert_eq!(
        count_stats_for_device_location(&pool, device.id, location.id).await,
        0
    );
}

#[sqlx::test]
async fn test_out_of_order_peer_updates_are_discarded(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let base_time = Utc::now().naive_utc();
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time,
        endpoint,
        100,
        200,
        base_time,
    ));
    let _ = harness.run_iteration().await;

    let session = VpnClientSession::try_get_active_session(&pool, location.id, device.id)
        .await
        .expect("failed to query active session")
        .expect("expected active session");
    assert_eq!(count_session_stats(&pool, session.id).await, 1);

    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time - TimeDelta::seconds(1),
        endpoint,
        150,
        260,
        base_time + TimeDelta::seconds(1),
    ));
    let _ = harness.run_iteration().await;
    assert_eq!(count_session_stats(&pool, session.id).await, 1);

    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time + TimeDelta::seconds(2),
        endpoint,
        150,
        260,
        base_time - TimeDelta::seconds(1),
    ));
    let _ = harness.run_iteration().await;
    assert_eq!(count_session_stats(&pool, session.id).await, 1);

    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time + TimeDelta::seconds(3),
        endpoint,
        90,
        190,
        base_time + TimeDelta::seconds(3),
    ));
    let _ = harness.run_iteration().await;
    assert_eq!(count_session_stats(&pool, session.id).await, 1);
}

#[sqlx::test]
async fn test_device_public_key_change_reuses_existing_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let mut device =
        create_device_with_pubkey(&pool, user.id, "device-pubkey-before-rotation").await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway = create_gateway(&pool, location.id, user.fullname()).await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let base_time = Utc::now().naive_utc();
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time,
        endpoint,
        100,
        200,
        base_time,
    ));
    let _ = harness.run_iteration().await;

    let existing_session = VpnClientSession::try_get_active_session(&pool, location.id, device.id)
        .await
        .expect("failed to query active session")
        .expect("expected active session");

    device.wireguard_pubkey = "device-pubkey-after-rotation".to_string();
    device
        .save(&pool)
        .await
        .expect("failed to update device pubkey");

    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time + TimeDelta::seconds(10),
        endpoint,
        150,
        260,
        base_time + TimeDelta::seconds(10),
    ));
    let _ = harness.run_iteration().await;

    let sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query active sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, existing_session.id);
    assert_eq!(count_session_stats(&pool, existing_session.id).await, 2);

    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, existing_session.id);
    assert_eq!(latest_stats.total_upload, 150);
    assert_eq!(latest_stats.total_download, 260);
}

#[sqlx::test]
async fn test_existing_session_in_db_is_reused_instead_of_creating_duplicate(
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

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let base_time = Utc::now().naive_utc();
    let existing_session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        Some(base_time - TimeDelta::seconds(5)),
        None,
        None,
    )
    .await;
    create_session_stats(
        &pool,
        existing_session.id,
        gateway.id,
        base_time - TimeDelta::seconds(5),
        base_time - TimeDelta::seconds(5),
        endpoint,
        100,
        200,
        0,
        0,
    )
    .await;

    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time,
        endpoint,
        160,
        280,
        base_time,
    ));
    let _ = harness.run_iteration().await;

    let sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query active sessions");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].id, existing_session.id);

    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, existing_session.id);
    assert_eq!(latest_stats.upload_diff, 60);
    assert_eq!(latest_stats.download_diff, 80);
}
