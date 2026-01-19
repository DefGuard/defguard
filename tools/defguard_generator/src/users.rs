use anyhow::Result;
use defguard_common::db::{Id, models::User};
use sqlx::PgPool;
use tracing::info;

pub async fn prepare_users(pool: &PgPool, num_users: usize) -> Result<Vec<User<Id>>> {
    info!("Preparing {num_users} random users for generating VPN session stats");

    // fetch all existing users
    let all_users = User::all(pool).await?;

    // if there are enough users just return the required number
    if all_users.len() >= num_users {
        info!(
            "Found {} existing users in the database. Using the required number.",
            all_users.len()
        );
        return Ok(all_users[..(num_users as usize)].to_vec());
    }
    //
    // if there are not enough users create new ones
    Ok(todo!())
}
