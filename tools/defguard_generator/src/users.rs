use anyhow::Result;
use defguard_common::db::{Id, models::User};
use rand::{Rng, rngs::ThreadRng};
use sqlx::PgPool;
use tracing::info;

pub async fn prepare_users(
    pool: &PgPool,
    rng: &mut ThreadRng,
    num_users: usize,
) -> Result<Vec<User<Id>>> {
    info!("Preparing {num_users} random users for generating VPN session stats");

    // fetch all existing users
    let mut all_users = User::all(pool).await?;

    // if there are enough users just return the required number
    if all_users.len() >= num_users {
        info!(
            "Found {} existing users in the database. Using the required number.",
            all_users.len()
        );
        return Ok(all_users[..num_users].to_vec());
    }

    // if there are not enough users create new ones
    for _ in 0..(num_users - all_users.len()) {
        let user: User = rng.r#gen();
        let user = user.save(pool).await?;
        all_users.push(user);
    }

    Ok(all_users)
}
