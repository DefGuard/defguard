use std::{collections::HashMap, fmt::Display};

use serde::Serialize;
use serde_json::{json, value::to_value, Value};
use sqlx::{Pool, Postgres};

use crate::{
    config::DefGuardConfig,
    db::{models::device::WireguardNetworkDevice, Settings, User, WireguardNetwork},
    VERSION,
};

/// Unwraps the result returning a json representation of value or error
fn unwrap_json(result: Result<impl Serialize, impl Display>) -> Value {
    match result {
        Ok(value) => to_value(value).expect("conversion to json failed"),
        Err(err) => json!({"error": err.to_string()}),
    }
}

/// Dumps all data that could be used for debugging.
pub async fn dump_config(db: &Pool<Postgres>, config: &DefGuardConfig) -> Value {
    // App settings DB records
    let settings = unwrap_json(Settings::all(db).await);
    // Networks
    let (networks, devices) = match WireguardNetwork::all(db).await {
        Ok(networks) => {
            // Devices for each network
            let mut devices = HashMap::<i64, Value>::default();
            for network in &networks {
                let network_id = network.id.unwrap();
                devices.insert(
                    network_id,
                    unwrap_json(WireguardNetworkDevice::all_for_network(db, network_id).await),
                );
            }
            (
                to_value(networks).expect("json serialization error"),
                to_value(devices).expect("json serialization error"),
            )
        }
        Err(err) => (json!({"error": err.to_string()}), Value::Null),
    };
    let users = unwrap_json(User::all(db).await);

    json!({
        "settings": settings,
        "networks": networks,
        "version": VERSION,
        "devices": devices,
        "users": users,
        "config": config,
    })
}
