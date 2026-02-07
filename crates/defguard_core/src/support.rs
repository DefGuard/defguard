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
use sqlx::PgPool;

use crate::server_config;

/// Unwraps the result returning a JSON representation of value or error
fn unwrap_json<S: Serialize, D: Display>(result: Result<S, D>) -> Value {
    match result {
        Ok(value) => to_value(value).expect("conversion to JSON failed"),
        Err(err) => json!({"error": err.to_string()}),
    }
}

/// Dumps all data that could be used for debugging.
pub(crate) async fn dump_config(db: &PgPool) -> Value {
    // App settings DB records
    let settings = match Settings::get(db).await {
        Ok(Some(mut settings)) => {
            settings.smtp_password = None;
            json!(settings)
        }
        Ok(None) => json!({"error": "Settings not found"}),
        Err(err) => json!({"error": err.to_string()}),
    };
    // Networks
    let (networks, devices) = match WireguardNetwork::all(db).await {
        Ok(networks) => {
            // Devices for each network
            let mut devices = HashMap::<Id, Value>::new();
            for network in &networks {
                devices.insert(
                    network.id,
                    unwrap_json(WireguardNetworkDevice::all_for_network(db, network.id).await),
                );
            }
            (
                to_value(networks).expect("JSON serialization error"),
                to_value(devices).expect("JSON serialization error"),
            )
        }
        Err(err) => (json!({"error": err.to_string()}), Value::Null),
    };
    let users_diagnostic_data = unwrap_json(User::all_without_sensitive_data(db).await);

    let proxies = match Proxy::all(db).await {
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

    let gateways = match Gateway::all(db).await {
        Ok(gateways) => json!(
            gateways
                .iter()
                .map(|g| json!({
                    "id": g.id,
                    "network_id": g.network_id,
                    "version": g.version.as_deref().unwrap_or("unknown"),
                    "url": g.url,
                    "hostname": g.hostname,
                    "connected_at": g.connected_at,
                }))
                .collect::<Vec<_>>()
        ),
        Err(err) => json!({"error": err.to_string()}),
    };

    json!({
        "settings": settings,
        "networks": networks,
        "version": VERSION,
        "devices": devices,
        "users": users_diagnostic_data,
        "config": server_config(),
        "proxies": proxies,
        "gateways": gateways,
    })
}
