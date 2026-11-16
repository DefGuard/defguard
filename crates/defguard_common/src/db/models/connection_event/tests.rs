use chrono::{TimeDelta, Utc};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

use super::*;
use crate::db::{
    models::{Device, DeviceType, User, WireguardNetwork, device::WireguardNetworkDevice},
    setup_pool,
};

fn build_event(location_id: Id, timestamp: NaiveDateTime) -> ConnectionEvent {
    ConnectionEvent {
        id: NoId,
        location_id,
        gateway_id: 1,
        timestamp,
        direction: ConnectionDirection::Inbound,
        status: ConnectionStatus::Allowed,
        protocol: 17,
        source_ip: "10.0.0.2".parse().unwrap(),
        source_port: Some(5353),
        destination_ip: "fd00::1".parse().unwrap(),
        destination_port: None,
        rule_id: None,
        device_id: None,
        upload_bytes: 1,
        download_bytes: 2,
        count: 1,
    }
}

#[sqlx::test]
async fn test_save_and_delete_older_than(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = WireguardNetwork::default().save(&pool).await.unwrap();

    let now = Utc::now().naive_utc();
    build_event(location.id, now - TimeDelta::days(10))
        .save(&pool)
        .await
        .unwrap();
    build_event(location.id, now).save(&pool).await.unwrap();

    let count = ConnectionEvent::count(&pool).await.unwrap();
    assert_eq!(count, 2);

    let removed = ConnectionEvent::delete_older_than(&pool, now - TimeDelta::days(1))
        .await
        .unwrap();
    assert_eq!(removed, 1);

    let count = ConnectionEvent::count(&pool).await.unwrap();
    assert_eq!(count, 1);
}

async fn add_device(pool: &PgPool, location_id: Id, user_id: Id, name: &str, ip: &str) -> Id {
    let device = Device::new(
        name.to_owned(),
        format!("{name}-pubkey"),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap();
    WireguardNetworkDevice::new(location_id, device.id, vec![ip.parse().unwrap()])
        .insert(pool)
        .await
        .unwrap();
    device.id
}

#[sqlx::test]
async fn test_save_all_with_device(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let location = WireguardNetwork::default().save(&pool).await.unwrap();
    let other_location = WireguardNetwork::default().save(&pool).await.unwrap();
    let user = User::new(
        "testuser",
        None,
        "LastName",
        "FirstName",
        "test@example.com",
        None,
    )
    .save(&pool)
    .await
    .unwrap();
    let laptop = add_device(&pool, location.id, user.id, "laptop", "10.0.0.2").await;
    let phone = add_device(&pool, location.id, user.id, "phone", "10.0.0.3").await;
    let now = Utc::now().naive_utc();
    // `count` tells the stored events apart.
    let event = |count, direction, source_ip: &str, destination_ip: &str| ConnectionEvent {
        count,
        direction,
        source_ip: source_ip.parse().unwrap(),
        destination_ip: destination_ip.parse().unwrap(),
        ..build_event(location.id, now)
    };
    let events = [
        // Inbound traffic: the device is the destination.
        event(1, ConnectionDirection::Inbound, "192.168.1.1", "10.0.0.3"),
        // Only the other side is a known device.
        event(2, ConnectionDirection::Inbound, "10.0.0.2", "192.168.1.1"),
        // Both sides are devices: outbound traffic prefers the source.
        event(3, ConnectionDirection::Outbound, "10.0.0.2", "10.0.0.3"),
        // Same addresses in another location.
        ConnectionEvent {
            location_id: other_location.id,
            ..event(4, ConnectionDirection::Outbound, "10.0.0.2", "10.0.0.3")
        },
    ];

    assert_eq!(
        ConnectionEvent::save_all_with_device(&pool, &[])
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        ConnectionEvent::save_all_with_device(&pool, &events)
            .await
            .unwrap(),
        4
    );

    let mut stored = ConnectionEvent::all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|event| (event.count, event.device_id))
        .collect::<Vec<_>>();
    stored.sort_unstable();
    assert_eq!(
        stored,
        vec![
            (1, Some(phone)),
            (2, Some(laptop)),
            (3, Some(laptop)),
            (4, None)
        ]
    );
}
