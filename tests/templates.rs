mod common;

use claims::assert_ok;
use common::{client::TestClient, make_test_client, ClientState};
use defguard::{
    templates::{
        desktop_start_mail, enrollment_admin_notification, enrollment_start_mail,
        enrollment_welcome_mail, get_base_tera, mfa_configured_mail, new_device_added_mail,
        support_data_mail, test_mail, TemplateLocation,
    },
    VERSION,
};
use reqwest::Url;
use tera::Context;

async fn make_client() -> (TestClient, ClientState) {
    let (client, client_state) = make_test_client().await;
    (client, client_state)
}

fn get_welcome_context() -> Context {
    let mut context = Context::new();
    context.insert("first_name", "test_first");
    context.insert("last_name", "test_last");
    context.insert("username", "username");
    context.insert("defguard_url", "test_url");
    context.insert("defguard_version", &VERSION);
    context.insert("admin_first_name", "test_first_name");
    context.insert("admin_last_name", "test_last_name");
    context.insert("admin_email", "test_email");
    context.insert("admin_phone", "test_phone");
    context
}

#[tokio::test]
async fn test_base_mail_no_context() {
    assert_ok!(get_base_tera(None));
}

#[tokio::test]
async fn test_base_mail_external_context() {
    let external_context: Context = Context::new();
    assert_ok!(get_base_tera(Some(external_context)));
}

#[tokio::test]
async fn test_test_mail() {
    assert_ok!(test_mail());
}

#[tokio::test]
async fn test_enrollment_start_mail() {
    assert_ok!(enrollment_start_mail(
        Context::new(),
        Url::parse("http://localhost:8080").unwrap(),
        "test_token"
    ));
}

#[tokio::test]
async fn test_enrollment_welcome_mail() {
    assert_ok!(enrollment_welcome_mail("Hi there! Welcome to DefGuard."));
}

#[tokio::test]
async fn test_support_data_mail() {
    assert_ok!(support_data_mail());
}

#[tokio::test]
async fn test_desktop_start_mail() {
    let external_context = get_welcome_context();
    let url = Url::parse("http://127.0.0.1:8080").unwrap();
    let token = "TestToken";
    assert_ok!(desktop_start_mail(external_context, url, token));
}

#[tokio::test]
async fn test_enrollment_admin_notification() {
    let (_, state) = make_client().await;
    assert_ok!(enrollment_admin_notification(
        &state.test_user,
        &state.test_user
    ));
}

#[tokio::test]
async fn test_new_device_added_mail() {
    let template_locations: Vec<TemplateLocation> = vec![
        TemplateLocation {
            name: "Test 01".into(),
            assigned_ip: "10.0.0.10".into(),
        },
        TemplateLocation {
            name: "Test 02".into(),
            assigned_ip: "10.0.0.10".into(),
        },
    ];
    assert_ok!(new_device_added_mail(
        "Test device",
        "TestKey",
        &template_locations
    ));
}

#[tokio::test]
async fn test_mfa_configured() {
    assert_ok!(mfa_configured_mail("TOTP".into()));
}
