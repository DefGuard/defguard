use std::{collections::HashMap, fmt::Display};

use serde::Serialize;
use serde_json::{json, value::to_value, Value};
use sqlx::{Pool, Postgres};

use crate::{
    mask,
    config::DefGuardConfig, 
    db::{models::device::WireguardNetworkDevice, Settings, User, WireguardNetwork},
    VERSION,
};

/// Unwraps the result returning a JSON representation of value or error
fn unwrap_json(result: Result<impl Serialize, impl Display>) -> Value {
    match result {
        Ok(value) => to_value(value).expect("conversion to JSON failed"),
        Err(err) => json!({"error": err.to_string()}),
    }
}

fn mask_fields(mut data: Value, fields_to_mask: &[&str]) -> Value {
    let objects = data.as_array_mut().unwrap();
    for obj in objects {
        for field in fields_to_mask {
            if let Some(field_value) = obj.get_mut(*field) {
                *field_value = json!("*");
            }
        }
    }

    data
}

/// Dumps all data that could be used for debugging.
pub async fn dump_config(db: &Pool<Postgres>,  config: &DefGuardConfig) -> Value {
    // App settings DB records
    let settings = unwrap_json(Settings::all(db).await);
    // Networks
    let (networks, devices) = match WireguardNetwork::all(db).await {
        Ok(networks) => {
            // Devices for each network
            let mut devices = HashMap::<i64, Value>::default();
            for network in &networks {
                let network_id = match network.id {
                    Some(id) => id,
                    None => continue,
                };
                devices.insert(
                    network_id,
                    unwrap_json(WireguardNetworkDevice::all_for_network(db, network_id).await),
                );
            }
            (
                to_value(networks).expect("JSON serialization error"),
                to_value(devices).expect("JSON serialization error"),
            )
        }
        Err(err) => (json!({"error": err.to_string()}), Value::Null),
    };
    let users = unwrap_json(User::all_without_sensitive_data(db).await);
    let settings = mask_fields(settings, &["smtp_password"]);
    let config = mask!(config, secret_key, database_password, ldap_bind_password, default_admin_password);

    json!({
        "settings": settings,
        "networks": networks,
        "version": VERSION,
        "devices": devices,
        "users": users,
        "config": config,
    })
}
