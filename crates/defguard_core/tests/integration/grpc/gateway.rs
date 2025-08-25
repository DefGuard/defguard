use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use chrono::{Days, Utc};
use claims::{assert_err_eq, assert_matches};
use defguard_core::{
    db::{
        Device, Id, NoId, User, WireguardNetwork,
        models::{
            device::DeviceType, wireguard::LocationMfaMode,
            wireguard_peer_stats::WireguardPeerStats,
        },
        setup_pool,
    },
    events::GrpcEvent,
    grpc::{
        gateway::{Configuration, Update, update},
        proto::gateway::{PeerStats, StatsUpdate, stats_update::Payload},
    },
};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::{
    sync::mpsc::error::TryRecvError,
    time::{advance, pause, sleep},
};
use tokio_stream::StreamExt;
use tonic::Code;

use crate::grpc::common::{
    TestGrpcServer, create_client_channel, make_grpc_test_server, mock_gateway::MockGateway,
};

async fn setup_test_server(
    pool: PgPool,
) -> (TestGrpcServer, MockGateway, WireguardNetwork<Id>, User<Id>) {
    let test_server = make_grpc_test_server(&pool).await;

    // setup mock gateway
    let mut gateway = MockGateway::new(test_server.client_channel.clone()).await;

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
    )
    .save(&pool)
    .await
    .unwrap();

    // set auth token for gateway
    let token = location
        .generate_gateway_token()
        .expect("failed to generate gateway token");
    gateway.set_token(&token);

    // set hostname for gateway
    gateway.set_hostname("test_gateway");

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
    let (_test_server, mut gateway, test_location, _test_user) = setup_test_server(pool).await;

    // remove auth token
    gateway.clear_token();

    // make a request without auth token
    let response = gateway.get_gateway_config().await;

    // check that response code is Status::Unauthenticated
    assert!(response.is_err());
    let status = response.err().unwrap();
    assert_eq!(status.code(), Code::Unauthenticated);

    // set invalid token and check again
    gateway.set_token("invalid_token");
    let response = gateway.get_gateway_config().await;
    assert!(response.is_err());
    let status = response.err().unwrap();
    assert_eq!(status.code(), Code::Unauthenticated);

    // set valid token and retry
    let token = test_location.generate_gateway_token().unwrap();
    gateway.set_token(&token);
    let response = gateway.get_gateway_config().await;
    assert!(response.is_ok());
}

#[sqlx::test]
async fn test_gateway_hostname_is_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (_test_server, mut gateway, _test_location, _test_user) = setup_test_server(pool).await;

    // remove hostname
    gateway.clear_hostname();

    // make a request without hostname
    let response = gateway.get_gateway_config().await;

    // check that response code is Status::Unauthenticated
    assert!(response.is_err());
    let status = response.err().unwrap();
    assert_eq!(status.code(), Code::Internal);

    // set hostname and retry
    gateway.set_hostname("hostname");
    let response = gateway.get_gateway_config().await;
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

// test correct config is sent to gw
// firewall rules are included

// test updates stream
// filtering by id
// sent to multiple gateways

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
    let (mut test_server, mut gateway_1, test_location, test_user) =
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
    )
    .save(&pool)
    .await
    .unwrap();
    let mut gateway_2 = MockGateway::new(test_server.client_channel.clone()).await;

    // set auth token for gateway
    let token = test_location_2
        .generate_gateway_token()
        .expect("failed to generate gateway token");
    gateway_2.set_token(&token);

    // set hostname for gateway
    gateway_2.set_hostname("test_gateway_2");

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
