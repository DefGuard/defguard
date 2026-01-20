use anyhow::Result;
use defguard_common::db::{
    Id,
    models::{Device, User},
};
use rand::{Rng, rngs::ThreadRng};
use sqlx::PgPool;
use tracing::info;

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

    // if there are not enough users create new ones
    for _ in 0..(devices_per_user - user_devices.len()) {
        let mut device: Device = rng.r#gen();
        device.user_id = user.id;
        let device = device.save(pool).await?;
        user_devices.push(device);
    }

    Ok(user_devices)
}
