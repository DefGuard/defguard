use std::{net::Ipv4Addr, str::FromStr};

use claims::{assert_err, assert_ok};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::{
    csv::AsCsv,
    db::{
        models::{gateway::Gateway, vpn_session_stats::VpnSessionStats},
        setup_pool,
    },
};

impl Device<Id> {
    /// Create new device and assign IP in a given network
    // TODO: merge with `assign_network_ip()`
    pub(crate) async fn new_with_ip(
        pool: &PgPool,
        user_id: Id,
        name: String,
        pubkey: String,
        network: &WireguardNetwork<Id>,
    ) -> Result<(Self, WireguardNetworkDevice), ModelError> {
        if let Some(address) = network.address().first() {
            let net_ip = address.ip();
            let net_network = address.network();
            let net_broadcast = address.broadcast();
            for ip in address {
                if ip == net_ip || ip == net_network || ip == net_broadcast {
                    continue;
                }
                // Break loop if IP is unassigned and return device
                if Self::find_by_ip(pool, ip, network.id).await?.is_none() {
                    let device =
                        Device::new(name.clone(), pubkey, user_id, DeviceType::User, None, true)
                            .save(pool)
                            .await?;
                    info!("Created device: {}", device.name);
                    debug!("For user: {}", device.user_id);
                    let wireguard_network_device =
                        WireguardNetworkDevice::new(network.id, device.id, [ip]);
                    wireguard_network_device.insert(pool).await?;
                    info!(
                        "Assigned IP: {ip} for device: {name} in network: {}",
                        network.id
                    );
                    return Ok((device, wireguard_network_device));
                }
            }
        }
        Err(ModelError::CannotCreate)
    }
}

#[sqlx::test]
async fn test_assign_device_ip(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/30")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let user = User::new(
        "testuser",
        Some("hunter2"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    let (_device, wireguard_network_device) =
        Device::new_with_ip(&pool, user.id, "dev1".into(), "key1".into(), &network)
            .await
            .unwrap();
    assert_eq!(wireguard_network_device.wireguard_ips.as_csv(), "10.1.1.2");

    let device = Device::new_with_ip(&pool, 1, "dev4".into(), "key4".into(), &network).await;
    assert!(device.is_err());
}

/// Test that assign_next_network_ip correctly preserves or reassigns device IPs
/// when a network's address list changes.
/// Initial network: 10.0.0.0/8, 123.10.0.0/16, 123.123.123.0/24
/// Device IPs:      10.0.0.234,  123.10.33.44,  123.123.123.52
/// New network:     10.0.0.0/16, 123.12.0.0/16, 123.123.0.0/16
/// Expected:
///  - 10.0.0.234     KEPT    (still within 10.0.0.0/16)
///  - 123.10.33.44   CHANGED (not within 123.12.0.0/16)
///  - 123.123.123.52 KEPT    (still within 123.123.0.0/16)
#[sqlx::test]
async fn test_assign_next_network_ip_preserves_matching_subnets(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.0.1/8,123.10.0.1/16,123.123.123.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "dev1".into(),
        "key1".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let ip = IpAddr::from_str("10.0.0.234").unwrap();
    let ip2 = IpAddr::from_str("123.10.33.44").unwrap();
    let ip3 = IpAddr::from_str("123.123.123.52").unwrap();
    let initial_ips = vec![ip, ip2, ip3];

    let mut conn = pool.acquire().await.unwrap();
    WireguardNetworkDevice::new(network.id, device.id, initial_ips.clone())
        .insert(&mut *conn)
        .await
        .unwrap();

    let updated_network = network
        .clone()
        .set_address([
            "10.0.0.1/16".parse().unwrap(),
            "123.12.0.1/16".parse().unwrap(),
            "123.123.0.1/16".parse().unwrap(),
        ])
        .unwrap();
    updated_network.save(&mut *conn).await.unwrap();

    let used_ips = updated_network
        .all_used_ip_addresses(&mut conn)
        .await
        .unwrap();

    let result = device
        .assign_next_network_ip(
            &mut conn,
            &updated_network,
            &used_ips,
            None,
            Some(&initial_ips),
        )
        .await
        .unwrap();

    let new_ips = &result.wireguard_ips;
    assert_eq!(new_ips.len(), 3, "should have one IP per subnet");

    assert!(
        new_ips.contains(&ip),
        "10.0.0.234 should be kept – it is still within 10.0.0.0/16; got {new_ips:?}"
    );

    assert!(
        !new_ips.contains(&ip2),
        "123.10.33.44 should be reassigned – not within 123.12.0.0/16; got {new_ips:?}"
    );
    let network: IpNetwork = "123.12.0.0/16".parse().unwrap();
    assert!(
        new_ips.iter().any(|ip| network.contains(*ip)),
        "a new IP within 123.12.0.0/16 should be assigned; got {new_ips:?}"
    );

    assert!(
        new_ips.contains(&ip3),
        "123.123.123.52 should be kept – it is still within 123.123.0.0/16; got {new_ips:?}"
    );
}
/// Initial:  10.0.0.0/8  | 10.1.0.5
/// Modified: 10.0.0.0/16 | 10.1.0.5 should be replaced with a 10.0.x.x address
#[sqlx::test]
async fn test_assign_next_network_ip_subnet_narrowed(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.0.1/8")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "dev1".into(),
        "key1".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let ip = IpAddr::from_str("10.1.0.5").unwrap();
    let initial_ips = vec![ip];

    let mut conn = pool.acquire().await.unwrap();
    WireguardNetworkDevice::new(network.id, device.id, initial_ips.clone())
        .insert(&mut *conn)
        .await
        .unwrap();

    let updated_network = network
        .clone()
        .set_address(["10.0.0.1/16".parse().unwrap()])
        .unwrap();
    updated_network.save(&mut *conn).await.unwrap();

    let used_ips = updated_network
        .all_used_ip_addresses(&mut conn)
        .await
        .unwrap();

    let result = device
        .assign_next_network_ip(
            &mut conn,
            &updated_network,
            &used_ips,
            None,
            Some(&initial_ips),
        )
        .await
        .unwrap();

    let new_ips = &result.wireguard_ips;
    assert_eq!(new_ips.len(), 1, "should have one IP per subnet");

    assert!(
        !new_ips.contains(&ip),
        "10.1.0.5 should be reassigned – outside narrowed 10.0.0.0/16; got {new_ips:?}"
    );
    let narrowed_net: IpNetwork = "10.0.0.0/16".parse().unwrap();
    assert!(
        new_ips.iter().all(|ip| narrowed_net.contains(*ip)),
        "new IP must be within 10.0.0.0/16; got {new_ips:?}"
    );
}

/// Initial:  123.123.123.0/24 | 123.123.123.254
/// Modified: 123.123.0.0/16   | 123.123.123.254 still fits
#[sqlx::test]
async fn test_assign_next_network_ip_still_valid_after_widening(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let network = WireguardNetwork::default()
        .try_set_address("123.123.123.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "dev1".into(),
        "key1".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let ip = IpAddr::from_str("123.123.123.254").unwrap();
    let initial_ips = vec![ip];

    let mut conn = pool.acquire().await.unwrap();
    WireguardNetworkDevice::new(network.id, device.id, initial_ips.clone())
        .insert(&mut *conn)
        .await
        .unwrap();

    let updated_network = network
        .clone()
        .set_address(["123.123.0.1/16".parse().unwrap()])
        .unwrap();
    updated_network.save(&mut *conn).await.unwrap();

    let used_ips = updated_network
        .all_used_ip_addresses(&mut conn)
        .await
        .unwrap();

    let result = device
        .assign_next_network_ip(
            &mut conn,
            &updated_network,
            &used_ips,
            None,
            Some(&initial_ips),
        )
        .await
        .unwrap();

    let new_ips = &result.wireguard_ips;
    assert_eq!(new_ips.len(), 1, "should have one IP per subnet");

    assert!(
        new_ips.contains(&ip),
        "123.123.123.254 should be preserved – still within widened 123.123.0.0/16; got {new_ips:?}"
    );
}

#[test]
fn test_pubkey_validation() {
    let invalid_test_key = "invalid_key";
    assert_err!(Device::validate_pubkey(invalid_test_key));

    let valid_test_key = "sejIy0WCLvOR7vWNchP9Elsayp3UTK/QCnEJmhsHKTc=";
    assert_ok!(Device::validate_pubkey(valid_test_key));
}

#[sqlx::test]
async fn test_runtime_mfa_state_marks_mfa_session_without_preshared_key_as_unauthorized(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let network = WireguardNetwork::new(
        "runtime-mfa-network".into(),
        51820,
        "vpn.example.com".into(),
        None,
        Vec::<IpNetwork>::new(),
        false,
        false,
        false,
        false,
        true, // mfa_enabled
        ServiceLocationMode::Disabled,
    )
    .try_set_address("10.1.1.1/24")
    .unwrap()
    .save(&pool)
    .await
    .unwrap();

    let wireguard_network_device = WireguardNetworkDevice {
        wireguard_network_id: network.id,
        wireguard_ips: vec![IpAddr::from_str("10.1.1.2").unwrap()],
        device_id: 1,
    };
    let active_session = VpnClientSession {
        id: 1,
        location_id: network.id,
        user_id: 1,
        device_id: wireguard_network_device.device_id,
        created_at: Utc::now().naive_utc(),
        connected_at: None,
        disconnected_at: None,
        is_mfa_session: true,
        state: VpnClientSessionState::New,
        preshared_key: None,
    };

    let network_info =
        wireguard_network_device.to_device_network_info(&network, Some(&active_session));

    assert_eq!(network_info.preshared_key, None);
    assert!(!network_info.is_authorized);
}

#[sqlx::test]
async fn test_runtime_mfa_state_keeps_session_preshared_key_for_authorized_runtime_reads(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let network = WireguardNetwork::new(
        "runtime-mfa-network".into(),
        51820,
        "vpn.example.com".into(),
        None,
        Vec::<IpNetwork>::new(),
        false,
        false,
        false,
        false,
        true, // mfa_enabled
        ServiceLocationMode::Disabled,
    )
    .try_set_address("10.1.1.1/24")
    .unwrap()
    .save(&pool)
    .await
    .unwrap();

    let wireguard_network_device = WireguardNetworkDevice {
        wireguard_network_id: network.id,
        wireguard_ips: vec![IpAddr::from_str("10.1.1.2").unwrap()],
        device_id: 1,
    };
    let active_session = VpnClientSession {
        id: 1,
        location_id: network.id,
        user_id: 1,
        device_id: wireguard_network_device.device_id,
        created_at: Utc::now().naive_utc(),
        connected_at: Some(Utc::now().naive_utc()),
        disconnected_at: None,
        is_mfa_session: true,
        state: VpnClientSessionState::Connected,
        preshared_key: Some("runtime-session-psk".into()),
    };

    let network_info =
        wireguard_network_device.to_device_network_info(&network, Some(&active_session));

    assert_eq!(
        network_info.preshared_key,
        Some("runtime-session-psk".into())
    );
    assert!(network_info.is_authorized);
}

#[sqlx::test]
async fn test_device_info_marks_mfa_session_without_preshared_key_as_unauthorized(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::new(
        "device-info-network".into(),
        51820,
        "vpn.example.com".into(),
        None,
        Vec::<IpNetwork>::new(),
        false,
        false,
        false,
        false,
        true, // mfa_enabled
        ServiceLocationMode::Disabled,
    )
    .try_set_address("10.1.1.1/24")
    .unwrap();
    let network = network.save(&pool).await.unwrap();

    let wireguard_network_device = WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    );
    wireguard_network_device.insert(&pool).await.unwrap();

    let session = VpnClientSession::new(network.id, user.id, device.id, None, true);
    session.save(&pool).await.unwrap();

    let device_info = DeviceInfo::from_device(&pool, device).await.unwrap();
    let network_info = device_info
        .network_info
        .into_iter()
        .find(|info| info.network_id == network.id)
        .unwrap();

    assert!(!network_info.is_authorized);
    assert_eq!(network_info.preshared_key, None);
}

#[sqlx::test]
async fn test_device_info_keeps_mfa_session_preshared_key_for_authorized_full_sync_reads(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::new(
        "device-info-network".into(),
        51820,
        "vpn.example.com".into(),
        None,
        Vec::<IpNetwork>::new(),
        false,
        false,
        false,
        false,
        true, // mfa_enabled
        ServiceLocationMode::Disabled,
    )
    .try_set_address("10.1.1.1/24")
    .unwrap()
    .save(&pool)
    .await
    .unwrap();

    let wireguard_network_device = WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    );
    wireguard_network_device.insert(&pool).await.unwrap();

    let mut session = VpnClientSession::new(
        network.id,
        user.id,
        device.id,
        Some(Utc::now().naive_utc()),
        true,
    );
    session.preshared_key = Some("device-info-session-psk".into());
    session.save(&pool).await.unwrap();

    let device_info = DeviceInfo::from_device(&pool, device).await.unwrap();
    let network_info = device_info
        .network_info
        .into_iter()
        .find(|info| info.network_id == network.id)
        .unwrap();

    assert!(network_info.is_authorized);
    assert_eq!(
        network_info.preshared_key,
        Some("device-info-session-psk".into())
    );
}

#[sqlx::test]
async fn test_device_info_keeps_non_mfa_location_authorized_without_exposing_session_preshared_key(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::new(
        "device-info-network".into(),
        51820,
        "vpn.example.com".into(),
        None,
        Vec::<IpNetwork>::new(),
        false,
        false,
        false,
        false,
        false, // mfa_enabled
        ServiceLocationMode::Disabled,
    )
    .try_set_address("10.1.1.1/24")
    .unwrap()
    .save(&pool)
    .await
    .unwrap();

    let wireguard_network_device = WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    );
    wireguard_network_device.insert(&pool).await.unwrap();

    let mut session = VpnClientSession::new(network.id, user.id, device.id, None, false);
    session.preshared_key = Some("legacy-session-psk".into());
    session.save(&pool).await.unwrap();

    let device_info = DeviceInfo::from_device(&pool, device).await.unwrap();
    let network_info = device_info
        .network_info
        .into_iter()
        .find(|info| info.network_id == network.id)
        .unwrap();

    assert!(network_info.is_authorized);
    assert_eq!(network_info.preshared_key, None);
}

#[sqlx::test]
async fn test_user_device_from_device_keeps_latest_successful_connection_timestamp(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();

    let gateway = Gateway::new(network.id, "gateway", "198.51.100.1", 51820, "tester")
        .save(&pool)
        .await
        .unwrap();

    let last_successful_connection = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 4, 5)
        .expect("expected valid time");
    let last_successful_stats_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 5, 6)
        .expect("expected valid time");
    let newer_session_created_at = NaiveDate::from_ymd_opt(2026, 1, 3)
        .expect("expected valid date")
        .and_hms_opt(4, 5, 6)
        .expect("expected valid time");
    let newer_session_stats_at = NaiveDate::from_ymd_opt(2026, 1, 3)
        .expect("expected valid date")
        .and_hms_opt(4, 6, 7)
        .expect("expected valid time");

    let mut connected_session = VpnClientSession::new(
        network.id,
        user.id,
        device.id,
        Some(last_successful_connection),
        false,
    );
    connected_session.created_at = last_successful_connection;
    let connected_session = connected_session.save(&pool).await.unwrap();

    VpnSessionStats::new(
        connected_session.id,
        gateway.id,
        last_successful_stats_at,
        last_successful_stats_at,
        "203.0.113.10:51820".into(),
        1,
        1,
        1,
        1,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut disconnected_session =
        VpnClientSession::new(network.id, user.id, device.id, None, false);
    disconnected_session.created_at = newer_session_created_at;
    disconnected_session.disconnected_at = Some(newer_session_created_at);
    disconnected_session.state = VpnClientSessionState::Disconnected;
    let disconnected_session = disconnected_session.save(&pool).await.unwrap();

    VpnSessionStats::new(
        disconnected_session.id,
        gateway.id,
        newer_session_stats_at,
        newer_session_stats_at,
        "198.51.100.99:51820".into(),
        2,
        2,
        2,
        2,
    )
    .save(&pool)
    .await
    .unwrap();

    let user_device = UserDevice::from_device(&pool, device)
        .await
        .unwrap()
        .unwrap();
    let network_info = user_device
        .networks
        .into_iter()
        .find(|network_info| network_info.network_id == network.id)
        .expect("expected created network in user device response");

    assert!(network_info.is_active);
    assert_eq!(network_info.last_connected_ip, Some("203.0.113.10".into()));
    assert_eq!(
        network_info.last_connected_at,
        Some(last_successful_connection)
    );
}

#[sqlx::test]
async fn test_user_device_from_device_keeps_latest_successful_connection_timestamp_for_newer_new_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();

    let last_successful_connection = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 4, 5)
        .expect("expected valid time");
    let newer_session_created_at = NaiveDate::from_ymd_opt(2026, 1, 3)
        .expect("expected valid date")
        .and_hms_opt(4, 5, 6)
        .expect("expected valid time");

    let disconnected_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 5, 6)
        .expect("expected valid time");

    let mut connected_session = VpnClientSession::new(
        network.id,
        user.id,
        device.id,
        Some(last_successful_connection),
        false,
    );
    connected_session.created_at = last_successful_connection;
    connected_session.disconnected_at = Some(disconnected_at);
    connected_session.state = VpnClientSessionState::Disconnected;
    connected_session.save(&pool).await.unwrap();

    let mut new_session = VpnClientSession::new(network.id, user.id, device.id, None, false);
    new_session.created_at = newer_session_created_at;
    new_session.save(&pool).await.unwrap();

    let user_device = UserDevice::from_device(&pool, device)
        .await
        .unwrap()
        .unwrap();
    let network_info = user_device
        .networks
        .into_iter()
        .find(|network_info| network_info.network_id == network.id)
        .expect("expected created network in user device response");

    assert!(!network_info.is_active);
    assert_eq!(
        network_info.last_connected_at,
        Some(last_successful_connection)
    );
}

#[sqlx::test]
async fn test_user_device_from_device_reads_latest_endpoint_from_session_stats(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();

    let gateway = Gateway::new(network.id, "gateway", "198.51.100.1", 51820, "tester")
        .save(&pool)
        .await
        .unwrap();

    let connected_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 4, 5)
        .expect("expected valid time");
    let collected_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 5, 6)
        .expect("expected valid time");

    let session = VpnClientSession::new(network.id, user.id, device.id, Some(connected_at), false)
        .save(&pool)
        .await
        .unwrap();

    VpnSessionStats::new(
        session.id,
        gateway.id,
        collected_at,
        collected_at,
        "[2001:db8::1]:51820".into(),
        1,
        1,
        1,
        1,
    )
    .save(&pool)
    .await
    .unwrap();

    let user_device = UserDevice::from_device(&pool, device)
        .await
        .unwrap()
        .unwrap();
    let network_info = user_device
        .networks
        .into_iter()
        .find(|network_info| network_info.network_id == network.id)
        .expect("expected created network in user device response");

    assert_eq!(network_info.last_connected_ip, Some("2001:db8::1".into()));
    assert_eq!(network_info.last_connected_at, Some(connected_at));
    assert!(network_info.is_active);
}

#[sqlx::test]
async fn test_user_device_from_device_returns_empty_connection_fields_without_successful_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();

    let gateway = Gateway::new(network.id, "gateway", "198.51.100.1", 51820, "tester")
        .save(&pool)
        .await
        .unwrap();

    let attempted_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 4, 5)
        .expect("expected valid time");
    let stats_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 5, 6)
        .expect("expected valid time");

    let mut attempted_session = VpnClientSession::new(network.id, user.id, device.id, None, false);
    attempted_session.created_at = attempted_at;
    let attempted_session = attempted_session.save(&pool).await.unwrap();

    VpnSessionStats::new(
        attempted_session.id,
        gateway.id,
        stats_at,
        stats_at,
        "203.0.113.10:51820".into(),
        1,
        1,
        1,
        1,
    )
    .save(&pool)
    .await
    .unwrap();

    let user_device = UserDevice::from_device(&pool, device)
        .await
        .unwrap()
        .unwrap();
    let network_info = user_device
        .networks
        .into_iter()
        .find(|network_info| network_info.network_id == network.id)
        .expect("expected created network in user device response");

    assert_eq!(network_info.last_connected_at, None);
    assert_eq!(network_info.last_connected_ip, None);
    assert!(!network_info.is_active);
}

#[sqlx::test]
async fn test_user_device_from_device_returns_none_for_successful_session_without_stats(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();

    let connected_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 4, 5)
        .expect("expected valid time");

    VpnClientSession::new(network.id, user.id, device.id, Some(connected_at), false)
        .save(&pool)
        .await
        .unwrap();

    let user_device = UserDevice::from_device(&pool, device)
        .await
        .unwrap()
        .unwrap();
    let network_info = user_device
        .networks
        .into_iter()
        .find(|network_info| network_info.network_id == network.id)
        .expect("expected created network in user device response");

    assert_eq!(network_info.last_connected_at, Some(connected_at));
    assert_eq!(network_info.last_connected_ip, None);
    assert!(network_info.is_active);
}

#[sqlx::test]
async fn test_user_device_from_device_uses_stats_id_as_tie_breaker_for_latest_successful_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("password"),
        "Tester",
        "Test",
        "test@test.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let device = Device::new(
        "device".into(),
        "pubkey".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    WireguardNetworkDevice::new(
        network.id,
        device.id,
        [IpAddr::from_str("10.1.1.2").unwrap()],
    )
    .insert(&pool)
    .await
    .unwrap();

    let gateway = Gateway::new(network.id, "gateway", "198.51.100.1", 51820, "tester")
        .save(&pool)
        .await
        .unwrap();

    let connected_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 4, 5)
        .expect("expected valid time");
    let collected_at = NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("expected valid date")
        .and_hms_opt(3, 5, 6)
        .expect("expected valid time");

    let session = VpnClientSession::new(network.id, user.id, device.id, Some(connected_at), false)
        .save(&pool)
        .await
        .unwrap();

    VpnSessionStats::new(
        session.id,
        gateway.id,
        collected_at,
        collected_at,
        "198.51.100.10:51820".into(),
        1,
        1,
        1,
        1,
    )
    .save(&pool)
    .await
    .unwrap();

    VpnSessionStats::new(
        session.id,
        gateway.id,
        collected_at,
        collected_at,
        "198.51.100.11:51820".into(),
        2,
        2,
        2,
        2,
    )
    .save(&pool)
    .await
    .unwrap();

    let user_device = UserDevice::from_device(&pool, device)
        .await
        .unwrap()
        .unwrap();
    let network_info = user_device
        .networks
        .into_iter()
        .find(|network_info| network_info.network_id == network.id)
        .expect("expected created network in user device response");

    assert_eq!(network_info.last_connected_ip, Some("198.51.100.11".into()));
    assert_eq!(network_info.last_connected_at, Some(connected_at));
    assert!(network_info.is_active);
}

#[sqlx::test]
async fn test_all_for_network_and_user(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let user = User::new(
        "testuser",
        Some("hunter2"),
        "Tester",
        "Test",
        "email@email.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let user2 = User::new(
        "testuser2",
        Some("hunter2"),
        "Tester",
        "Test",
        "email2@email.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/24")
        .unwrap();
    network.allow_all_groups = true;
    let network = network.save(&pool).await.unwrap();
    let mut network_2 = WireguardNetwork::default()
        .try_set_address("10.1.2.1/24")
        .unwrap();
    network_2.name = "testnetwork2".into();
    network_2.allow_all_groups = true;
    let network2 = network_2.save(&pool).await.unwrap();

    let device = Device::new(
        "testdevice".into(),
        "key".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let device2 = Device::new(
        "testdevice2".into(),
        "key2".into(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let device3 = Device::new(
        "testdevice3".into(),
        "key3".into(),
        user2.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let device4 = Device::new(
        "testdevice4".into(),
        "key4".into(),
        user.id,
        DeviceType::Network,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    let mut transaction = pool.begin().await.unwrap();

    network
        .add_device_to_network(&mut transaction, &device, None)
        .await
        .unwrap();
    network2
        .add_device_to_network(&mut transaction, &device, None)
        .await
        .unwrap();
    network2
        .add_device_to_network(&mut transaction, &device2, None)
        .await
        .unwrap();
    network
        .add_device_to_network(&mut transaction, &device3, None)
        .await
        .unwrap();
    WireguardNetworkDevice::new(
        network.id,
        device4.id,
        [IpAddr::from_str("10.1.1.10").unwrap()],
    )
    .insert(&mut *transaction)
    .await
    .unwrap();

    transaction.commit().await.unwrap();

    let devices = WireguardNetworkDevice::all_for_network_and_user(&pool, network.id, user.id)
        .await
        .unwrap();

    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].device_id, device.id);
}

// Mimic what add_device handler does.
#[sqlx::test]
async fn test_saturated_network(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let user = User::new("tester", None, "Tester", "Test", "test@test.pl", None)
        .save(&pool)
        .await
        .unwrap();

    let mut network = WireguardNetwork::default()
        .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(192, 168, 42, 4)), 29).unwrap()])
        .unwrap();
    network.allow_all_groups = true;
    let network = network.save(&pool).await.unwrap();

    let mut conn = pool.begin().await.unwrap();

    for (name, pubkey) in [
        ("device1", "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU="),
        ("device2", "AJwxGkzvVVn5Q1xjpCDFo5RJSU9KOPHeoEixYaj+20M="),
        ("device3", "OLQNaEH3FxW0hiodaChEHoETzd+7UzcqIbsLs+X8rD0="),
        ("device4", "mgVXE8WcfStoD8mRatHcX5aaQ0DlcpjvPXibHEOr9y8="),
        ("device5", "hNuapt7lOxF93KUqZGUY00oKJxH8LYwwsUVB1uUa0y4="),
    ] {
        let device = Device::new(
            name.to_owned(),
            pubkey.to_owned(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&mut *conn)
        .await
        .unwrap();

        // Register device in the network.
        network
            .add_device_to_network(&mut conn, &device, None)
            .await
            .unwrap();
    }

    // This device won't fit in the address space.
    let _device = Device::new(
        "device6".to_owned(),
        "fF9K0tgatZTEJRvzpNUswr0h8HqCIi+v39B45+QZZzE=".to_owned(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&mut *conn)
    .await
    .unwrap();
    // FIXME: uncomment when `add_to_all_networks` is fixed.
    // assert!(device.add_to_all_networks(&mut conn).await.is_err());

    conn.commit().await.unwrap();

    let devices = Device::all(&pool).await.unwrap();
    assert_eq!(6, devices.len(), "{devices:#?}");
    let network_devices = WireguardNetworkDevice::all_for_network(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(5, network_devices.len(), "{network_devices:#?}");
}
