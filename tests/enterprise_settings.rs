// mod common;

use defguard::handlers::Auth;
use reqwest::StatusCode;

use self::common::make_test_client;
use defguard::{
    db::{
        models::{
            device::WireguardNetworkDevice,
            wireguard::{DEFAULT_DISCONNECT_THRESHOLD, DEFAULT_KEEPALIVE_INTERVAL},
        },
        Device, GatewayEvent, WireguardNetwork,
    },
    handlers::{wireguard::WireguardNetworkData, Auth, GroupInfo},
};
use matches::assert_matches;
use serde_json::{json, Value};

#[tokio::test]
async fn test_only_enterprise_can_modify() {
    let (client, client_state) = make_test_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .post("/api/v1/openid/provider")
        .json(&provider_data)
        .send()
        .await;

}
