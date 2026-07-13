use std::collections::HashSet;

use anyhow::Result;
use defguard_common::db::{
    Id,
    models::{Device, DeviceType, User},
};
use rand::{Rng, rngs::ThreadRng};
use sqlx::PgPool;
use tracing::info;

const DEVICE_NAMES: &str = include_str!("../data/device_names.txt");

pub async fn prepare_user_devices(
    pool: &PgPool,
    rng: &mut ThreadRng,
    user: &User<Id>,
    devices_per_user: usize,
) -> Result<Vec<Device<Id>>> {
    // fetch all existing devices for a given user
    let mut user_devices = Device::all_for_username(pool, &user.username).await?;

    // if there are enough users just return the required number
    if user_devices.len() >= devices_per_user {
        info!(
            "Found {} existing devices for user {user} in the database. Using the required number.",
            user_devices.len()
        );
        return Ok(user_devices[..devices_per_user].to_vec());
    }
    let device_names: Vec<&str> = DEVICE_NAMES.lines().collect();
    let mut taken_names: HashSet<String> = user_devices
        .iter()
        .map(|device| device.name.clone())
        .collect();

    // if there are not enough users create new ones
    for _ in 0..(devices_per_user - user_devices.len()) {
        let mut device: Device = rng.r#gen();
        let base_name = device_names[rng.gen_range(0..device_names.len())];
        device.name = unique_device_name(base_name, &mut taken_names);
        device.user_id = user.id;
        device.device_type = DeviceType::User;
        device.description = None;
        let device = device.save(pool).await?;
        user_devices.push(device);
    }

    Ok(user_devices)
}

fn unique_device_name(base: &str, taken: &mut HashSet<String>) -> String {
    let mut name = base.to_string();
    let mut suffix = 1;
    while taken.contains(&name) {
        suffix += 1;
        name = format!("{base}{suffix}");
    }
    taken.insert(name.clone());
    name
}
