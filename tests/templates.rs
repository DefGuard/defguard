mod common;

use self::common::{client::TestClient, make_test_client};
use claims::assert_ok;
use common::ClientState;
use defguard::{
    db::User,
    handlers::AddUserData,
    templates::{
        desktop_start_mail, enrollment_admin_notification, enrollment_start_mail,
        enrollment_welcome_mail, get_base_tera, mfa_configured_mail, new_device_added_mail,
        support_data_mail, test_mail, TemplateLocation,
    },
    VERSION,
};
use hyper::StatusCode;
use reqwest::Url;
use tera::Context;

async fn make_client() -> (TestClient, ClientState, User, User) {
    let (client, state) = make_test_client().await;

    let pool = state.pool.clone();

    let admin_user = User::find_by_username(&pool, "admin")
        .await
        .unwrap()
        .unwrap();

    let test_user_add = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: Some("1234".into()),
        password: Some("Password1234543$!".into()),
    };

    let response = client
        .post("/api/v1/user")
        .json(&test_user_add)
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);

    let test_user = User::find_by_username(&pool, "adumbeldore")
        .await
        .unwrap()
        .unwrap();

    (client, state, admin_user, test_user)
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
    let (_client, _state, admin, user) = make_client().await;
    assert_ok!(enrollment_admin_notification(&user, &admin));
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
