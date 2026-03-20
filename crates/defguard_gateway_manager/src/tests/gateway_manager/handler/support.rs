use std::net::IpAddr;

use defguard_common::db::{
    Id,
    models::{
        device::{Device, DeviceInfo, DeviceNetworkInfo, DeviceType, WireguardNetworkDevice},
        user::User,
        vpn_client_session::VpnClientSession,
        wireguard::{LocationMfaMode, WireguardNetwork},
    },
};
use defguard_core::grpc::GatewayEvent;
use defguard_proto::enterprise::firewall::{
    FirewallConfig, FirewallPolicy, FirewallRule, IpAddress, IpVersion, Port, Protocol,
    SnatBinding, ip_address::Address, port::Port as PortInner,
};
use defguard_proto::gateway::{
    CoreResponse, Update, UpdateType, core_response,
    update::{self},
};
use sqlx::postgres::PgConnectOptions;

use crate::tests::common::HandlerTestContext;

macro_rules! assert_send_ok {
    ($result:expr, $message:literal) => {
        match $result {
            Ok(value) => value,
            Err(_) => panic!($message),
        }
    };
}

pub(crate) use assert_send_ok;

pub(crate) fn panic_unexpected(message: &str) -> ! {
    panic!("{message}")
}

pub(crate) async fn create_device_info_for_current_network(
    context: &HandlerTestContext,
    device_name: &str,
    device_pubkey: &str,
    device_ip: &str,
) -> DeviceInfo {
    create_device_info_for_network(
        context,
        context.network.id,
        device_name,
        device_pubkey,
        device_ip,
    )
    .await
}

pub(crate) async fn create_authorized_mfa_device_for_current_network(
    context: &HandlerTestContext,
    device_name: &str,
    device_pubkey: &str,
    device_ip: &str,
    preshared_key: Option<&str>,
) -> (Device<Id>, DeviceNetworkInfo) {
    create_authorized_mfa_device_for_network(
        context,
        context.network.id,
        device_name,
        device_pubkey,
        device_ip,
        preshared_key,
    )
    .await
}

pub(crate) async fn create_authorized_mfa_device_for_network(
    context: &HandlerTestContext,
    network_id: Id,
    device_name: &str,
    device_pubkey: &str,
    device_ip: &str,
    preshared_key: Option<&str>,
) -> (Device<Id>, DeviceNetworkInfo) {
    let Some(preshared_key) = preshared_key else {
        panic!("authorized MFA test device requires a preshared key")
    };

    let device =
        create_device_for_network(context, network_id, device_name, device_pubkey, device_ip).await;
    let network_device = WireguardNetworkDevice::find(&context.pool, device.id, network_id)
        .await
        .expect("failed to load MFA device network info")
        .expect("expected MFA device network info");

    let network = WireguardNetwork::find_by_id(&context.pool, network_id)
        .await
        .expect("failed to load MFA test network")
        .expect("expected MFA test network");

    let mut session = VpnClientSession::new(network_id, device.user_id, device.id, None, None);
    session.preshared_key = Some(preshared_key.to_owned());
    session
        .save(&context.pool)
        .await
        .expect("failed to persist MFA device session");

    let device_network_info = network_device
        .to_device_network_info_runtime(&context.pool, &network)
        .await
        .expect("failed to build MFA device network info");

    assert!(device_network_info.is_authorized);
    assert_eq!(
        device_network_info.preshared_key.as_deref(),
        Some(preshared_key)
    );

    (device, device_network_info)
}

pub(crate) async fn create_device_info_for_network(
    context: &HandlerTestContext,
    network_id: Id,
    device_name: &str,
    device_pubkey: &str,
    device_ip: &str,
) -> DeviceInfo {
    let device =
        create_device_for_network(context, network_id, device_name, device_pubkey, device_ip).await;

    DeviceInfo::from_device(&context.pool, device)
        .await
        .expect("failed to load device info")
}

pub(crate) async fn create_device_for_network(
    context: &HandlerTestContext,
    network_id: Id,
    device_name: &str,
    device_pubkey: &str,
    device_ip: &str,
) -> Device<Id> {
    let username = format!("{device_name}-user");
    let email = format!("{device_name}@example.com");
    let user = User::new(
        username,
        Some("pass123"),
        "Peer".to_string(),
        "Test".to_string(),
        email,
        None,
    )
    .save(&context.pool)
    .await
    .expect("failed to create test user");
    let device = Device::new(
        device_name.to_string(),
        device_pubkey.to_string(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&context.pool)
    .await
    .expect("failed to create test device");

    WireguardNetworkDevice::new(network_id, device.id, vec![parse_test_ip(device_ip)])
        .insert(&context.pool)
        .await
        .expect("failed to attach device to network");

    device
}

pub(crate) async fn enable_internal_mfa_for_network(
    pool: &sqlx::PgPool,
    network: &mut WireguardNetwork<Id>,
) {
    network.location_mfa_mode = LocationMfaMode::Internal;
    network
        .save(pool)
        .await
        .expect("failed to enable MFA for test network");
    assert!(network.mfa_enabled());
}

pub(crate) async fn assert_device_event_is_ignored_before_config_handshake(
    options: PgConnectOptions,
    device_name: &str,
    device_pubkey: &str,
    device_ip: &str,
    build_event: fn(DeviceInfo) -> GatewayEvent,
) {
    let mut context = HandlerTestContext::new(options).await;
    assert_eq!(context.events_tx().receiver_count(), 0);

    let _broadcast_guard = context.events_tx().subscribe();
    let device_info =
        create_device_info_for_current_network(&context, device_name, device_pubkey, device_ip)
            .await;

    assert_send_ok!(
        context.events_tx().send(build_event(device_info)),
        "failed to broadcast ignored device event"
    );

    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

pub(crate) async fn assert_device_event_for_different_network_is_ignored(
    options: PgConnectOptions,
    device_name: &str,
    device_pubkey: &str,
    device_ip: &str,
    build_event: fn(DeviceInfo) -> GatewayEvent,
) {
    let mut context = HandlerTestContext::new(options).await;
    let other_network = context.create_other_network().await;
    assert_ne!(other_network.id, context.network.id);

    let _ = context.complete_config_handshake().await;
    let device_info = create_device_info_for_network(
        &context,
        other_network.id,
        device_name,
        device_pubkey,
        device_ip,
    )
    .await;

    assert_send_ok!(
        context.events_tx().send(build_event(device_info)),
        "failed to broadcast ignored device event"
    );

    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

pub(crate) async fn assert_firewall_event_for_different_network_is_ignored(
    options: PgConnectOptions,
    build_event: impl FnOnce(Id) -> GatewayEvent,
) {
    let mut context = HandlerTestContext::new(options).await;
    let other_network = context.create_other_network().await;
    assert_ne!(other_network.id, context.network.id);

    let _ = context.complete_config_handshake().await;

    assert_send_ok!(
        context.events_tx().send(build_event(other_network.id)),
        "failed to broadcast ignored firewall event"
    );

    context.mock_gateway_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}

pub(crate) fn expected_keepalive_interval(context: &HandlerTestContext) -> u32 {
    u32::try_from(context.network.keepalive_interval)
        .expect("expected non-negative network keepalive interval")
}

pub(crate) fn parse_test_ip(ip: &str) -> IpAddr {
    ip.parse().expect("failed to parse test peer IP address")
}

pub(crate) fn assert_peer_update(
    outbound: CoreResponse,
    expected_update_type: UpdateType,
    expected_pubkey: &str,
    expected_allowed_ips: &[&str],
    expected_preshared_key: Option<&str>,
    expected_keepalive_interval: Option<u32>,
) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::Peer(peer)),
        })) => {
            assert_eq!(update_type, expected_update_type as i32);
            assert_eq!(peer.pubkey, expected_pubkey);
            assert_eq!(
                peer.allowed_ips,
                expected_allowed_ips
                    .iter()
                    .map(|allowed_ip| allowed_ip.to_string())
                    .collect::<Vec<_>>()
            );
            assert_eq!(peer.preshared_key.as_deref(), expected_preshared_key);
            assert_eq!(peer.keepalive_interval, expected_keepalive_interval);
        }
        _ => panic_unexpected("expected peer update"),
    }
}

pub(crate) fn assert_network_delete_update(outbound: CoreResponse, expected_network_name: &str) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::Network(network)),
        })) => {
            assert_eq!(update_type, UpdateType::Delete as i32);
            assert_eq!(network.name, expected_network_name);
        }
        _ => panic_unexpected("expected network delete update"),
    }
}

pub(crate) fn assert_network_create_update(
    outbound: CoreResponse,
    expected_network_name: &str,
    expected_address: &str,
    expected_port: u32,
    expected_mtu: u32,
    expected_fwmark: u32,
) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::Network(network)),
        })) => {
            assert_eq!(update_type, UpdateType::Create as i32);
            assert_eq!(network.name, expected_network_name);
            assert_eq!(network.addresses, vec![expected_address.to_string()]);
            assert_eq!(network.port, expected_port);
            assert_eq!(network.peers, Vec::new());
            assert_eq!(network.firewall_config, None);
            assert_eq!(network.mtu, expected_mtu);
            assert_eq!(network.fwmark, expected_fwmark);
        }
        _ => panic_unexpected("expected network create update"),
    }
}

pub(crate) fn assert_network_modify_update(
    outbound: CoreResponse,
    expected_network_name: &str,
    expected_address: &str,
    expected_port: u32,
    expected_mtu: u32,
    expected_fwmark: u32,
) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::Network(network)),
        })) => {
            assert_eq!(update_type, UpdateType::Modify as i32);
            assert_eq!(network.name, expected_network_name);
            assert_eq!(network.addresses, vec![expected_address.to_string()]);
            assert_eq!(network.port, expected_port);
            assert_eq!(network.peers, Vec::new());
            assert_eq!(network.firewall_config, None);
            assert_eq!(network.mtu, expected_mtu);
            assert_eq!(network.fwmark, expected_fwmark);
        }
        _ => panic_unexpected("expected network modify update"),
    }
}

pub(crate) fn build_test_firewall_config() -> FirewallConfig {
    FirewallConfig {
        default_policy: i32::from(FirewallPolicy::Allow),
        rules: vec![FirewallRule {
            id: 101,
            source_addrs: vec![IpAddress {
                address: Some(Address::IpSubnet("10.10.0.0/24".to_string())),
            }],
            destination_addrs: vec![IpAddress {
                address: Some(Address::Ip("198.51.100.20".to_string())),
            }],
            destination_ports: vec![Port {
                port: Some(PortInner::SinglePort(443)),
            }],
            protocols: vec![i32::from(Protocol::Tcp)],
            verdict: i32::from(FirewallPolicy::Deny),
            comment: Some("block test https destination".to_string()),
            ip_version: i32::from(IpVersion::Ipv4),
        }],
        snat_bindings: vec![SnatBinding {
            id: 202,
            source_addrs: vec![IpAddress {
                address: Some(Address::IpSubnet("10.10.0.0/24".to_string())),
            }],
            public_ip: "203.0.113.44".to_string(),
            comment: Some("test snat binding".to_string()),
        }],
    }
}

pub(crate) fn assert_firewall_modify_update(
    outbound: CoreResponse,
    expected_firewall_config: &FirewallConfig,
) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::FirewallConfig(firewall_config)),
        })) => {
            assert_eq!(update_type, UpdateType::Modify as i32);
            assert_eq!(
                firewall_config.default_policy,
                expected_firewall_config.default_policy
            );
            assert_eq!(
                firewall_config.rules.len(),
                expected_firewall_config.rules.len()
            );
            assert_eq!(
                firewall_config.snat_bindings.len(),
                expected_firewall_config.snat_bindings.len()
            );

            let firewall_rule = firewall_config
                .rules
                .first()
                .expect("expected firewall rule in update payload");
            let expected_firewall_rule = expected_firewall_config
                .rules
                .first()
                .expect("expected firewall rule in test config");
            assert_eq!(firewall_rule.id, expected_firewall_rule.id);
            assert_eq!(
                firewall_rule.source_addrs,
                expected_firewall_rule.source_addrs
            );
            assert_eq!(
                firewall_rule.destination_addrs,
                expected_firewall_rule.destination_addrs
            );
            assert_eq!(
                firewall_rule.destination_ports,
                expected_firewall_rule.destination_ports
            );
            assert_eq!(firewall_rule.protocols, expected_firewall_rule.protocols);
            assert_eq!(firewall_rule.verdict, expected_firewall_rule.verdict);
            assert_eq!(firewall_rule.comment, expected_firewall_rule.comment);
            assert_eq!(firewall_rule.ip_version, expected_firewall_rule.ip_version);

            let snat_binding = firewall_config
                .snat_bindings
                .first()
                .expect("expected SNAT binding in update payload");
            let expected_snat_binding = expected_firewall_config
                .snat_bindings
                .first()
                .expect("expected SNAT binding in test config");
            assert_eq!(snat_binding.id, expected_snat_binding.id);
            assert_eq!(
                snat_binding.source_addrs,
                expected_snat_binding.source_addrs
            );
            assert_eq!(snat_binding.public_ip, expected_snat_binding.public_ip);
            assert_eq!(snat_binding.comment, expected_snat_binding.comment);
        }
        _ => panic_unexpected("expected firewall config update"),
    }
}

pub(crate) fn assert_firewall_disable_update(outbound: CoreResponse) {
    match outbound.payload {
        Some(core_response::Payload::Update(Update {
            update_type,
            update: Some(update::Update::DisableFirewall(())),
        })) => {
            assert_eq!(update_type, UpdateType::Delete as i32);
        }
        _ => panic_unexpected("expected firewall disable update"),
    }
}
