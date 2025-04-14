use crate::common::{make_client_with_state, setup_pool};
use defguard::{
    db::models::settings::{Settings, SettingsPatch},
    handlers::Auth,
};
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[sqlx::test]
async fn test_settings(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _client_state) = make_client_with_state(pool).await;
    let auth = Auth::new("admin", "pass123");
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
