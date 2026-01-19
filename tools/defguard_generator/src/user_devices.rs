use anyhow::Result;
use defguard_common::db::{
    Id,
    models::{Device, User},
};
use sqlx::PgPool;

pub async fn prepare_user_devices(
    pool: &PgPool,
    user: &User<Id>,
    devices_per_user: u8,
) -> Result<Vec<Device<Id>>> {
    let user_devices = Device::all_for_username(pool, &user.username).await?;
    todo!()
}
