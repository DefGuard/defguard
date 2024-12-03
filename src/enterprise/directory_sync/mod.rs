use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use crate::db::{Group, Id, User};
use sqlx::error::Error as SqlxError;
use thiserror::Error;

use super::db::models::openid_provider::OpenIdProvider;

#[derive(Debug, Error)]
pub enum DirectorySyncError {
    #[error("Database error")]
    DbError(#[from] SqlxError),
    #[error("Access token has expired")]
    AccessTokenExpired,
    #[error("Failed to process request")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to build a JWT token")]
    JWTError(#[from] jsonwebtoken::errors::Error),
    #[error("The selected provider {0} is not supported for directory sync")]
    UnsupportedProvider(String),
    #[error("Directory sync not configured")]
    NotConfigured,
}

pub mod google;

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryGroup {
    pub id: String,
    pub name: String,
}

pub trait DirectorySync {
    fn get_groups(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryGroup>, DirectorySyncError>> + Send;

    fn get_user_groups(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, DirectorySyncError>> + Send;

    fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> impl std::future::Future<Output = Result<Vec<String>, DirectorySyncError>> + Send;

    fn prepare(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), DirectorySyncError>> + Send;
}

async fn make_groups(groups: &Vec<String>, pool: &PgPool) {
    for group in groups {
        if Group::find_by_name(pool, group).await.unwrap().is_none() {
            Group::new(group).save(pool).await.unwrap();
        }
    }
}

pub async fn sync_user_groups<T: DirectorySync>(
    directory_sync: &T,
    user: &User<Id>,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    info!("Syncing user groups for {}", user.email);
    let groups = directory_sync.get_user_groups(&user.email).await?;

    let mut transaction = pool.begin().await.unwrap();

    let current_groups = user.member_of(&mut *transaction).await.unwrap();
    let current_group_names: Vec<&str> = current_groups.iter().map(|g| g.name.as_str()).collect();

    make_groups(&groups, pool).await;

    for group in &groups {
        if !current_group_names.contains(&group.as_str()) {
            let group = Group::find_by_name(pool, group).await.unwrap().unwrap();
            user.add_to_group(&mut *transaction, &group).await.unwrap();
        }
    }

    for current_group in &current_groups {
        if !groups.contains(&current_group.name) {
            user.remove_from_group(&mut *transaction, current_group)
                .await
                .unwrap();
        }
    }

    transaction.commit().await.unwrap();

    Ok(())
}

async fn create_and_add_to_group(
    user: &User<Id>,
    group_name: &str,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    let group = if let Some(group) = Group::find_by_name(pool, group_name).await? {
        group
    } else {
        Group::new(group_name).save(pool).await?;
        Group::find_by_name(pool, group_name).await?.unwrap()
    };
    user.add_to_group(pool, &group).await?;
    Ok(())
}

pub async fn sync_all_users<T: DirectorySync>(
    directory_sync: &T,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    info!("Syncing all users' groups, this may take a while...");
    let groups = directory_sync.get_groups().await?;
    info!("Found {} groups to sync", groups.len());

    let mut user_group_map: HashMap<String, HashSet<&str>> = HashMap::new();
    for group in &groups {
        match directory_sync.get_group_members(group).await {
            Ok(members) => {
                for member in members {
                    if let Some(user) = User::find_by_email(pool, &member).await? {
                        user_group_map
                            .entry(user.email)
                            .or_default()
                            .insert(&group.name);
                    }
                }
            }
            Err(err) => {
                error!(
                    "Failed to get group members for group {}. Error details: {}",
                    group.name, err
                );
            }
        }
    }

    let mut transaction = pool.begin().await.unwrap();
    for (user, groups) in user_group_map.into_iter() {
        let Some(user) = User::find_by_email(pool, &user).await? else {
            debug!(
                "User {} not found in the database, skipping group sync",
                user
            );
            continue;
        };

        let current_groups = user.member_of_names(&mut *transaction).await?;
        for current_group in &current_groups {
            debug!(
                "Checking if user {} is still a member of group {}",
                user.email, current_group
            );
            if !groups.contains(current_group.as_str()) {
                let group = Group::find_by_name(pool, current_group).await?;
                if let Some(group) = group {
                    info!("Removing user {} from group {} as they are not a member of it in the directory", user.email, group.name);
                    user.remove_from_group(&mut *transaction, &group).await?;
                }
            }
        }

        for group in groups.iter() {
            create_and_add_to_group(&user, group, pool).await?;
        }
    }
    transaction.commit().await.unwrap();

    info!("Syncing all users' groups done.");
    Ok(())
}

async fn get_directory_sync(pool: &PgPool) -> Result<impl DirectorySync, DirectorySyncError> {
    let provider_settings = OpenIdProvider::get_current(pool).await?.unwrap();

    match provider_settings.name.as_str() {
        "Google" => {
            if provider_settings.google_service_account_key.is_none()
                || provider_settings.google_service_account_email.is_none()
                || provider_settings.admin_email.is_none()
            {
                return Err(DirectorySyncError::NotConfigured);
            }

            let client = google::GoogleDirectorySync::new(&provider_settings);
            println!("{:?}", client);
            Ok(client)
        }
        _ => Err(DirectorySyncError::UnsupportedProvider(
            provider_settings.name.clone(),
        )),
    }
}

pub async fn run_periodic_directory_sync(pool: &PgPool) -> Result<(), DirectorySyncError> {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

    loop {
        match get_directory_sync(pool).await {
            Ok(mut dir_sync) => {
                let _ = dir_sync.prepare().await;
                sync_all_users(&dir_sync, pool).await?;
            }
            Err(err) => {
                error!("Failed to get directory sync client: {}", err);
            }
        }

        interval.tick().await;
    }
}
