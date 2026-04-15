use std::{env, str::FromStr, time::Duration};

use chrono::Utc;
use defguard_common::{
    config::{DefGuardConfig, SERVER_CONFIG},
    db::{
        models::{
            MFAMethod, Settings,
            settings::{SmtpEncryption, initialize_current_settings, set_settings},
        },
        setup_pool,
    },
    secret::SecretStringWrapper,
};
use reqwest::Url;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tera::Context;
use tokio::time::sleep;

use super::{Attachment, mail::MailMessage, templates};

#[test]
fn dg25_8_server_side_template_injection() {
    let mut tera = templates::safe_tera();
    tera.add_raw_template("text", "PATH={{ get_env(name=\"PATH\") }}")
        .unwrap();
    assert!(tera.render("text", &Context::new()).is_err());
}

/// Set SMTP settings from environment variables.
async fn set_smtp_settings(pool: &PgPool) {
    let config = DefGuardConfig::new_test_config();
    let _ = SERVER_CONFIG.set(config);
    initialize_current_settings(pool).await.unwrap();

    let mut settings = Settings::get_current_settings();
    settings.smtp_server = env::var("SMTP_SERVER").ok();
    settings.smtp_port = Some(env::var("SMTP_PORT").map_or(587, |s| s.parse().unwrap()));
    settings.smtp_encryption = SmtpEncryption::StartTls;
    settings.smtp_user = env::var("SMTP_USER").ok();
    settings.smtp_password =
        Some(SecretStringWrapper::from_str(&env::var("SMTP_PASSWORD").unwrap()).unwrap());
    settings.smtp_sender = env::var("SMTP_FROM").ok();
    set_settings(Some(settings));
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_desktop_start(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let context = Context::new();
    let url = Url::parse("http://localhost:8001").unwrap();
    let token = "zXc6N1ndXpWFeyBuogiFp1bD1UomAbZc";
    templates::desktop_start_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        context,
        &url,
        token,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_new_device_added(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let device_name = "My beloved machine";
    let public_key = "6N8h7HILMcQ6nqEfQMBAYQH26X+y3t/WdWSOW4bNNxw=";
    let locations = &[
        templates::TemplateLocation {
            name: String::from("Location 1"),
            assigned_ips: String::from("192.168.1.42"),
        },
        templates::TemplateLocation {
            name: String::from("Location 2"),
            assigned_ips: String::from("192.168.2.69"),
        },
    ];
    templates::new_device_added_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        device_name,
        public_key,
        locations,
        Some("1.2.3.4"),
        Some("unknown device"),
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_mfa_code(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let first_name = "Nebuchadnezzar";
    let code = "123456";
    templates::mfa_code_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        first_name,
        code,
        None,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_new_account(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let url = Url::parse("http://localhost:8001").unwrap();
    let context = Context::new();
    let token = "zXc6N1ndXpWFeyBuogiFp1bD1UomAbZc";
    templates::new_account_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        context,
        url,
        token,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_mfa_activation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let first_name = "Nebuchadnezzar";
    let code = "123456";
    templates::mfa_activation_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        first_name,
        code,
        None,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_enrollment_admin_notification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let user_name = "Nebuchadnezzar the Great";
    let admin_name = "Nabopolassar the Admin";
    let ip_address = "1.2.3.4";
    templates::enrollment_admin_notification(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        user_name,
        admin_name,
        ip_address,
        None,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_gateway_disconnected_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let gateway_name = "Portal";
    let ip_address = "1.2.3.4";
    let location_name = "Somewhere";
    templates::gateway_disconnected_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        gateway_name,
        ip_address,
        location_name,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_gateway_reconnected_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let gateway_name = "Portal";
    let ip_address = "1.2.3.4";
    let location_name = "Somewhere";
    templates::gateway_reconnected_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        gateway_name,
        ip_address,
        location_name,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_mfa_configured_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    templates::mfa_configured_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        None,
        &MFAMethod::Email,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_new_device_login_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let created = Utc::now().naive_utc();
    templates::new_device_login_mail(&env::var("SMTP_TO").unwrap(), &mut conn, None, created)
        .await
        .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_new_device_oidc_login_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let client_name = "RemoteApp";
    let username = "testuser";
    templates::new_device_oidc_login_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        None,
        client_name,
        username,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_password_reset_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let proxy_url = Url::parse("http://localhost:8000").unwrap();
    let token = "blablabla";
    templates::password_reset_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut conn,
        proxy_url,
        token,
        None,
        None,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_password_reset_success_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    templates::password_reset_success_mail(&env::var("SMTP_TO").unwrap(), &mut conn, None, None)
        .await
        .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_test_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    templates::test_mail(&env::var("SMTP_TO").unwrap(), &mut conn, None)
        .await
        .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_support_data_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut conn = pool.begin().await.unwrap();
    let config = Attachment::new(
        "defguard-support-data-test.json".into(),
        b"{\"key\":\"value\"}".into(),
    );
    templates::support_data_mail(&env::var("SMTP_TO").unwrap(), &mut conn, vec![config])
        .await
        .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[ignore = "requires SMTP server"]
#[sqlx::test]
fn send_enrollment_welcome_mail(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let markdown = "Paragraph **bold** _italic_.";
    templates::enrollment_welcome_mail(&env::var("SMTP_TO").unwrap(), markdown, None, None)
        .unwrap();

    // Delay, so send_and_forget() can process the message.
    sleep(Duration::from_secs(2)).await;
}

#[test]
fn test_mfa_configured_subject_totp() {
    // TOTP
    let subject = MailMessage::MFAConfigured {
        method: MFAMethod::OneTimePassword,
    }
    .subject();
    assert!(
        subject.contains("TOTP"),
        "MFAConfigured subject for OneTimePassword must mention \"TOTP\", got: {subject:?}"
    );

    // WebAuthn
    let subject = MailMessage::MFAConfigured {
        method: MFAMethod::Webauthn,
    }
    .subject();
    assert!(
        subject.contains("WebAuthn"),
        "MFAConfigured subject for Webauthn must mention \"WebAuthn\", got: {subject:?}"
    );

    // email
    let subject = MailMessage::MFAConfigured {
        method: MFAMethod::Email,
    }
    .subject();
    assert!(
        subject.contains("Email"),
        "MFAConfigured subject for Email must mention \"Email\", got: {subject:?}"
    );
}
