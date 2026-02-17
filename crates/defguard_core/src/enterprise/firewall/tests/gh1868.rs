use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use defguard_common::db::{
    Id, NoId,
    models::{Device, DeviceType, User, WireguardNetwork, device::WireguardNetworkDevice},
    setup_pool,
};
use defguard_proto::enterprise::firewall::{FirewallPolicy, IpVersion};
use ipnetwork::IpNetwork;
use rand::{Rng, rngs::ThreadRng, thread_rng};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use crate::enterprise::{
    db::models::acl::{AclRule, RuleState},
    firewall::{tests::set_test_license_business, try_get_location_firewall_config},
};

async fn setup_user_and_device(
    rng: &mut ThreadRng,
    pool: &PgPool,
    location: &WireguardNetwork<Id>,
) {
    let user: User<NoId> = rng.r#gen();
    let user = user.save(pool).await.unwrap();

    let device = Device {
        id: NoId,
        name: format!("device-{}", user.id),
        user_id: user.id,
        device_type: DeviceType::User,
        description: None,
        wireguard_pubkey: Default::default(),
        created: Default::default(),
        configured: true,
    };
    let device = device.save(pool).await.unwrap();

    let wireguard_ips = location
        .address
        .iter()
        .map(|subnet| match subnet {
            IpNetwork::V4(ipv4_network) => {
                let octets = ipv4_network.network().octets();
                IpAddr::V4(Ipv4Addr::new(
                    octets[0],
                    octets[1],
                    user.id as u8,
                    device.id as u8,
                ))
            }
            IpNetwork::V6(ipv6_network) => {
                let mut octets = ipv6_network.network().octets();
                // Set the last two octets (bytes 14 and 15)
                octets[14] = user.id as u8;
                octets[15] = device.id as u8;
                IpAddr::V6(Ipv6Addr::from(octets))
            }
        })
        .collect();

    // assign network address to device
    let network_device = WireguardNetworkDevice {
        device_id: device.id,
        wireguard_network_id: location.id,
        wireguard_ips,
        preshared_key: None,
        is_authorized: true,
        authorized_at: None,
    };
    network_device.insert(pool).await.unwrap();
}

#[sqlx::test]
async fn test_gh1868_ipv6_rule_is_not_created_with_v4_only_destination(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;
    let mut rng = thread_rng();

    // Create test location with both IPv4 and IPv6 subnet
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![
            IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 0, 80, 1)), 24).unwrap(),
            IpNetwork::new(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
                64,
            )
            .unwrap(),
        ],
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // setup user & device
    setup_user_and_device(&mut rng, &pool, &location).await;

    // create a rule with only an IPv4 destination
    let acl_rule = AclRule {
        all_locations: true,
        allow_all_users: true,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        any_address: false,
        addresses: vec!["192.168.1.0/24".parse().unwrap()],
        use_manual_destination_settings: true,
        enabled: true,
        state: RuleState::Applied,
        ..Default::default()
    };
    acl_rule.save(&pool).await.unwrap();

    // verify only IPv4 rules are created
    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap();
    let generated_firewall_rules = generated_firewall_config.rules;
    println!("{generated_firewall_rules:#?}");
    assert_eq!(generated_firewall_rules.len(), 2);

    let allow_rule = &generated_firewall_rules[0];
    assert_eq!(allow_rule.verdict(), FirewallPolicy::Allow);
    assert_eq!(allow_rule.ip_version(), IpVersion::Ipv4);

    let deny_rule = &generated_firewall_rules[1];
    assert_eq!(deny_rule.verdict(), FirewallPolicy::Deny);
    assert_eq!(allow_rule.ip_version(), IpVersion::Ipv4);
}

#[sqlx::test]
async fn test_gh1868_ipv4_rule_is_not_created_with_v6_only_destination(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test location with both IPv4 and IPv6 subnet
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![
            IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 0, 80, 1)), 24).unwrap(),
            IpNetwork::new(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
                64,
            )
            .unwrap(),
        ],
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // setup user & device
    setup_user_and_device(&mut rng, &pool, &location).await;

    // create a rule with only an IPv6 destination
    let acl_rule = AclRule {
        all_locations: true,
        allow_all_users: true,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        any_address: false,
        addresses: vec!["fc00::0/112".parse().unwrap()],
        enabled: true,
        state: RuleState::Applied,
        ..Default::default()
    };
    acl_rule.save(&pool).await.unwrap();

    // verify only IPv6 rules are created
    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap();
    let generated_firewall_rules = generated_firewall_config.rules;
    assert_eq!(generated_firewall_rules.len(), 2);

    let allow_rule = &generated_firewall_rules[0];
    assert_eq!(allow_rule.verdict, i32::from(FirewallPolicy::Allow));
    assert_eq!(allow_rule.ip_version, i32::from(IpVersion::Ipv6));

    let deny_rule = &generated_firewall_rules[1];
    assert_eq!(deny_rule.verdict, i32::from(FirewallPolicy::Deny));
    assert_eq!(allow_rule.ip_version, i32::from(IpVersion::Ipv6));
}

#[sqlx::test]
async fn test_gh1868_ipv4_and_ipv6_rules_are_created_with_any_destination(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let pool = setup_pool(options).await;

    let mut rng = thread_rng();

    // Create test location with both IPv4 and IPv6 subnet
    let location = WireguardNetwork {
        id: NoId,
        acl_enabled: true,
        address: vec![
            IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 0, 80, 1)), 24).unwrap(),
            IpNetwork::new(
                IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
                64,
            )
            .unwrap(),
        ],
        ..Default::default()
    };
    let location = location.save(&pool).await.unwrap();

    // setup user & device
    setup_user_and_device(&mut rng, &pool, &location).await;

    // create a rule with any destination enabled
    let acl_rule = AclRule {
        all_locations: true,
        allow_all_users: true,
        deny_all_users: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        any_address: true,
        addresses: vec!["fc00::0/112".parse().unwrap()],
        enabled: true,
        state: RuleState::Applied,
        ..Default::default()
    };
    acl_rule.save(&pool).await.unwrap();

    // verify only IPv4 rules are created
    let mut conn = pool.acquire().await.unwrap();
    let generated_firewall_config = try_get_location_firewall_config(&location, &mut conn)
        .await
        .unwrap()
        .unwrap();
    let generated_firewall_rules = generated_firewall_config.rules;
    assert_eq!(generated_firewall_rules.len(), 4);

    let allow_rule_ipv4 = &generated_firewall_rules[0];
    assert_eq!(allow_rule_ipv4.verdict(), FirewallPolicy::Allow);
    assert_eq!(allow_rule_ipv4.ip_version(), IpVersion::Ipv4);
    let allow_rule_ipv6 = &generated_firewall_rules[1];
    assert_eq!(allow_rule_ipv6.verdict(), FirewallPolicy::Allow);
    assert_eq!(allow_rule_ipv6.ip_version(), IpVersion::Ipv6);

    let deny_rule_ipv4 = &generated_firewall_rules[2];
    assert_eq!(deny_rule_ipv4.verdict(), FirewallPolicy::Deny);
    assert_eq!(allow_rule_ipv4.ip_version(), IpVersion::Ipv4);
    let deny_rule_ipv6 = &generated_firewall_rules[3];
    assert_eq!(deny_rule_ipv6.verdict(), FirewallPolicy::Deny);
    assert_eq!(allow_rule_ipv6.ip_version(), IpVersion::Ipv6);
}
