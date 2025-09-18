use defguard_core::{SERVER_CONFIG, config::DefGuardConfig, db::User};
use reqwest::Url;
use secrecy::ExposeSecret;
use sqlx::PgPool;

/// Allows overriding the default DefGuard URL for tests, as during the tests, the server has a random port, making the URL unpredictable beforehand.
// TODO: Allow customizing the whole config, not just the URL
pub(crate) fn init_config(custom_defguard_url: Option<&str>) -> DefGuardConfig {
    let url = custom_defguard_url.unwrap_or("http://localhost:8000");
    let mut config = DefGuardConfig::new_test_config();
    config.url = Url::parse(url).unwrap();
    let _ = SERVER_CONFIG.set(config.clone());
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
