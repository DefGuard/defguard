mod common;

use axum::http::StatusCode;
use defguard::{
    db::models::settings::{Settings, SmtpEncryption},
    handlers::Auth,
};

use self::common::{client::TestClient, make_test_client};

async fn make_client() -> TestClient {
    let (client, _) = make_test_client().await;
    client
}

#[tokio::test]
async fn test_settings() {
    let client = make_client().await;

    let auth = Auth::new("admin".into(), "pass123".into());
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // get settings
    let response = client.get("/api/v1/settings").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut settings: Settings = response.json().await;
    assert_eq!(
        settings,
        Settings {
            id: None,
            openid_enabled: true,
            ldap_enabled: true,
            wireguard_enabled: true,
            webhooks_enabled: true,
            worker_enabled: true,
            main_logo_url: "/svg/logo-defguard-white.svg".into(),
            nav_logo_url: "/svg/defguard-nav-logo.svg".into(),
            instance_name: "Defguard".into(),
            challenge_template: "
                Please read this carefully:\n\n\
                Click to sign to prove you are in possesion of your private key to the account.\n\
                This request will not trigger a blockchain transaction or cost any gas fees."
                .trim_start()
                .into(),
            smtp_server: None,
            smtp_port: None,
            smtp_encryption: SmtpEncryption::StartTls,
            smtp_user: None,
            smtp_password: None,
            smtp_sender: None,
            enrollment_vpn_step_optional: true,
            enrollment_welcome_message: None,
            enrollment_welcome_email: None,
            enrollment_welcome_email_subject: None,
            enrollment_use_welcome_message_as_email: true,
            uuid: settings.uuid,
        }
    );

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
}
