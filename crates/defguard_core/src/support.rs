use std::{collections::HashMap, fmt::Display};

use defguard_common::{
    VERSION,
    db::{
        Id,
        models::{
            Settings, User, WireguardNetwork, device::WireguardNetworkDevice, gateway::Gateway,
            proxy::Proxy,
        },
    },
};
use serde::Serialize;
use serde_json::{Value, json, value::to_value};
use sqlx::PgConnection;

use crate::server_config;

/// Unwraps the result returning a JSON representation of value or error
fn unwrap_json<S: Serialize, D: Display>(result: Result<S, D>) -> Result<Value, serde_json::Error> {
    Ok(match result {
        Ok(value) => to_value(value)?,
        Err(err) => json!({"error": err.to_string()}),
    })
}

/// Dumps all data that could be used for debugging.
pub(crate) async fn dump_config(conn: &mut PgConnection) -> Result<Value, serde_json::Error> {
    // App settings DB records
    let settings = match Settings::get(&mut *conn).await {
        Ok(Some(mut settings)) => {
            // Hide sensitive fields.
            settings.smtp.password = None;
            settings.smtp.oauth_client_secret = None;
            settings.smtp.oauth_refresh_token = None;
            settings.ldap_bind_password = None;
            settings.license = None;
            json!(settings)
        }
        Ok(None) => json!({"error": "Settings not found"}),
        Err(err) => json!({"error": err.to_string()}),
    };
    // Networks
    let (networks, devices) = match WireguardNetwork::all(&mut *conn).await {
        Ok(networks) => {
            // Devices for each network
            let mut devices = HashMap::<Id, Value>::new();
            for network in &networks {
                devices.insert(
                    network.id,
                    unwrap_json(
                        WireguardNetworkDevice::all_for_network(&mut *conn, network.id).await,
                    )?,
                );
            }
            (to_value(networks)?, to_value(devices)?)
        }
        Err(err) => (json!({"error": err.to_string()}), Value::Null),
    };
    let users_diagnostic_data = unwrap_json(User::all_without_sensitive_data(&mut *conn).await)?;

    let proxies = match Proxy::all(&mut *conn).await {
        Ok(proxies) => json!(
            proxies
                .iter()
                .map(|p| json!({
                    "id": p.id,
                    "name": p.name,
                    "version": p.version.as_deref().unwrap_or("unknown"),
                    "address": p.address,
                    "connected_at": p.connected_at
                }))
                .collect::<Vec<_>>()
        ),
        Err(err) => json!({"error": err.to_string()}),
    };

    let gateways = match Gateway::all(&mut *conn).await {
        Ok(gateways) => json!(
            gateways
                .iter()
                .map(|g| json!({
                    "id": g.id,
                    "network_id": g.location_id,
                    "version": g.version.as_deref().unwrap_or("unknown"),
                    "address": g.address,
                    "port": g.port,
                    "name": g.name,
                    "connected_at": g.connected_at,
                }))
                .collect::<Vec<_>>()
        ),
        Err(err) => json!({"error": err.to_string()}),
    };

    Ok(json!({
        "settings": settings,
        "networks": networks,
        "version": VERSION,
        "devices": devices,
        "users": users_diagnostic_data,
        "config": server_config(),
        "proxies": proxies,
        "gateways": gateways,
    }))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use defguard_common::{
        config::{DefGuardConfig, SERVER_CONFIG},
        db::{
            models::{
                Settings, User, WireguardNetwork,
                wireguard::{LocationMfaMode, ServiceLocationMode},
            },
            setup_pool,
        },
        secret::SecretStringWrapper,
    };
    use ipnetwork::IpNetwork;
    use secrecy::SecretString;
    use serde_json::Value;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::dump_config;

    // Sentinel secrets. None of these may ever show up in the support dump.
    const SMTP_PASSWORD: &str = "sentinel-smtp-password";
    const SMTP_OAUTH_CLIENT_SECRET: &str = "sentinel-smtp-oauth-client-secret";
    const SMTP_OAUTH_REFRESH_TOKEN: &str = "sentinel-smtp-oauth-refresh-token";
    const LDAP_BIND_PASSWORD: &str = "sentinel-ldap-bind-password";
    const LICENSE: &str = "sentinel-license-key";
    const SECRET_KEY: &str = "sentinel-secret-key-0123456789abcdef0123456789abcdef0123456789ab";
    const DATABASE_PASSWORD: &str = "sentinel-database-password";
    const NETWORK_PRVKEY: &str = "sentinel-network-private-key";
    const RECOVERY_CODE: &str = "sentinel-recovery-code";
    const USER_PASSWORD: &str = "sentinel-user-password";

    /// Substrings in a JSON key which mark its value as sensitive.
    const SENSITIVE_KEY_PARTS: &[&str] = &[
        "password",
        "secret",
        "token",
        "license",
        "prvkey",
        "private",
        "recovery_codes",
        "hash",
    ];

    /// Recursively assert that no key which looks sensitive carries a value.
    /// Numbers are ignored on purpose - names like `enrollment_token_timeout_hours`
    /// match the patterns above, but a timeout is not a secret.
    fn assert_sensitive_keys_empty(value: &Value, path: &str) {
        match value {
            Value::Object(map) => {
                for (key, val) in map {
                    let child_path = format!("{path}.{key}");
                    let lowercase_key = key.to_lowercase();
                    if SENSITIVE_KEY_PARTS
                        .iter()
                        .any(|part| lowercase_key.contains(part))
                    {
                        match val {
                            Value::String(string) => assert!(
                                string.is_empty(),
                                "{child_path} exposes a sensitive value: {string}"
                            ),
                            Value::Array(items) => assert!(
                                items.is_empty(),
                                "{child_path} exposes sensitive values: {val}"
                            ),
                            _ => (),
                        }
                    }
                    assert_sensitive_keys_empty(val, &child_path);
                }
            }
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    assert_sensitive_keys_empty(item, &format!("{path}[{index}]"));
                }
            }
            _ => (),
        }
    }

    #[sqlx::test]
    async fn dump_config_hides_sensitive_data(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        // Make sure the runtime config holds secrets, too. Another test in this binary may
        // have won the race for the `OnceLock`; in that case the config simply has no secrets.
        let mut config = DefGuardConfig::new_test_config();
        config.database_password = SecretString::from(DATABASE_PASSWORD.to_owned());
        #[expect(
            deprecated,
            reason = "the deprecated secret key must not leak either, so test it"
        )]
        {
            config.secret_key = Some(SecretString::from(SECRET_KEY.to_owned()));
        }
        let _ = SERVER_CONFIG.set(config);

        // Fill the settings with secrets.
        let mut settings = Settings::get(&pool)
            .await
            .expect("failed to fetch settings")
            .expect("settings should exist");
        settings.instance_name = "test instance".into();
        settings.smtp.password = Some(SecretStringWrapper::from(SMTP_PASSWORD.to_owned()));
        settings.smtp.oauth_client_secret = Some(SecretStringWrapper::from(
            SMTP_OAUTH_CLIENT_SECRET.to_owned(),
        ));
        settings.smtp.oauth_refresh_token = Some(SMTP_OAUTH_REFRESH_TOKEN.to_owned());
        settings.ldap_bind_password =
            Some(SecretStringWrapper::from(LDAP_BIND_PASSWORD.to_owned()));
        settings.license = Some(LICENSE.into());
        settings.set_secret_key(Some(SECRET_KEY.to_owned()));
        settings.save(&pool).await.expect("failed to save settings");

        // A location carries a WireGuard private key.
        let mut network = WireguardNetwork::new(
            "test location".to_owned(),
            50051,
            "10.1.1.1".to_owned(),
            None,
            vec![IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 0)), 24).unwrap()],
            true,
            false,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 1, 1, 1)), 24).unwrap()])
        .unwrap();
        NETWORK_PRVKEY.clone_into(&mut network.prvkey);
        network.save(&pool).await.expect("failed to save location");

        // A user carries a password hash and recovery codes.
        let mut user = User::new(
            "testuser",
            Some(USER_PASSWORD),
            "Test",
            "User",
            "test.user@example.com",
            None,
        );
        user.recovery_codes = vec![RECOVERY_CODE.to_owned()];
        let user = user.save(&pool).await.expect("failed to save user");
        let password_hash = user
            .password_hash
            .expect("user should have a password hash");

        let mut conn = pool.acquire().await.expect("failed to acquire connection");
        let dump = dump_config(&mut conn).await.expect("failed to dump config");

        // Sanity check: the dump is not empty, so the assertions below aren't vacuous.
        for key in [
            "settings", "networks", "version", "devices", "users", "config", "proxies", "gateways",
        ] {
            assert!(dump.get(key).is_some(), "dump is missing the {key} section");
        }
        assert_eq!(dump["settings"]["instance_name"], "test instance");
        assert_eq!(dump["networks"][0]["name"], "test location");
        assert_eq!(dump["users"].as_array().map(Vec::len), Some(1));

        let serialized = serde_json::to_string(&dump).expect("failed to serialize dump");
        for (name, secret) in [
            ("SMTP password", SMTP_PASSWORD),
            ("SMTP OAuth client secret", SMTP_OAUTH_CLIENT_SECRET),
            ("SMTP OAuth refresh token", SMTP_OAUTH_REFRESH_TOKEN),
            ("LDAP bind password", LDAP_BIND_PASSWORD),
            ("license", LICENSE),
            ("secret key", SECRET_KEY),
            ("database password", DATABASE_PASSWORD),
            ("location private key", NETWORK_PRVKEY),
            ("recovery code", RECOVERY_CODE),
            ("password hash", password_hash.as_str()),
        ] {
            assert!(
                !serialized.contains(secret),
                "support dump exposes the {name}"
            );
        }

        // Catch sensitive fields added in the future, even without a sentinel value.
        assert_sensitive_keys_empty(&dump, "dump");
    }
}
