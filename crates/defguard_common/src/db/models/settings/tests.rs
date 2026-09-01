use std::str::FromStr;

use humantime::Duration;
use reqwest::Url;
use rsa::{RsaPrivateKey, pkcs8::EncodePrivateKey};
use secrecy::SecretString;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::db::setup_pool;

#[test]
fn test_smtp_config() {
    let mut settings = Settings::default();
    assert!(!settings.smtp_configured());

    // incomplete SMTP config
    settings.smtp.server = Some("localhost".into());
    settings.smtp.port = Some(587);
    assert!(!settings.smtp_configured());

    // no-auth SMTP config
    settings.smtp.sender = Some("no-reply@defguard.net".into());
    assert!(settings.smtp_configured());

    // add non-default encryption
    settings.smtp.encryption = SmtpEncryption::StartTls;
    assert!(settings.smtp_configured());

    // add auth info
    settings.smtp.user = Some("smtp_user".into());
    settings.smtp.password = Some(SecretStringWrapper::from_str("hunter2").unwrap());
    assert!(settings.smtp_configured());
}

#[test]
fn dg25_32_test_dont_expose_license_key() {
    let key = "0000000000000000";
    let settings = Settings {
        license: Some(key.to_owned()),
        ..Default::default()
    };

    let debug = format!("{settings:?}");
    assert!(!debug.contains("license"));
    assert!(!debug.contains(key));
}

#[test]
fn test_callback_url() {
    let mut s = Settings {
        defguard_url: "https://defguard.example.com".into(),
        ..Default::default()
    };
    assert_eq!(
        s.callback_url().unwrap().as_str(),
        "https://defguard.example.com/auth/callback"
    );

    s.defguard_url = "https://defguard.example.com:8443/path".into();
    assert_eq!(
        s.callback_url().unwrap().as_str(),
        "https://defguard.example.com:8443/path/auth/callback"
    );
}

#[test]
#[allow(deprecated)]
fn test_apply_from_config_maps_migrated_fields() {
    let mut settings = Settings {
        defguard_url: "https://defguard.example.com".into(),
        ..Default::default()
    };
    let mut config = DefGuardConfig::new_test_config();

    config.secret_key = Some(SecretString::from("a".repeat(64)));
    config.enrollment_url = Some(Url::parse("https://proxy.example.com").unwrap());
    config.mfa_code_timeout = Some(Duration::from(std::time::Duration::from_secs(75)));
    config.session_timeout = Some(Duration::from(std::time::Duration::from_hours(240)));
    config.disable_stats_purge = Some(true);
    config.stats_purge_frequency = Some(Duration::from(std::time::Duration::from_hours(5)));
    config.stats_purge_threshold = Some(Duration::from(std::time::Duration::from_hours(288)));
    config.enrollment_token_timeout = Some(Duration::from(std::time::Duration::from_hours(7)));
    config.password_reset_token_timeout = Some(Duration::from(std::time::Duration::from_hours(9)));
    config.enrollment_session_timeout = Some(Duration::from(std::time::Duration::from_mins(15)));
    config.password_reset_session_timeout =
        Some(Duration::from(std::time::Duration::from_mins(20)));

    settings.apply_from_config(&config);

    assert_eq!(
        settings.secret_key(),
        Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    );
    assert_eq!(settings.webauthn_rp_id().unwrap(), "defguard.example.com");
    assert_eq!(settings.public_proxy_url, "https://proxy.example.com/");
    assert_eq!(settings.mfa_code_timeout_seconds, 75);
    assert_eq!(settings.authentication_period_days, 10);
    assert!(!settings.enable_stats_purge);
    assert_eq!(settings.stats_purge_frequency_hours, 5);
    assert_eq!(settings.stats_purge_threshold_days, 12);
    assert_eq!(settings.enrollment_token_timeout_hours, 7);
    assert_eq!(settings.password_reset_token_timeout_hours, 9);
    assert_eq!(settings.enrollment_session_timeout_minutes, 15);
    assert_eq!(settings.password_reset_session_timeout_minutes, 20);
}

#[test]
fn test_apply_from_config_keeps_values_when_config_is_none() {
    let mut settings = Settings {
        defguard_url: "https://defguard.example.com".into(),
        secret_key: Some(SecretStringWrapper::from("z".repeat(SECRET_KEY_MIN_LEN))),
        public_proxy_url: "https://proxy.initial".into(),
        mfa_code_timeout_seconds: 123,
        authentication_period_days: 9,
        enable_stats_purge: false,
        ..Default::default()
    };
    let config = DefGuardConfig::new_test_config();
    let existing_secret = "z".repeat(SECRET_KEY_MIN_LEN);

    settings.apply_from_config(&config);

    assert_eq!(settings.secret_key(), Some(existing_secret.as_str()));
    assert_eq!(settings.webauthn_rp_id().unwrap(), "defguard.example.com");
    assert_eq!(settings.public_proxy_url, "https://proxy.initial");
    assert_eq!(settings.mfa_code_timeout_seconds, 123);
    assert_eq!(settings.authentication_period_days, 9);
    assert!(!settings.enable_stats_purge);
}

#[test]
fn test_webauthn_rp_id_rejects_invalid_defguard_url() {
    let mut settings = Settings {
        defguard_url: "this is not an url".into(),
        ..Default::default()
    };
    let config = DefGuardConfig::new_test_config();

    settings.apply_from_config(&config);

    assert!(matches!(
        settings.webauthn_rp_id(),
        Err(SettingsUrlError::InvalidDefguardUrl(_))
    ));
}

#[test]
fn test_parse_defguard_url_parses_valid_hostname_url() {
    let settings = Settings {
        defguard_url: "https://defguard.example.com:8443/path".into(),
        ..Default::default()
    };

    let url = settings.parse_defguard_url().unwrap();

    assert_eq!(url.host_str(), Some("defguard.example.com"));
    assert_eq!(url.port(), Some(8443));
    assert_eq!(url.path(), "/path");
}

#[test]
fn test_parse_defguard_url_rejects_ip_host() {
    let settings = Settings {
        defguard_url: "http://127.0.0.1:8000".into(),
        ..Default::default()
    };

    assert!(matches!(
        settings.parse_defguard_url(),
        Err(SettingsUrlError::DefguardUrlUsesIpAddress(_))
    ));
}

#[test]
fn test_cookie_domain_derives_from_defguard_url() {
    let settings = Settings {
        defguard_url: "https://defguard.example.com:8443/path".into(),
        ..Default::default()
    };

    assert_eq!(settings.cookie_domain().unwrap(), "defguard.example.com");
}

#[test]
fn test_cookie_domain_allows_localhost() {
    let settings = Settings {
        defguard_url: "http://localhost:8000".into(),
        ..Default::default()
    };

    assert_eq!(settings.cookie_domain().unwrap(), "localhost");
}

#[test]
fn test_cookie_domain_rejects_ip_hosts() {
    let settings = Settings {
        defguard_url: "http://127.0.0.1:8000".into(),
        ..Default::default()
    };

    assert!(matches!(
        settings.cookie_domain(),
        Err(SettingsUrlError::DefguardUrlUsesIpAddress(_))
    ));
}

#[test]
fn test_configured_public_proxy_url_returns_configured_url() {
    let settings = Settings {
        public_proxy_url: "https://edge.example.com".into(),
        ..Default::default()
    };

    assert_eq!(
        settings.configured_public_proxy_url().as_deref(),
        Some("https://edge.example.com")
    );
}

#[test]
fn test_configured_public_proxy_url_returns_none_for_empty_url() {
    let settings = Settings {
        public_proxy_url: String::new(),
        ..Default::default()
    };

    assert_eq!(settings.configured_public_proxy_url(), None);
}

#[test]
fn test_public_settings_changed_returns_false_for_identical_settings() {
    let settings = Settings::default();

    assert!(!settings.edge_public_settings_changed(&settings));
}

#[test]
fn test_public_settings_changed_returns_false_for_unrelated_change() {
    let before = Settings::default();
    let mut after = before.clone();
    after.instance_name = "Changed name".into();

    assert!(!before.edge_public_settings_changed(&after));
}

#[test]
fn test_public_settings_changed_returns_true_for_smtp_state_change() {
    let before = Settings::default();
    let mut after = before.clone();
    after.smtp.server = Some("smtp.example.com".into());
    after.smtp.port = Some(587);
    after.smtp.sender = Some("noreply@example.com".into());

    assert!(!before.smtp_configured());
    assert!(after.smtp_configured());
    assert!(before.edge_public_settings_changed(&after));
}

#[test]
fn test_public_settings_changed_returns_true_for_public_proxy_url_change() {
    let before = Settings {
        public_proxy_url: "https://old.example.com".into(),
        ..Default::default()
    };
    let mut after = before.clone();
    after.public_proxy_url = "http://new.example.com".into();

    assert!(before.edge_public_settings_changed(&after));
}

// Regression tests for cookie_secure(): the secure flag on session/auth cookies
// must be derived from the defguard_url scheme when cookie_insecure is not set.

#[test]
fn test_cookie_secure_returns_true_for_https_url() {
    let settings = Settings {
        defguard_url: "https://defguard.example.com".into(),
        ..Default::default()
    };

    assert!(settings.cookie_secure().unwrap());
}

#[test]
fn test_cookie_secure_returns_false_for_http_url() {
    let settings = Settings {
        defguard_url: "http://defguard.example.com".into(),
        ..Default::default()
    };

    assert!(!settings.cookie_secure().unwrap());
}

#[test]
fn test_cookie_secure_returns_false_for_http_localhost() {
    let settings = Settings {
        defguard_url: "http://localhost:8000".into(),
        ..Default::default()
    };

    assert!(!settings.cookie_secure().unwrap());
}

#[test]
fn test_cookie_secure_returns_true_for_https_with_port_and_path() {
    let settings = Settings {
        defguard_url: "https://defguard.example.com:8443/path".into(),
        ..Default::default()
    };

    assert!(settings.cookie_secure().unwrap());
}

#[test]
fn test_cookie_secure_propagates_ip_address_error() {
    let settings = Settings {
        defguard_url: "https://127.0.0.1:8443".into(),
        ..Default::default()
    };

    assert!(matches!(
        settings.cookie_secure(),
        Err(SettingsUrlError::DefguardUrlUsesIpAddress(_))
    ));
}

#[test]
fn test_cookie_secure_propagates_invalid_url_error() {
    let settings = Settings {
        defguard_url: "not a url".into(),
        ..Default::default()
    };

    assert!(matches!(
        settings.cookie_secure(),
        Err(SettingsUrlError::InvalidDefguardUrl(_))
    ));
}

#[test]
fn test_validate_accepts_valid_hostname() {
    let mut settings = Settings {
        defguard_url: "https://defguard.example.com".into(),
        ..Default::default()
    };

    assert!(settings.validate().is_ok());
}

#[test]
fn test_validate_rejects_invalid_url() {
    let mut settings = Settings {
        defguard_url: "not a url".into(),
        ..Default::default()
    };

    assert!(matches!(
        settings.validate(),
        Err(SettingsValidationError::InvalidDefguardUrl(_))
    ));
}

/// Regression test for https://github.com/DefGuard/defguard/issues/3394
///
/// Disabling LDAP remote enrollment while the dependent "send invite" option
/// is still set must not fail validation. The value is left untouched - the
/// email-sending path guards on both flags, so no invite is sent regardless.
#[test]
fn test_validate_accepts_send_invite_when_remote_enrollment_disabled() {
    let mut settings = Settings {
        defguard_url: "https://defguard.example.com".into(),
        ldap_remote_enrollment_enabled: false,
        ldap_remote_enrollment_send_invite: true,
        ..Default::default()
    };

    assert!(
        settings.validate().is_ok(),
        "disabling remote enrollment must not fail validation when send invite is still set"
    );
    assert!(settings.ldap_remote_enrollment_send_invite);
}

#[test]
#[allow(deprecated)]
fn test_apply_from_config_invalid_secret_key_generates_new() {
    let mut settings = Settings::default();
    let mut config = DefGuardConfig::new_test_config();
    config.secret_key = Some(SecretString::from(" short ".to_owned()));

    settings.apply_from_config(&config);

    let generated = settings
        .secret_key()
        .expect("secret key should be generated");
    assert_eq!(generated.len(), SECRET_KEY_MIN_LEN);
    assert_ne!(generated, " short ");
    assert!(settings.validate_secret_key().is_ok());
}

#[test]
#[allow(deprecated)]
fn test_apply_from_config_valid_secret_key_is_used() {
    let mut settings = Settings::default();
    let mut config = DefGuardConfig::new_test_config();
    let valid_secret = "b".repeat(64);
    config.secret_key = Some(SecretString::from(valid_secret.clone()));

    settings.apply_from_config(&config);

    assert_eq!(settings.secret_key(), Some(valid_secret.as_str()));
}

#[test]
#[allow(deprecated)]
fn test_apply_from_config_valid_openid_signing_key_overwrites_existing_value() {
    let mut settings = Settings {
        openid_signing_key_der: Some(Settings::generate_openid_signing_key_der().unwrap()),
        ..Default::default()
    };
    let mut config = DefGuardConfig::new_test_config();
    let configured_key = RsaPrivateKey::new(&mut OsRng, OPENID_KEY_SIZE).unwrap();
    let expected_der = configured_key.to_pkcs8_der().unwrap().as_bytes().to_vec();
    config.openid_signing_key = Some(configured_key);

    settings.apply_from_config(&config);

    assert_eq!(settings.openid_signing_key_der, Some(expected_der));
}

#[test]
fn test_apply_from_config_keeps_openid_signing_key_when_config_is_none() {
    let existing = Settings::generate_openid_signing_key_der().unwrap();
    let mut settings = Settings {
        openid_signing_key_der: Some(existing.clone()),
        ..Default::default()
    };
    let config = DefGuardConfig::new_test_config();

    settings.apply_from_config(&config);

    assert_eq!(settings.openid_signing_key_der, Some(existing));
}

#[test]
fn test_openid_key_required_rejects_missing_key() {
    let settings = Settings::default();

    assert!(matches!(
        settings.openid_key_required(),
        Err(SettingsInitializationError::Missing(
            "openid_signing_key_der"
        ))
    ));
}

#[test]
fn test_openid_key_required_rejects_invalid_der() {
    let settings = Settings {
        openid_signing_key_der: Some(vec![1, 2, 3]),
        ..Default::default()
    };

    assert!(matches!(
        settings.openid_key_required(),
        Err(SettingsInitializationError::Invalid(
            "openid_signing_key_der",
            _
        ))
    ));
}

#[test]
fn test_openid_key_required_accepts_valid_der() {
    let settings = Settings {
        openid_signing_key_der: Some(Settings::generate_openid_signing_key_der().unwrap()),
        ..Default::default()
    };

    assert!(settings.openid_key_required().is_ok());
}

#[sqlx::test]
#[allow(deprecated)]
async fn test_update_from_config_persists_and_updates_current_settings(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();

    let mut settings = Settings::get_current_settings();
    settings.defguard_url = "https://defguard.example.com".into();
    update_current_settings(&pool, settings.clone())
        .await
        .unwrap();

    let mut config = DefGuardConfig::new_test_config();
    config.mfa_code_timeout = Some(Duration::from(std::time::Duration::from_secs(90)));
    config.session_timeout = Some(Duration::from(std::time::Duration::from_hours(48)));
    config.disable_stats_purge = Some(true);

    settings.update_from_config(&pool, &config).await.unwrap();

    let current = Settings::get_current_settings();
    let from_db = Settings::get(&pool).await.unwrap().unwrap();

    assert_eq!(current.mfa_code_timeout_seconds, 90);
    assert_eq!(current.authentication_period_days, 2);
    assert!(!current.enable_stats_purge);

    assert_eq!(from_db.mfa_code_timeout_seconds, 90);
    assert_eq!(from_db.authentication_period_days, 2);
    assert!(!from_db.enable_stats_purge);
}

#[sqlx::test]
async fn test_initialize_runtime_defaults_keeps_valid_defguard_url(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();

    let mut settings = Settings::get_current_settings();
    settings.defguard_url = "https://defguard.example.com:8443/path".into();
    settings.secret_key = Some(SecretStringWrapper::from("a".repeat(SECRET_KEY_MIN_LEN)));
    update_current_settings(&pool, settings).await.unwrap();

    Settings::initialize_runtime_defaults(&pool).await.unwrap();

    let current = Settings::get_current_settings();
    let from_db = Settings::get(&pool).await.unwrap().unwrap();

    assert_eq!(current.webauthn_rp_id().unwrap(), "defguard.example.com");
    assert_eq!(from_db.webauthn_rp_id().unwrap(), "defguard.example.com");
}

#[sqlx::test]
async fn test_initialize_runtime_defaults_generates_openid_signing_key(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool).await.unwrap();

    Settings::initialize_runtime_defaults(&pool).await.unwrap();

    let current = Settings::get_current_settings();
    let from_db = Settings::get(&pool).await.unwrap().unwrap();

    let current_key = current
        .openid_signing_key_der
        .as_deref()
        .expect("current settings should contain OpenID signing key");
    let db_key = from_db
        .openid_signing_key_der
        .as_deref()
        .expect("database settings should contain OpenID signing key");

    assert!(RsaPrivateKey::from_pkcs8_der(current_key).is_ok());
    assert!(RsaPrivateKey::from_pkcs8_der(db_key).is_ok());
}

#[test]
fn test_edge_callback_url() {
    let mut s = Settings {
        public_proxy_url: "https://edge.example.com".into(),
        ..Default::default()
    };

    assert_eq!(
        s.edge_callback_url(AuthFlowType::Enrollment)
            .unwrap()
            .as_str(),
        "https://edge.example.com/openid/callback"
    );
    assert_eq!(
        s.edge_callback_url(AuthFlowType::Mfa).unwrap().as_str(),
        "https://edge.example.com/openid/mfa/callback"
    );

    s.public_proxy_url = "https://edge.example.com:8443/path".into();
    assert_eq!(
        s.edge_callback_url(AuthFlowType::Enrollment)
            .unwrap()
            .as_str(),
        "https://edge.example.com:8443/path/openid/callback"
    );
    assert_eq!(
        s.edge_callback_url(AuthFlowType::Mfa).unwrap().as_str(),
        "https://edge.example.com:8443/path/openid/mfa/callback"
    );
}
