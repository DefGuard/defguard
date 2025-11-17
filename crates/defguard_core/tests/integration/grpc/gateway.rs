use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use chrono::{Days, Utc};
use claims::{assert_err_eq, assert_matches};
use defguard_common::db::{Id, NoId, models::wireguard_peer_stats::WireguardPeerStats, setup_pool};
use defguard_core::{
    db::{
        Device, User, WireguardNetwork,
        models::{
            device::DeviceType,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
    enterprise::{license::set_cached_license, limits::update_counts},
    events::GrpcEvent,
    grpc::MIN_GATEWAY_VERSION,
};
use defguard_proto::{
    enterprise::firewall::FirewallPolicy,
    gateway::{Configuration, PeerStats, StatsUpdate, Update, stats_update::Payload, update},
};
use semver::Version;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::{sync::mpsc::error::TryRecvError, time::sleep};
use tonic::Code;

use crate::grpc::common::{TestGrpcServer, make_grpc_test_server, mock_gateway::MockGateway};

async fn setup_test_server(
    pool: PgPool,
) -> (TestGrpcServer, MockGateway, WireguardNetwork<Id>, User<Id>) {
    let test_server = make_grpc_test_server(&pool).await;

    // create a test location
    let location = WireguardNetwork::new(
        "test location".to_string(),
        Vec::new(),
        1000,
        "endpoint1".to_string(),
        None,
        Vec::new(),
        100,
        100,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .save(&pool)
    .await
    .unwrap();

    // set auth token for gateway
    let token = location
        .generate_gateway_token()
        .expect("failed to generate gateway token");

    // setup mock gateway
    let gateway = MockGateway::new(
        test_server.client_channel.clone(),
        MIN_GATEWAY_VERSION,
        Some(token),
        Some("test gateway".into()),
    )
    .await;

    // get test user
    let test_user = User::find_by_username(&pool, "hpotter")
        .await
        .unwrap()
        .unwrap();

    (test_server, gateway, location, test_user)
}

#[sqlx::test]
async fn test_gateway_authorization(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (test_server, _gateway, test_location, _test_user) = setup_test_server(pool).await;

    // setup another test gateway without a token
    let mut test_gateway = MockGateway::new(
        test_server.client_channel.clone(),
        MIN_GATEWAY_VERSION,
        None,
        Some("test gateway".into()),
    )
    .await;

    // make a request without auth token
    let response = test_gateway.get_gateway_config().await;

    // check that response code is `Code::Unauthenticated`
    assert!(response.is_err());
    let status = response.err().unwrap();
    assert_eq!(status.code(), Code::Unauthenticated);

    // setup another test gateway with an invalid token
    let mut test_gateway = MockGateway::new(
        test_server.client_channel.clone(),
        MIN_GATEWAY_VERSION,
        Some("invalid_token".into()),
        Some("test gateway".into()),
    )
    .await;
    let response = test_gateway.get_gateway_config().await;
    assert!(response.is_err());
    let status = response.err().unwrap();
    assert_eq!(status.code(), Code::Unauthenticated);

    // use valid token and retry
    let token = test_location.generate_gateway_token().unwrap();
    // setup another test gateway without a token
    let mut test_gateway = MockGateway::new(
        test_server.client_channel.clone(),
        MIN_GATEWAY_VERSION,
        Some(token),
        Some("test gateway".into()),
    )
    .await;
    let response = test_gateway.get_gateway_config().await;
    assert!(response.is_ok());
}

#[sqlx::test]
async fn test_gateway_hostname_is_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (test_server, _gateway, test_location, _test_user) = setup_test_server(pool).await;

    // setup gateway without hostname
    let token = test_location.generate_gateway_token().unwrap();
    let mut test_gateway = MockGateway::new(
        test_server.client_channel.clone(),
        MIN_GATEWAY_VERSION,
        Some(token.clone()),
        None,
    )
    .await;

    // make a request without hostname
    let response = test_gateway.get_gateway_config().await;

    // check that response code is `Code::Internal`
    assert!(response.is_err());
    let status = response.err().unwrap();
    assert_eq!(status.code(), Code::Internal);

    // set hostname and retry
    let mut test_gateway = MockGateway::new(
        test_server.client_channel.clone(),
        MIN_GATEWAY_VERSION,
        Some(token),
        Some("test gateway".into()),
    )
    .await;
    let response = test_gateway.get_gateway_config().await;
    assert!(response.is_ok());
}

#[sqlx::test]
async fn test_gateway_status(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (test_server, mut gateway, test_location, _test_user) = setup_test_server(pool).await;

    // initial gateway map is empty
    {
        let gateway_map = test_server.get_gateway_map();
        assert!(gateway_map.is_empty())
    }

    // gateway request initial config
    // it should be added to status map as disconnected
    let response = gateway.get_gateway_config().await;
    assert!(response.is_ok());
    {
        let gateway_map = test_server.get_gateway_map();
        let location_gateways = gateway_map.get_network_gateway_status(test_location.id);
        assert_eq!(location_gateways.len(), 1);
        let gateway_state = location_gateways.first().unwrap();
        assert!(!gateway_state.connected);
        assert!(gateway_state.connected_at.is_none());
        assert!(gateway_state.disconnected_at.is_none());
        assert_eq!(gateway_state.hostname, gateway.hostname());
    }

    // gateway connects to updates stream
    // it should be marked as connected
    gateway.connect_to_updates_stream().await;
    {
        let gateway_map = test_server.get_gateway_map();
        let location_gateways = gateway_map.get_network_gateway_status(test_location.id);
        assert_eq!(location_gateways.len(), 1);
        let gateway_state = location_gateways.first().unwrap();
        assert!(gateway_state.connected);
        assert!(gateway_state.connected_at.is_some());
        assert!(gateway_state.disconnected_at.is_none());
        assert_eq!(gateway_state.hostname, gateway.hostname());
    }

    // gateway disconnect from updates stream
    // it should be marked as disconnected
    gateway.disconnect_from_updates_stream();
    // wait for the background thread to handle the disconnect
    sleep(Duration::from_millis(100)).await;

    {
        let gateway_map = test_server.get_gateway_map();
        let location_gateways = gateway_map.get_network_gateway_status(test_location.id);
        assert_eq!(location_gateways.len(), 1);
        let gateway_state = location_gateways.first().unwrap();
        assert!(!gateway_state.connected);
        assert!(gateway_state.connected_at.is_some());
        assert!(gateway_state.disconnected_at.is_some());
        assert_eq!(gateway_state.hostname, gateway.hostname());
    }
}

#[sqlx::test]
async fn test_vpn_client_connected(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut test_server, mut gateway, test_location, test_user) =
        setup_test_server(pool.clone()).await;

    // initial client map is empty
    {
        let client_map = test_server.get_client_map();
        assert!(client_map.is_empty())
    }

    // connect stats stream
    let stats_tx = gateway.setup_stats_update_stream().await;
    let mut update_id = 1;

    // add user device
    let device_pubkey = "wYOt6ImBaQ3BEMQ3Xf5P5fTnbqwOvjcqYkkSBt+1xOg=";
    let test_device = Device::new(
        "test device".into(),
        device_pubkey.into(),
        test_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    // send stats update for existing device with old handshake
    // and verify no gRPC event is emitted
    stats_tx
        .send(StatsUpdate {
            id: update_id,
            payload: Some(Payload::PeerStats(PeerStats {
                public_key: device_pubkey.into(),
                endpoint: "1.2.3.4:1234".into(),
                latest_handshake: 0,
                ..Default::default()
            })),
        })
        .expect("failed to send stats update");

    assert_err_eq!(test_server.grpc_event_rx.try_recv(), TryRecvError::Empty);

    // send stats update with current handshake
    update_id += 1;
    stats_tx
        .send(StatsUpdate {
            id: update_id,
            payload: Some(Payload::PeerStats(PeerStats {
                public_key: device_pubkey.into(),
                endpoint: "1.2.3.4:1234".into(),
                latest_handshake: Utc::now().timestamp() as u64,
                ..Default::default()
            })),
        })
        .expect("failed to send stats update");

    // wait for event to be emitted
    sleep(Duration::from_millis(100)).await;
    let grpc_event = test_server
        .grpc_event_rx
        .try_recv()
        .expect("failed to receive gRPC event");

    assert_matches!(
        grpc_event,
        GrpcEvent::ClientConnected {
            context: _,
            location,
            device
        } if ((location.id == test_location.id) & (device.id == test_device.id))
    );
}

#[sqlx::test]
async fn test_vpn_client_disconnected(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut test_server, mut gateway, test_location, test_user) =
        setup_test_server(pool.clone()).await;

    // add user device
    let device_pubkey = "wYOt6ImBaQ3BEMQ3Xf5P5fTnbqwOvjcqYkkSBt+1xOg=";
    let test_device = Device::new(
        "test device".into(),
        device_pubkey.into(),
        test_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    // insert device into client map with an old handshake
    {
        let mut client_map = test_server.get_client_map();
        let now = Utc::now().naive_utc();
        let stats = WireguardPeerStats {
            id: NoId,
            device_id: test_device.id,
            collected_at: now,
            network: test_location.id,
            endpoint: None,
            upload: 0,
            download: 0,
            latest_handshake: now.checked_sub_days(Days::new(1)).unwrap(),
            allowed_ips: None,
        };
        client_map
            .connect_vpn_client(
                test_location.id,
                &gateway.hostname(),
                device_pubkey,
                &test_device,
                &test_user,
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
                &stats,
            )
            .expect("failed to insert connected client");
    }

    // connect stats stream
    let stats_tx = gateway.setup_stats_update_stream().await;
    let mut update_id = 1;

    // send stats update with old handshake
    update_id += 1;
    stats_tx
        .send(StatsUpdate {
            id: update_id,
            payload: Some(Payload::PeerStats(PeerStats {
                public_key: device_pubkey.into(),
                endpoint: "1.2.3.4:1234".into(),
                latest_handshake: 0,
                ..Default::default()
            })),
        })
        .expect("failed to send stats update");

    // wait for event to be emitted
    sleep(Duration::from_millis(100)).await;
    let grpc_event = test_server
        .grpc_event_rx
        .try_recv()
        .expect("failed to receive gRPC event");

    assert_matches!(
        grpc_event,
        GrpcEvent::ClientDisconnected {
            context: _,
            location,
            device
        } if ((location.id == test_location.id) & (device.id == test_device.id))
    );
}

#[sqlx::test]
async fn test_gateway_update_routing(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (test_server, mut gateway_1, test_location, _test_user) =
        setup_test_server(pool.clone()).await;

    // setup another test location & gateway
    let test_location_2 = WireguardNetwork::new(
        "test location 2".to_string(),
        Vec::new(),
        1000,
        "endpoint2".to_string(),
        None,
        Vec::new(),
        100,
        100,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .save(&pool)
    .await
    .unwrap();

    // set auth token for gateway
    let token = test_location_2
        .generate_gateway_token()
        .expect("failed to generate gateway token");
    let mut gateway_2 = MockGateway::new(
        test_server.client_channel.clone(),
        MIN_GATEWAY_VERSION,
        Some(token),
        Some("test_gateway_2".into()),
    )
    .await;

    // register gateways with core
    let _config_1 = gateway_1.get_gateway_config().await;
    let _config_2 = gateway_2.get_gateway_config().await;

    // connect gateways to the updates stream
    gateway_1.connect_to_updates_stream().await;
    gateway_2.connect_to_updates_stream().await;

    // send update for location 1
    test_server.send_wireguard_event(defguard_core::db::GatewayEvent::NetworkDeleted(
        test_location.id,
        "network name".into(),
    ));

    // only one gateway should receive this update
    assert!(gateway_2.receive_next_update().await.is_none());
    let update = gateway_1.receive_next_update().await.unwrap();
    let expected_update = Update {
        update_type: 2,
        update: Some(update::Update::Network(Configuration {
            name: "network name".into(),
            prvkey: String::new(),
            addresses: Vec::new(),
            port: 0,
            peers: Vec::new(),
            firewall_config: None,
        })),
    };
    assert_eq!(update, expected_update);

    // send update for location 2
    test_server.send_wireguard_event(defguard_core::db::GatewayEvent::NetworkDeleted(
        test_location_2.id,
        "network name 2".into(),
    ));

    // only one gateway should receive this update
    assert!(gateway_1.receive_next_update().await.is_none());
    let update = gateway_2.receive_next_update().await.unwrap();
    let expected_update = Update {
        update_type: 2,
        update: Some(update::Update::Network(Configuration {
            name: "network name 2".into(),
            prvkey: String::new(),
            addresses: Vec::new(),
            port: 0,
            peers: Vec::new(),
            firewall_config: None,
        })),
    };
    assert_eq!(update, expected_update);

    // send update for location which does not exist
    test_server.send_wireguard_event(defguard_core::db::GatewayEvent::NetworkDeleted(
        1234,
        "does not exist".into(),
    ));

    // no gateway should receive this update
    assert!(gateway_1.receive_next_update().await.is_none());
    assert!(gateway_2.receive_next_update().await.is_none());
}

#[sqlx::test]
async fn test_gateway_config(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (_test_server, mut gateway, mut test_location, _test_user) =
        setup_test_server(pool.clone()).await;

    // get gateway config
    let config = gateway.get_gateway_config().await.unwrap().into_inner();

    assert_eq!(config.name, test_location.name);
    assert!(config.firewall_config.is_none());

    // enable ACL for test location
    test_location.acl_enabled = true;
    test_location
        .save(&pool)
        .await
        .expect("failed to update location");

    // get gateway config
    let config = gateway.get_gateway_config().await.unwrap().into_inner();
    assert!(config.firewall_config.is_some());
    assert_eq!(
        config.firewall_config.unwrap().default_policy == i32::from(FirewallPolicy::Allow),
        test_location.acl_default_allow
    );

    // unset the license and create another location to exceed limits and disable enterprise features
    set_cached_license(None);
    let _test_location_2 = WireguardNetwork::new(
        "test location 2".to_string(),
        Vec::new(),
        1000,
        "endpoint2".to_string(),
        None,
        Vec::new(),
        100,
        100,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .save(&pool)
    .await
    .unwrap();
    update_counts(&pool).await.unwrap();

    let config = gateway.get_gateway_config().await.unwrap().into_inner();
    assert!(config.firewall_config.is_none());
}

#[sqlx::test]
async fn test_gateway_version_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (test_server, _gateway, test_location, _test_user) = setup_test_server(pool.clone()).await;

    // setup gateway with unsupported version
    let unsupported_version =
        Version::new(MIN_GATEWAY_VERSION.major, MIN_GATEWAY_VERSION.minor - 1, 0);
    let token = test_location.generate_gateway_token().unwrap();
    // setup another test gateway without a token
    let mut test_gateway = MockGateway::new(
        test_server.client_channel.clone(),
        unsupported_version,
        Some(token),
        Some("test gateway".into()),
    )
    .await;
    let response = test_gateway.get_gateway_config().await;

    // check that response code is `Code::FailedPrecondition`
    assert!(response.is_err());
    let status = response.err().unwrap();
    assert_eq!(status.code(), Code::FailedPrecondition);
}

// https://github.com/DefGuard/defguard/issues/1671
#[sqlx::test]
async fn test_device_pubkey_change(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut test_server, mut gateway, test_location, test_user) =
        setup_test_server(pool.clone()).await;

    // initial client map is empty
    {
        let client_map = test_server.get_client_map();
        assert!(client_map.is_empty())
    }

    // connect stats stream
    let stats_tx = gateway.setup_stats_update_stream().await;
    let mut update_id = 1;

    // add user device
    let device_pubkey = "wYOt6ImBaQ3BEMQ3Xf5P5fTnbqwOvjcqYkkSBt+1xOg=";
    let mut test_device = Device::new(
        "test device".into(),
        device_pubkey.into(),
        test_user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    // send stats update for existing device
    stats_tx
        .send(StatsUpdate {
            id: update_id,
            payload: Some(Payload::PeerStats(PeerStats {
                public_key: device_pubkey.into(),
                endpoint: "1.2.3.4:1234".into(),
                latest_handshake: Utc::now().timestamp() as u64,
                ..Default::default()
            })),
        })
        .expect("failed to send stats update");

    // wait for event to be emitted
    sleep(Duration::from_millis(100)).await;
    let grpc_event = test_server
        .grpc_event_rx
        .try_recv()
        .expect("failed to receive gRPC event");
    assert_matches!(
    grpc_event,
    GrpcEvent::ClientConnected {
        context: _,
        location,
        device
    } if ((location.id == test_location.id) & (device.id == test_device.id))
    );

    // change device pubkey
    let new_device_pubkey = "TJG2T6rhndZtk06KnIIOlD6hhd7wpVkBss8sfyvMCAA=";
    test_device.wireguard_pubkey = new_device_pubkey.to_owned();
    test_device.save(&pool).await.unwrap();

    // send stats update with old pubkey
    update_id += 1;
    stats_tx
        .send(StatsUpdate {
            id: update_id,
            payload: Some(Payload::PeerStats(PeerStats {
                public_key: device_pubkey.into(),
                endpoint: "1.2.3.4:1234".into(),
                latest_handshake: Utc::now().timestamp() as u64,
                ..Default::default()
            })),
        })
        .expect("failed to send stats update");

    // no event should be emitted
    sleep(Duration::from_millis(100)).await;
    assert_err_eq!(test_server.grpc_event_rx.try_recv(), TryRecvError::Empty);

    // send stats update with new pubkey
    update_id += 1;
    stats_tx
        .send(StatsUpdate {
            id: update_id,
            payload: Some(Payload::PeerStats(PeerStats {
                public_key: new_device_pubkey.into(),
                endpoint: "1.2.3.4:1234".into(),
                latest_handshake: Utc::now().timestamp() as u64,
                ..Default::default()
            })),
        })
        .expect("failed to send stats update");

    // wait for event
    // FIXME: ideally this should not be emitted; we'll fix it once we implement a more robust VPN session logic
    sleep(Duration::from_millis(100)).await;
    let grpc_event = test_server
        .grpc_event_rx
        .try_recv()
        .expect("failed to receive gRPC event");

    assert_matches!(
        grpc_event,
        GrpcEvent::ClientConnected {
            context: _,
            location,
            device
        } if ((location.id == test_location.id) & (device.id == test_device.id))
    );
}
