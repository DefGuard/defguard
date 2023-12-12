mod common;

use common::ClientState;
use defguard::{
    db::models::settings::{Settings, SettingsPatch},
    handlers::Auth,
};
use reqwest::StatusCode;

use self::common::{client::TestClient, make_test_client};

async fn make_client() -> (TestClient, ClientState) {
    let (client, state) = make_test_client().await;
    (client, state)
}

#[tokio::test]
async fn test_settings() {
    let (client, _client_state) = make_client().await;
    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    // get settings
    let response = client.get("/api/v1/settings").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut settings: Settings = response.json().await;
    // modify settings
    settings.wireguard_enabled = false;
    settings.challenge_template = "Modified".to_string();
    let response = client.put("/api/v1/settings").json(&settings).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    // verify modified settings
    let response = client.get("/api/v1/settings").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let new_settings: Settings = response.json().await;
    assert_eq!(new_settings, settings);
    // patch settings
    let patch_json: &str = r#"
    {
        "wireguard_enabled": true
    }"#;
    let settings_patch: SettingsPatch = serde_json::from_str(patch_json).unwrap();
    let response = client
        .patch("/api/v1/settings")
        .json(&settings_patch)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/settings").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let new_settings: Settings = response.json().await;
    assert!(new_settings.wireguard_enabled);
}
