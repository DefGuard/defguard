use std::collections::HashSet;

use anyhow::Result;
use defguard_common::db::{Id, models::User};
use rand::{Rng, rngs::ThreadRng};
use sqlx::PgPool;
use tracing::info;

const FIRST_NAMES: &str = include_str!("../data/first_names.txt");
const LAST_NAMES: &str = include_str!("../data/last_names.txt");

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

    let first_names: Vec<&str> = FIRST_NAMES.lines().collect();
    let last_names: Vec<&str> = LAST_NAMES.lines().collect();
    let mut taken_usernames: HashSet<String> =
        all_users.iter().map(|user| user.username.clone()).collect();

    // if there are not enough users create new ones
    for _ in 0..(num_users - all_users.len()) {
        let mut user: User = rng.r#gen();
        let first_name = first_names[rng.gen_range(0..first_names.len())];
        let last_name = last_names[rng.gen_range(0..last_names.len())];
        user.username = unique_username(first_name, last_name, &mut taken_usernames);
        user.first_name = first_name.to_string();
        user.last_name = last_name.to_string();
        user.email = format!("{}@defguard.net", user.username);
        let user = user.save(pool).await?;
        all_users.push(user);
    }

    Ok(all_users)
}

fn unique_username(first_name: &str, last_name: &str, taken: &mut HashSet<String>) -> String {
    let base = format!("{first_name}.{last_name}").to_lowercase();
    let mut username = base.clone();
    let mut suffix = 1;
    while taken.contains(&username) {
        suffix += 1;
        username = format!("{base}{suffix}");
    }
    taken.insert(username.clone());
    username
}
