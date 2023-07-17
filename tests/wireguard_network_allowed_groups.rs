use defguard::db::{DbPool, Device, GatewayEvent, Group, User, WireguardNetwork};
use defguard::handlers::Auth;
use matches::assert_matches;
use rocket::http::Status;
use rocket::serde::json::json;

mod common;
use crate::common::make_test_client;

// setup user groups, test users and devices
async fn setup_test_users(pool: &DbPool) -> (Vec<User>, Vec<Device>) {
    let mut users = Vec::new();
    let mut devices = Vec::new();
    // create user groups
    let mut allowed_group = Group::new("allowed group");
    allowed_group.save(pool).await.unwrap();

    let mut not_allowed_group = Group::new("not allowed group");
    not_allowed_group.save(pool).await.unwrap();

    // admin user
    let admin_user = User::find_by_username(pool, "admin")
        .await
        .unwrap()
        .unwrap();
    let mut admin_device = Device::new(
        "admin device".into(),
        "nst4lmZz9kPTq6OdeQq2G2th3n+QneHKmG1wJJ3Jrq0=".into(),
        admin_user.id.unwrap(),
    );
    admin_device.save(pool).await.unwrap();
    users.push(admin_user);
    devices.push(admin_device);

    // standard user in allowed group
    let test_user = User::find_by_username(pool, "hpotter")
        .await
        .unwrap()
        .unwrap();
    test_user.add_to_group(pool, &allowed_group).await.unwrap();
    let mut test_device = Device::new(
        "test device".into(),
        "wYOt6ImBaQ3BEMQ3Xf5P5fTnbqwOvjcqYkkSBt+1xOg=".into(),
        test_user.id.unwrap(),
    );
    test_device.save(pool).await.unwrap();
    users.push(test_user);
    devices.push(test_device);

    // standard user in other, non-allowed group
    let mut other_user = User::new(
        "ssnape".into(),
        "pass123",
        "Snape".into(),
        "Severus".into(),
        "s.snape@hogwart.edu.uk".into(),
        None,
    );
    other_user.save(pool).await.unwrap();
    other_user
        .add_to_group(pool, &not_allowed_group)
        .await
        .unwrap();
    let mut other_device = Device::new(
        "other device".into(),
        "v2U14sjNN4tOYD3P15z0WkjriKY9Hl85I3vIEPomrYs=".into(),
        other_user.id.unwrap(),
    );
    other_device.save(pool).await.unwrap();
    users.push(other_user);
    devices.push(other_device);

    // standard user in no groups
    let mut non_group_user = User::new(
        "dobby".into(),
        "pass123",
        "Elf".into(),
        "Dobby".into(),
        "dobby@hogwart.edu.uk".into(),
        None,
    );
    non_group_user.save(pool).await.unwrap();
    let mut non_group_device = Device::new(
        "non group device".into(),
        "6xmL/jRuxmzQ3J2/kVZnKnh+6dwODcEEczmmkIKU4sM=".into(),
        non_group_user.id.unwrap(),
    );
    non_group_device.save(pool).await.unwrap();
    users.push(non_group_user);
    devices.push(non_group_device);

    (users, devices)
}

#[rocket::async_test]
async fn test_create_new_network() {
    let (client, client_state) = make_test_client().await;
    let (_users, devices) = setup_test_users(&client_state.pool).await;

    let mut wg_rx = client_state.wireguard_rx;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).dispatch().await;
    assert_eq!(response.status(), Status::Ok);

    // create network
    let response = client
        .post("/api/v1/network")
        .json(&json!({
            "name": "network",
            "address": "10.1.1.1/24",
            "port": 55555,
            "endpoint": "192.168.4.14",
            "allowed_ips": "10.1.1.0/24",
            "dns": "1.1.1.1",
            "allowed_groups": ["allowed group"],
        }))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created);
    let network: WireguardNetwork = response.into_json().await.unwrap();
    assert_eq!(network.name, "network");
    let event = wg_rx.try_recv().unwrap();
    assert_matches!(event, GatewayEvent::NetworkCreated(..));

    // network configuration was created only for admin and allowed user
    let peers = network.get_peers(&client_state.pool).await.unwrap();
    assert_eq!(peers.len(), 2);
    assert_eq!(peers[0].pubkey, devices[0].wireguard_pubkey);
    assert_eq!(peers[1].pubkey, devices[1].wireguard_pubkey);
}
