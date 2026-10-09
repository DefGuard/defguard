use std::{
    collections::HashMap,
    net::IpAddr,
    str::FromStr,
    sync::{Arc, atomic::AtomicBool},
};

use chrono::{DateTime, Utc};
use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceType, User,
            device::WireguardNetworkDevice,
            gateway::Gateway,
            vpn_client_session::VpnClientSession,
            wireguard::{ServiceLocationMode, WireguardNetwork},
        },
        setup_pool,
    },
    gateway_event::GatewayCommand,
    gateway_types::WireguardPeer,
};
use defguard_proto::gateway::{Configuration, PeerStats, core_response};
use prost_types::Timestamp;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::sync::{broadcast, mpsc::unbounded_channel, watch};

use crate::updates::GatewayUpdatesHandler;

use super::{GatewayHandler, try_protos_into_stats_message};

fn test_network(mfa_enabled: bool) -> WireguardNetwork<Id> {
    WireguardNetwork::new(
        "test-network".into(),
        51820,
        "127.0.0.1".into(),
        None,
        Vec::new(),
        true,
        false,
        false,
        false,
        mfa_enabled,
        ServiceLocationMode::Disabled,
    )
    .with_id(1)
}

fn build_peer_stats(endpoint: &str) -> PeerStats {
    PeerStats {
        public_key: "peer-public-key".to_owned(),
        endpoint: endpoint.to_owned(),
        upload: 123,
        download: 456,
        keepalive_interval: 25,
        latest_handshake: Some(prost_types::Timestamp {
            seconds: 1_700_000_000,
            nanos: 0,
        }),
        allowed_ips: "10.10.0.2/32".to_owned(),
    }
}

fn build_network() -> WireguardNetwork<Id> {
    let mut network = WireguardNetwork::new(
        "test-network".to_owned(),
        51820,
        "198.51.100.10".to_owned(),
        Some("1.1.1.1".to_owned()),
        ["0.0.0.0/0".parse().expect("valid allowed IP network")],
        false,
        false,
        false,
        false,
        false, // mfa_enabled
        ServiceLocationMode::default(),
    )
    .set_address([
        "10.10.0.1/24".parse().expect("valid IPv4 network"),
        "fd00::1/64".parse().expect("valid IPv6 network"),
    ])
    .expect("valid network addresses")
    .with_id(1);
    network.pubkey = "network-public-key".to_owned();
    network.prvkey = "network-private-key".to_owned();
    network.mtu = 1420;
    network.fwmark = 4321;
    network.keepalive_interval = 25;
    network.peer_disconnect_threshold = 180;
    network
}

#[test]
fn try_protos_into_stats_message_maps_valid_peer_stats() {
    let stats = try_protos_into_stats_message(build_peer_stats("203.0.113.10:51820"), 11, 22)
        .expect("valid peer stats should be converted");

    assert_eq!(stats.location_id, 11);
    assert_eq!(stats.gateway_id, 22);
    assert_eq!(stats.device_pubkey, "peer-public-key");
    assert_eq!(stats.endpoint.to_string(), "203.0.113.10:51820");
    assert_eq!(stats.upload, 123);
    assert_eq!(stats.download, 456);
    assert_eq!(
        stats.latest_handshake,
        DateTime::from_timestamp(1_700_000_000, 0)
            .expect("valid handshake timestamp")
            .naive_utc()
    );
}

#[test]
fn try_protos_into_stats_message_rejects_invalid_endpoint() {
    let stats = try_protos_into_stats_message(build_peer_stats("not-a-socket-address"), 11, 22);

    assert!(stats.is_none());
}

#[test]
fn try_protos_into_stats_message_returns_none_for_missing_handshake() {
    let stats = try_protos_into_stats_message(
        PeerStats {
            latest_handshake: None,
            ..build_peer_stats("203.0.113.10:51820")
        },
        11,
        22,
    );

    assert!(stats.is_none());
}

#[test]
fn try_protos_into_stats_message_returns_none_for_invalid_timestamp() {
    let stats = try_protos_into_stats_message(
        PeerStats {
            latest_handshake: Some(Timestamp {
                seconds: i64::MAX,
                nanos: 0,
            }),
            ..build_peer_stats("203.0.113.10:51820")
        },
        11,
        22,
    );

    assert!(stats.is_none());
}

#[test]
fn gen_config_maps_network_fields() {
    use defguard_common::gateway_types::{FirewallConfig as NativeFirewallConfig, FirewallPolicy};
    use defguard_proto::enterprise::firewall::FirewallPolicy as ProtoFirewallPolicy;
    let config = Configuration::new(
        &build_network(),
        vec![WireguardPeer {
            pubkey: "peer-public-key".to_owned(),
            allowed_ips: vec!["10.10.0.2/32".to_owned()],
            preshared_key: Some("peer-preshared-key".to_owned()),
            keepalive_interval: Some(25),
        }],
        Some(NativeFirewallConfig {
            default_policy: FirewallPolicy::Unspecified,
            rules: Vec::new(),
            snat_bindings: Vec::new(),
        }),
    );

    assert_eq!(config.name, "test-network");
    assert_eq!(config.port, 51820);
    assert_eq!(config.private_key, "network-private-key");
    assert_eq!(config.addresses, vec!["10.10.0.1/24", "fd00::1/64"]);
    assert_eq!(config.mtu, 1420);
    assert_eq!(config.fwmark, 4321);

    let peer = config
        .peers
        .first()
        .expect("generated config should include peer");
    assert_eq!(peer.pubkey, "peer-public-key");
    assert_eq!(peer.allowed_ips, vec!["10.10.0.2/32"]);
    assert_eq!(peer.preshared_key.as_deref(), Some("peer-preshared-key"));
    assert_eq!(peer.keepalive_interval, Some(25));

    let firewall_config = config
        .firewall_config
        .expect("generated config should include firewall config");
    assert_eq!(
        firewall_config.default_policy,
        ProtoFirewallPolicy::Unspecified as i32
    );
    assert_eq!(
        firewall_config.rules,
        [] as [defguard_proto::enterprise::firewall::FirewallRule; 0]
    );
    assert_eq!(
        firewall_config.snat_bindings,
        [] as [defguard_proto::enterprise::firewall::SnatBinding; 0]
    );
}

#[test]
fn gen_config_preserves_absent_firewall_config_and_empty_peers() {
    let config = Configuration::new(&build_network(), Vec::new(), None);

    assert_eq!(config.peers, [] as [defguard_proto::gateway::Peer; 0]);
    assert!(config.firewall_config.is_none());
}

fn test_handler(mfa_enabled: bool) -> GatewayUpdatesHandler {
    let network = test_network(mfa_enabled);
    let (events_tx, events_rx) = broadcast::channel(1);
    let (tx, _rx) = unbounded_channel();
    drop(events_tx);

    let mut handler =
        GatewayUpdatesHandler::new(network.id, network, "gateway".into(), None, events_rx, tx);
    handler.session_authorization_required = handler.network.mfa_enabled;
    handler
}

#[test]
fn test_runtime_peer_update_strips_preshared_key_for_non_mfa_locations() {
    let handler = test_handler(false);

    let peer = handler
        .runtime_peer_update(
            "device",
            "device-pubkey".into(),
            vec!["10.1.1.2".into()],
            true,
            Some("legacy-psk".into()),
        )
        .unwrap();

    assert_eq!(peer.pubkey, "device-pubkey");
    assert_eq!(peer.allowed_ips, ["10.1.1.2"]);
    assert_eq!(peer.preshared_key, None);
    assert_eq!(peer.keepalive_interval, Some(25));
}

#[test]
fn test_runtime_peer_update_skips_authorized_mfa_peer_without_session_preshared_key() {
    let handler = test_handler(true);

    let peer = handler.runtime_peer_update(
        "device",
        "device-pubkey".into(),
        vec!["10.1.1.2".into()],
        true,
        None,
    );

    assert_eq!(peer, None);
}

#[test]
fn test_runtime_peer_update_preserves_session_preshared_key_for_authorized_mfa_peer() {
    let handler = test_handler(true);

    let peer = handler
        .runtime_peer_update(
            "device",
            "device-pubkey".into(),
            vec!["10.1.1.2".into()],
            true,
            Some("session-psk".into()),
        )
        .unwrap();

    assert_eq!(peer.preshared_key, Some("session-psk".into()));
}

#[test]
fn test_runtime_peer_update_preserves_session_preshared_key_for_authorized_posture_peer() {
    let mut handler = test_handler(false);
    handler.session_authorization_required = true;

    let peer = handler
        .runtime_peer_update(
            "device",
            "device-pubkey".into(),
            vec!["10.1.1.2".into()],
            true,
            Some("session-psk".into()),
        )
        .unwrap();

    assert_eq!(peer.preshared_key, Some("session-psk".into()));
}

#[sqlx::test]
async fn test_send_configuration_includes_mfa_peers_with_session_preshared_key(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password123"),
        "Test",
        "User",
        "test@example.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let new_device = Device::new(
        "device-new".into(),
        "pubkey-new".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let connected_device = Device::new(
        "device-connected".into(),
        "pubkey-connected".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.7.1.1/24")
        .unwrap();
    network.name = "mfa-full-config-location".to_owned();
    network.mfa_enabled = true;
    network.service_location_mode = ServiceLocationMode::Disabled;
    let network = network.save(&pool).await.unwrap();

    WireguardNetworkDevice::new(
        network.id,
        new_device.id,
        vec![IpAddr::from_str("10.7.1.2").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();

    WireguardNetworkDevice::new(
        network.id,
        connected_device.id,
        vec![IpAddr::from_str("10.7.1.3").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();

    let mut new_session = VpnClientSession::new(network.id, user.id, new_device.id, None, false);
    new_session.preshared_key = Some("new-session-psk".into());
    new_session.save(&pool).await.unwrap();

    let mut connected_session = VpnClientSession::new(
        network.id,
        user.id,
        connected_device.id,
        Some(Utc::now().naive_utc()),
        false,
    );
    connected_session.preshared_key = Some("connected-session-psk".into());
    connected_session.save(&pool).await.unwrap();

    let gateway = Gateway::new(network.id, "gateway", "127.0.0.1", 50051, "test")
        .save(&pool)
        .await
        .unwrap();
    let (events_tx, _events_rx) = broadcast::channel::<GatewayCommand>(1);
    let (connection_events_tx, _connection_events_rx) = unbounded_channel();
    let (peer_stats_tx, _peer_stats_rx) = unbounded_channel();
    let (_certs_tx, certs_rx) = watch::channel(Arc::new(HashMap::<Id, String>::new()));
    let handler = GatewayHandler::new(
        gateway,
        pool.clone(),
        events_tx,
        connection_events_tx,
        peer_stats_tx,
        certs_rx,
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let (tx, mut rx) = unbounded_channel();

    handler.send_configuration(&tx).await.unwrap();

    let response = rx.recv().await.unwrap();
    let Some(core_response::Payload::Config(configuration)) = response.payload else {
        panic!("expected gateway config payload");
    };

    assert_eq!(configuration.peers.len(), 2);
    assert_eq!(
        configuration
            .peers
            .iter()
            .map(|peer| (peer.pubkey.as_str(), peer.preshared_key.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("pubkey-new", Some("new-session-psk")),
            ("pubkey-connected", Some("connected-session-psk")),
        ]
    );
}
