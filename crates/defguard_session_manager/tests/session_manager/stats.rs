use std::net::SocketAddr;

use chrono::{TimeDelta, Utc};
use defguard_common::db::{
    models::{vpn_client_session::VpnClientSession, vpn_session_stats::VpnSessionStats},
    setup_pool,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::common::{
    SessionManagerHarness, attach_device_to_location, build_stats_update, count_session_stats,
    create_device, create_gateway, create_gateway_named, create_location, create_session,
    create_session_stats, create_user,
};

#[sqlx::test]
async fn test_session_manager_updates_stats_deltas_across_iterations(
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
    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time,
        endpoint,
        100,
        200,
        base_time - TimeDelta::seconds(5),
    ));
    let _ = harness.run_iteration().await;

    let first_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query session stats")
        .expect("expected session stats");
    assert_eq!(first_stats.upload_diff, 0);
    assert_eq!(first_stats.download_diff, 0);

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

    let second_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query session stats")
        .expect("expected session stats");
    assert_eq!(second_stats.upload_diff, 50);
    assert_eq!(second_stats.download_diff, 60);

    harness.send_stats(build_stats_update(
        location.id,
        gateway.id,
        &device.wireguard_pubkey,
        base_time + TimeDelta::seconds(20),
        endpoint,
        180,
        330,
        base_time + TimeDelta::seconds(20),
    ));
    let _ = harness.run_iteration().await;

    let third_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query session stats")
        .expect("expected session stats");
    assert_eq!(third_stats.upload_diff, 30);
    assert_eq!(third_stats.download_diff, 70);
}

#[sqlx::test]
async fn test_session_manager_calculates_stats_per_gateway(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let gateway_one = create_gateway_named(&pool, location.id, user.fullname(), "gateway-1").await;
    let gateway_two = create_gateway_named(&pool, location.id, user.fullname(), "gateway-2").await;
    let mut harness = SessionManagerHarness::new(pool.clone());

    let endpoint: SocketAddr = "203.0.113.10:51820".parse().unwrap();
    let base_time = Utc::now().naive_utc();
    harness.send_stats(build_stats_update(
        location.id,
        gateway_one.id,
        &device.wireguard_pubkey,
        base_time,
        endpoint,
        100,
        200,
        base_time,
    ));
    let _ = harness.run_iteration().await;

    harness.send_stats(build_stats_update(
        location.id,
        gateway_one.id,
        &device.wireguard_pubkey,
        base_time + TimeDelta::seconds(10),
        endpoint,
        130,
        240,
        base_time + TimeDelta::seconds(10),
    ));
    let _ = harness.run_iteration().await;

    harness.send_stats(build_stats_update(
        location.id,
        gateway_two.id,
        &device.wireguard_pubkey,
        base_time + TimeDelta::seconds(20),
        endpoint,
        500,
        700,
        base_time + TimeDelta::seconds(20),
    ));
    let _ = harness.run_iteration().await;

    harness.send_stats(build_stats_update(
        location.id,
        gateway_two.id,
        &device.wireguard_pubkey,
        base_time + TimeDelta::seconds(30),
        endpoint,
        560,
        780,
        base_time + TimeDelta::seconds(30),
    ));
    let _ = harness.run_iteration().await;

    let session = VpnClientSession::try_get_active_session(&pool, location.id, device.id)
        .await
        .expect("failed to query active session")
        .expect("expected active session");
    let gateway_stats = session
        .get_latest_stats_for_all_gateways(&pool)
        .await
        .expect("failed to query gateway stats");
    assert_eq!(gateway_stats.len(), 2);

    let stats_for_gateway_one = gateway_stats
        .iter()
        .find(|stats| stats.gateway_id == gateway_one.id)
        .expect("expected gateway one stats");
    assert_eq!(stats_for_gateway_one.total_upload, 130);
    assert_eq!(stats_for_gateway_one.total_download, 240);
    assert_eq!(stats_for_gateway_one.upload_diff, 30);
    assert_eq!(stats_for_gateway_one.download_diff, 40);

    let stats_for_gateway_two = gateway_stats
        .iter()
        .find(|stats| stats.gateway_id == gateway_two.id)
        .expect("expected gateway two stats");
    assert_eq!(stats_for_gateway_two.total_upload, 560);
    assert_eq!(stats_for_gateway_two.total_download, 780);
    assert_eq!(stats_for_gateway_two.upload_diff, 60);
    assert_eq!(stats_for_gateway_two.download_diff, 80);

    assert_eq!(count_session_stats(&pool, session.id).await, 4);
}

#[sqlx::test]
async fn test_out_of_order_updates_for_existing_db_session_are_discarded(
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
    let first_handshake = Utc::now().naive_utc() - TimeDelta::seconds(5);
    let existing_session = create_session(
        &pool,
        location.id,
        user.id,
        device.id,
        Some(first_handshake),
        None,
    )
    .await;
    create_session_stats(
        &pool,
        existing_session.id,
        gateway.id,
        first_handshake,
        first_handshake,
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
        first_handshake - TimeDelta::seconds(1),
        endpoint,
        110,
        210,
        first_handshake,
    ));
    let _ = harness.run_iteration().await;

    assert_eq!(count_session_stats(&pool, existing_session.id).await, 1);
    let latest_stats = VpnSessionStats::fetch_latest_for_device(&pool, device.id, location.id)
        .await
        .expect("failed to query latest stats")
        .expect("expected latest stats");
    assert_eq!(latest_stats.session_id, existing_session.id);
    assert_eq!(latest_stats.total_upload, 100);
    assert_eq!(latest_stats.total_download, 200);
}
