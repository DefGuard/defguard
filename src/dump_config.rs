use std::collections::HashMap;

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};

use crate::{
    config::DefGuardConfig,
    db::{models::device::WireguardNetworkDevice, Settings, WireguardNetwork, User},
    VERSION,
};

pub async fn dump_config(db: &Pool<Postgres>, config: &DefGuardConfig) -> Value {
    let settings = Settings::all(db).await.unwrap();
    let networks = WireguardNetwork::all(db).await.unwrap();
    let mut devices = HashMap::<i64, Vec<WireguardNetworkDevice>>::default();
    for network in &networks {
        let network_id = network.id.unwrap();
        devices.insert(
            network_id,
            WireguardNetworkDevice::all_for_network(db, network_id)
                .await
                .unwrap(),
        );
    }
    let users = User::all(db).await.unwrap();

    json!({
        "settings": settings,
        "networks": networks,
        "version": VERSION,
        "devices": devices,
        "users": users,
    })
}
