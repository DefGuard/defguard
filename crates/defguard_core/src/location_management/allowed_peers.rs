use defguard_common::db::{Id, models::WireguardNetwork};
use defguard_proto::gateway::Peer;
use sqlx::{Error as SqlxError, PgExecutor, query};

use crate::grpc::gateway::should_prevent_service_location_usage;

/// Get a list of all allowed peers for a given location
///
/// Each device is marked as allowed or not allowed in a given network,
/// which enables enforcing peer disconnect in MFA-protected networks.
///
/// If the location is a service location, only returns peers if enterprise features are enabled.
pub async fn get_location_allowed_peers<'e, E>(
    location: &WireguardNetwork<Id>,
    executor: E,
) -> Result<Vec<Peer>, SqlxError>
where
    E: PgExecutor<'e>,
{
    debug!("Fetching all allowed peers for location {}", location.id);

    if should_prevent_service_location_usage(location) {
        warn!(
            "Tried to use service location {} with disabled enterprise features. No clients will be allowed to connect.",
            location.name
        );
        return Ok(Vec::new());
    }

    let rows = query!(
        "SELECT d.wireguard_pubkey pubkey, preshared_key, \
                -- TODO possible to not use ARRAY-unnest here?
                ARRAY(
                    SELECT host(ip)
                    FROM unnest(wnd.wireguard_ips) AS ip
                ) \"allowed_ips!: Vec<String>\" \
            FROM wireguard_network_device wnd \
            JOIN device d ON wnd.device_id = d.id \
            JOIN \"user\" u ON d.user_id = u.id \
            WHERE wireguard_network_id = $1 AND (is_authorized = true OR NOT $2) \
            AND d.configured = true \
            AND u.is_active = true \
            ORDER BY d.id ASC",
        location.id,
        location.mfa_enabled()
    )
    .fetch_all(executor)
    .await?;

    // keepalive has to be added manually because Postgres
    // doesn't support unsigned integers
    let result = rows
        .into_iter()
        .map(|row| Peer {
            pubkey: row.pubkey,
            allowed_ips: row.allowed_ips,
            // Don't send preshared key if MFA is not enabled, it can't be used and may
            // cause issues with clients connecting if they expect no preshared key
            // e.g. when you disable MFA on a location
            preshared_key: if location.mfa_enabled() {
                row.preshared_key
            } else {
                None
            },
            keepalive_interval: Some(location.keepalive_interval as u32),
        })
        .collect();

    Ok(result)
}

#[cfg(test)]
mod test {
    use std::{net::IpAddr, str::FromStr};

    use defguard_common::db::{
        models::{
            Device, DeviceType, WireguardNetwork,
            device::WireguardNetworkDevice,
            user::User,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
        setup_pool,
    };
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use crate::location_management::allowed_peers::get_location_allowed_peers;

    #[ignore]
    #[sqlx::test]
    async fn test_get_peers_service_location_modes(_: PgPoolOptions, options: PgConnectOptions) {
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

        let device1 = Device::new(
            "device1".into(),
            "pubkey1".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        let device2 = Device::new(
            "device2".into(),
            "pubkey2".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        // Normal location (service_location_mode = Disabled) should return peers
        let mut network_normal = WireguardNetwork {
            name: "normal-location".to_string(),
            service_location_mode: ServiceLocationMode::Disabled,
            location_mfa_mode: LocationMfaMode::Disabled,
            ..Default::default()
        };
        network_normal.try_set_address("10.1.1.1/24").unwrap();
        let network_normal = network_normal.save(&pool).await.unwrap();

        WireguardNetworkDevice::new(
            network_normal.id,
            device1.id,
            vec![IpAddr::from_str("10.1.1.2").unwrap()],
        )
        .insert(&pool)
        .await
        .unwrap();

        let peers_normal = get_location_allowed_peers(&network_normal, &pool)
            .await
            .unwrap();
        assert_eq!(peers_normal.len(), 1, "Normal location should return peers");
        assert_eq!(peers_normal[0].pubkey, "pubkey1");

        // Service location with PreLogon mode returns peers when enterprise is enabled (test env default)
        let mut network_prelogon = WireguardNetwork {
            name: "prelogon-service-location".to_string(),
            service_location_mode: ServiceLocationMode::PreLogon,
            location_mfa_mode: LocationMfaMode::Disabled,
            ..Default::default()
        };
        network_prelogon.try_set_address("10.2.1.1/24").unwrap();
        let network_prelogon = network_prelogon.save(&pool).await.unwrap();

        WireguardNetworkDevice::new(
            network_prelogon.id,
            device2.id,
            vec![IpAddr::from_str("10.2.1.2").unwrap()],
        )
        .insert(&pool)
        .await
        .unwrap();

        // PreLogon service location should return peers when enterprise is enabled
        let peers_prelogon = get_location_allowed_peers(&network_prelogon, &pool)
            .await
            .unwrap();
        assert_eq!(
            peers_prelogon.len(),
            1,
            "PreLogon service location should return peers when enterprise is enabled"
        );
        assert_eq!(peers_prelogon[0].pubkey, "pubkey2");

        // Service location with AlwaysOn mode also returns peers when enterprise is enabled
        let mut network_alwayson = WireguardNetwork {
            name: "alwayson-service-location".to_string(),
            service_location_mode: ServiceLocationMode::AlwaysOn,
            location_mfa_mode: LocationMfaMode::Disabled,
            ..Default::default()
        };
        network_alwayson.try_set_address("10.3.1.1/24").unwrap();
        let network_alwayson = network_alwayson.save(&pool).await.unwrap();

        let device3 = Device::new(
            "device3".into(),
            "pubkey3".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        WireguardNetworkDevice::new(
            network_alwayson.id,
            device3.id,
            vec![IpAddr::from_str("10.3.1.2").unwrap()],
        )
        .insert(&pool)
        .await
        .unwrap();

        // AlwaysOn service location should return peers when enterprise is enabled
        let peers_alwayson = get_location_allowed_peers(&network_alwayson, &pool)
            .await
            .unwrap();
        assert_eq!(
            peers_alwayson.len(),
            1,
            "AlwaysOn service location should return peers when enterprise is enabled"
        );
        assert_eq!(peers_alwayson[0].pubkey, "pubkey3");
    }
}
