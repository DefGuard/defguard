use std::{env, str::FromStr, time::Duration};

use defguard_common::{
    config::{DefGuardConfig, SERVER_CONFIG},
    db::{
        models::{
            Settings,
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

use super::templates::{
    TemplateLocation, desktop_start_mail, mfa_code_mail, new_account_mail, new_device_added_mail,
};

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

#[ignore]
#[sqlx::test]
fn send_desktop_start(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut transaction = pool.begin().await.unwrap();
    let context = Context::new();
    let url = Url::parse("http://localhost:8000").unwrap();
    let token = "zXc6N1ndXpWFeyBuogiFp1bD1UomAbZc";
    desktop_start_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut transaction,
        context,
        &url,
        token,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    tokio::time::sleep(Duration::from_secs(2)).await;
}

#[ignore]
#[sqlx::test]
fn send_new_device_added(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut transaction = pool.begin().await.unwrap();
    let device_name = "My beloved machine";
    let public_key = "6N8h7HILMcQ6nqEfQMBAYQH26X+y3t/WdWSOW4bNNxw=";
    let locations = &[
        TemplateLocation {
            name: String::from("Location 1"),
            assigned_ips: String::from("192.168.1.42"),
        },
        TemplateLocation {
            name: String::from("Location 2"),
            assigned_ips: String::from("192.168.2.69"),
        },
    ];
    new_device_added_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut transaction,
        device_name,
        public_key,
        locations,
        Some("1.2.3.4"),
        Some("unknown device"),
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    tokio::time::sleep(Duration::from_secs(2)).await;
}

#[ignore]
#[sqlx::test]
fn send_mfa_code(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut transaction = pool.begin().await.unwrap();
    let first_name = "Nebuchadnezzar";
    let code = "123456";
    mfa_code_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut transaction,
        first_name,
        code,
        None,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    tokio::time::sleep(Duration::from_secs(2)).await;
}

#[ignore]
#[sqlx::test]
fn send_new_account(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_smtp_settings(&pool).await;

    let mut transaction = pool.begin().await.unwrap();
    let url = Url::parse("http://localhost:8000").unwrap();
    let context = Context::new();
    let token = "zXc6N1ndXpWFeyBuogiFp1bD1UomAbZc";
    new_account_mail(
        &env::var("SMTP_TO").unwrap(),
        &mut transaction,
        context,
        url,
        token,
    )
    .await
    .unwrap();

    // Delay, so send_and_forget() can process the message.
    tokio::time::sleep(Duration::from_secs(2)).await;
}
