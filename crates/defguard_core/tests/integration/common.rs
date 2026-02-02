use defguard_common::{
    config::{DefGuardConfig, SERVER_CONFIG},
    db::models::{Settings, User, settings::update_current_settings},
};
use reqwest::Url;
use secrecy::ExposeSecret;
use sqlx::PgPool;

/// Allows overriding the default DefGuard URL for tests, as during the tests, the server has a random port, making the URL unpredictable beforehand.
// TODO: Allow customizing the whole config, not just the URL
pub(crate) async fn init_config(
    custom_defguard_url: Option<&str>,
    pool: &PgPool,
) -> DefGuardConfig {
    let url = custom_defguard_url.unwrap_or("http://localhost:8000");
    let mut config = DefGuardConfig::new_test_config();
    config.url = Url::parse(url).unwrap();
    let _ = SERVER_CONFIG.set(config.clone());

    let mut settings = Settings::get_current_settings();
    settings.defguard_url = url.to_string();
    update_current_settings(pool, settings)
        .await
        .expect("Could not update current settings in the database");

    config
}

pub(crate) async fn initialize_users(pool: &PgPool, config: &DefGuardConfig) {
    User::init_admin_user(pool, config.default_admin_password.expose_secret())
        .await
        .unwrap();

    User::new(
        "hpotter",
        Some("pass123"),
        "Potter",
        "Harry",
        "h.potter@hogwart.edu.uk",
        None,
    )
    .save(pool)
    .await
    .unwrap();
}
