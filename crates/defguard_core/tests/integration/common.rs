use defguard_common::{
    config::{DefGuardConfig, SERVER_CONFIG},
    db::models::{
        Settings, User,
        settings::{initialize_current_settings, update_current_settings},
    },
};
use defguard_core::enterprise::license::{License, LicenseTier, set_cached_license};
use secrecy::ExposeSecret;
use sqlx::PgPool;

fn set_test_license_business() {
    let license = License {
        customer_id: "0c4dcb5400544d47ad8617fcdf2704cb".into(),
        limits: None,
        subscription: false,
        tier: LicenseTier::Business,
        valid_until: None,
        version_date_limit: None,
    };
    set_cached_license(Some(license));
}

/// Allows overriding the default DefGuard URL for tests, as during the tests, the server has a random port, making the URL unpredictable beforehand.
// TODO: Allow customizing the whole config, not just the URL
pub(crate) async fn init_config(
    custom_defguard_url: Option<&str>,
    pool: &PgPool,
) -> DefGuardConfig {
    let url = custom_defguard_url.unwrap_or("http://localhost:8000");
    let mut config = DefGuardConfig::new_test_config();
    initialize_current_settings(pool)
        .await
        .expect("Could not initialize current settings in the database");
    let mut settings = Settings::get_current_settings();
    settings.defguard_url = url.to_string();
    update_current_settings(pool, settings)
        .await
        .expect("Could not update current settings in the database");
    set_test_license_business();

    config.initialize_post_settings();
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
