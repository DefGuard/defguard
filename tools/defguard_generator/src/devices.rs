use std::collections::HashSet;

use anyhow::{Context, Result};
use defguard_common::db::{
    Id,
    models::{Device, DeviceType, User},
};
use rand::{Rng, rngs::ThreadRng};
use sqlx::PgPool;
use tracing::info;

use crate::{
    user_devices::{prepare_user_devices, unique_device_name},
    users::prepare_users,
};

const NETWORK_DEVICE_NAMES: [&str; 8] = [
    "Router",
    "Switch",
    "Firewall",
    "Access Point",
    "NAS",
    "Printer",
    "Camera",
    "Server",
];

pub struct DevicesGeneratorConfig {
    pub num_users: usize,
    pub devices_per_user: usize,
    pub network_devices: usize,
}

/// Tops up the database to the requested counts, so repeated runs with the same arguments
/// don't add more rows.
pub async fn generate_devices(pool: PgPool, config: DevicesGeneratorConfig) -> Result<()> {
    let mut rng = rand::thread_rng();

    let users = prepare_users(&pool, &mut rng, config.num_users).await?;
    for (i, user) in users.iter().enumerate() {
        info!(
            "[{}/{}] Preparing {} devices for user {user}",
            i + 1,
            users.len(),
            config.devices_per_user
        );
        prepare_user_devices(&pool, &mut rng, user, config.devices_per_user).await?;
    }

    prepare_network_devices(&pool, &mut rng, config.network_devices).await?;

    info!("Finished generating devices");
    Ok(())
}

async fn prepare_network_devices(pool: &PgPool, rng: &mut ThreadRng, count: usize) -> Result<()> {
    let existing = Device::find_by_type(pool, DeviceType::Network).await?;
    if existing.len() >= count {
        info!(
            "Found {} existing network devices in the database, skipping",
            existing.len()
        );
        return Ok(());
    }

    let creator_id: Id = match User::find_admins(pool).await?.first() {
        Some(admin) => admin.id,
        None => {
            User::all(pool)
                .await?
                .first()
                .context("No user found to assign as the network devices' creator")?
                .id
        }
    };
    let missing = count - existing.len();
    let mut taken_names: HashSet<String> = existing.into_iter().map(|device| device.name).collect();
    info!("Creating {missing} network devices");

    for _ in 0..missing {
        let mut device: Device = rng.r#gen();
        let base_name = NETWORK_DEVICE_NAMES[rng.gen_range(0..NETWORK_DEVICE_NAMES.len())];
        device.name = unique_device_name(base_name, &mut taken_names);
        device.user_id = creator_id;
        device.device_type = DeviceType::Network;
        device.description = None;
        device.save(pool).await?;
    }

    Ok(())
}
