use std::collections::{HashMap, HashSet};

use sqlx::PgPool;
use tokio::time::sleep;

use crate::{
    db::{Group, Id, User},
    enterprise::db::models::openid_provider::DirectorySyncUserBehavior,
};
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
    #[error("Defguard group not found: {0}")]
    DefGuardGroupNotFound(String),
}

pub mod google;

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryGroup {
    pub id: String,
    pub name: String,
}

pub enum DirectorySyncProvider {
    Google,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryUser {
    pub email: String,
    // Users may be disabled/suspended in the directory
    pub active: bool,
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

    fn get_provider_type(&self) -> DirectorySyncProvider;

    fn get_all_users(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryUser>, DirectorySyncError>> + Send;
}

async fn make_groups(groups: &Vec<String>, pool: &PgPool) -> Result<(), SqlxError> {
    debug!("Making groups, if they don't exist");
    for group in groups {
        if Group::find_by_name(pool, group).await?.is_none() {
            debug!("Creating group {}", group);
            Group::new(group).save(pool).await?;
            debug!("Group {} created", group);
        }
    }
    debug!("All groups created, if they didn't exist");

    Ok(())
}

pub(crate) async fn sync_user_groups<T: DirectorySync>(
    directory_sync: &T,
    user: &User<Id>,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    info!("Syncing groups of user {}", user.email);
    let groups = directory_sync.get_user_groups(&user.email).await?;

    let mut transaction = pool.begin().await?;

    let current_groups = user.member_of(&mut *transaction).await?;
    let current_group_names: Vec<&str> = current_groups.iter().map(|g| g.name.as_str()).collect();

    make_groups(&groups, pool).await?;

    for group in &groups {
        if !current_group_names.contains(&group.as_str()) {
            let group = Group::find_by_name(pool, group)
                .await?
                .ok_or(DirectorySyncError::DefGuardGroupNotFound(group.to_string()))?;

            user.add_to_group(&mut *transaction, &group).await?;
        }
    }

    for current_group in &current_groups {
        if !groups.contains(&current_group.name) {
            user.remove_from_group(&mut *transaction, current_group)
                .await?;
        }
    }

    transaction.commit().await?;

    Ok(())
}

async fn create_and_add_to_group(
    user: &User<Id>,
    group_name: &str,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    debug!(
        "Creating group {} if it doesn't exist and adding user {} to it if they are not already a member",
        user.email, group_name
    );
    let group = if let Some(group) = Group::find_by_name(pool, group_name).await? {
        debug!("Group {} already exists", group_name);
        group
    } else {
        debug!("Group {} didn't exist, creating it now", group_name);
        let new_group = Group::new(group_name).save(pool).await?;
        debug!("Group {} created", group_name);
        new_group
    };

    debug!(
        "Adding user {} to group {} if they are not already a member",
        user.email, group_name
    );
    user.add_to_group(pool, &group).await?;
    debug!(
        "User {} was added to group {} if they weren't already a member",
        user.email, group_name
    );
    Ok(())
}

/// Sync all users' groups with the directory
pub(crate) async fn sync_all_users_groups<T: DirectorySync>(
    directory_sync: &T,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    info!("Syncing all users' groups, this may take a while...");
    let groups = directory_sync.get_groups().await?;
    debug!("Found {} groups to sync", groups.len());

    // Create a map of user: group to apply later
    // It will be used to decide what groups should be removed from the user and what should be added
    let mut user_group_map: HashMap<String, HashSet<&str>> = HashMap::new();
    for group in &groups {
        match directory_sync.get_group_members(group).await {
            Ok(members) => {
                println!("GROUP: {:?}, MEMBERS: {:?}", group, members);
                for member in members {
                    // insert every user for now, we will filter non-existing users later
                    user_group_map
                        .entry(member)
                        .or_default()
                        .insert(&group.name);
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
                    debug!("Removing user {} from group {} as they are not a member of it in the directory", user.email, group.name);
                    user.remove_from_group(&mut *transaction, &group).await?;
                } else {
                    warn!(
                        "Group {} not found in the database, skipping removing it from user {}",
                        current_group, user.email
                    );
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
    let provider_settings = OpenIdProvider::get_current(pool)
        .await?
        .ok_or(DirectorySyncError::NotConfigured)?;

    match provider_settings.name.as_str() {
        "Google" => {
            match (
                provider_settings.google_service_account_key.as_ref(),
                provider_settings.google_service_account_email.as_ref(),
                provider_settings.admin_email.as_ref(),
            ) {
                (Some(key), Some(email), Some(admin_email)) => {
                    debug!("Google directory sync is configured");
                    let client = google::GoogleDirectorySync::new(key, email, admin_email);
                    Ok(client)
                }
                _ => Err(DirectorySyncError::NotConfigured),
            }
        }
        _ => Err(DirectorySyncError::UnsupportedProvider(
            provider_settings.name.clone(),
        )),
    }
}

async fn is_directory_sync_enabled(pool: &PgPool) -> Result<bool, DirectorySyncError> {
    debug!("Checking if directory sync is enabled");
    if let Some(provider_settings) = OpenIdProvider::get_current(pool).await? {
        debug!(
            "Directory sync enabled state: {}",
            provider_settings.directory_sync_enabled
        );
        Ok(provider_settings.directory_sync_enabled)
    } else {
        debug!("No openid provider found, directory sync is disabled");
        Ok(false)
    }
}

async fn sync_all_users_state<T: DirectorySync>(
    directory_sync: &T,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    let all_users = directory_sync.get_all_users().await?;
    let settings = OpenIdProvider::get_current(pool)
        .await?
        .ok_or(DirectorySyncError::NotConfigured)?;

    let user_behavior = settings.directory_sync_user_behavior;
    let admin_behavior = settings.directory_sync_admin_behavior;

    let emails = all_users
        .iter()
        // We want to filter out the main admin user, as he shouldn't be deleted
        .map(|u| u.email.as_str())
        .collect::<Vec<&str>>();

    let missing_users = User::exclude(pool, &emails)
        .await?
        .into_iter()
        // We don't want to disable the main admin user
        .filter(|u| u.email != "admin@defguard")
        .collect::<Vec<User<Id>>>();
    let disabled_users_emails = all_users
        .iter()
        .filter(|u| !u.active)
        .map(|u| u.email.as_str())
        .collect::<Vec<&str>>();
    let users_to_disable = User::find_many_by_emails(pool, &disabled_users_emails).await?;

    debug!(
        "There are {} disabled users in the directory, disabling them in Defguard...",
        users_to_disable.len()
    );

    for mut user in users_to_disable {
        debug!(
            "Disabling user {} because they are disabled in the directory",
            user.email
        );
        user.is_active = false;
        user.save(pool).await?;
    }

    debug!(
        "There are {} users missing from the directory but present in Defguard, \
    deciding what to do next based on the following settings: user action: {}, admin action: {}",
        missing_users.len(),
        user_behavior,
        admin_behavior
    );
    for mut user in missing_users {
        match user.is_admin(pool).await? {
            true => match admin_behavior {
                DirectorySyncUserBehavior::Keep => {
                    debug!(
                        "Keeping admin {} despite not being in the directory",
                        user.email
                    );
                }
                DirectorySyncUserBehavior::Disable => {
                    debug!(
                        "Disabling admin {} because they are not in the directory",
                        user.email
                    );
                    user.is_active = false;
                    user.save(pool).await?;
                }
                DirectorySyncUserBehavior::Delete => {
                    debug!(
                        "Deleting admin {} because they are not in the directory",
                        user.email
                    );
                    user.delete(pool).await?;
                }
            },
            false => match user_behavior {
                DirectorySyncUserBehavior::Keep => {
                    debug!(
                        "Keeping user {} despite not being in the directory",
                        user.email
                    );
                }
                DirectorySyncUserBehavior::Disable => {
                    debug!(
                        "Disabling user {} because they are not in the directory",
                        user.email
                    );
                    user.is_active = false;
                    user.save(pool).await?;
                }
                DirectorySyncUserBehavior::Delete => {
                    debug!(
                        "Deleting user {} because they are not in the directory",
                        user.email
                    );
                    user.delete(pool).await?;
                }
            },
        }
    }
    debug!("Done processing missing users");
    Ok(())
}

// The default inverval for the directory sync job
const DIRECTORY_SYNC_INTERVAL: u64 = 60 * 10;

pub(crate) async fn get_directory_sync_interval(pool: &PgPool) -> u64 {
    if let Ok(Some(provider_settings)) = OpenIdProvider::get_current(pool).await {
        provider_settings
            .directory_sync_interval
            .try_into()
            .unwrap_or(DIRECTORY_SYNC_INTERVAL)
    } else {
        DIRECTORY_SYNC_INTERVAL
    }
}

pub(crate) async fn do_directory_sync(pool: &PgPool) -> Result<(), DirectorySyncError> {
    if !is_directory_sync_enabled(pool).await? {
        debug!("Directory sync is disabled, skipping performing directory sync");
        return Ok(());
    }

    match get_directory_sync(pool).await {
        Ok(mut dir_sync) => {
            dir_sync.prepare().await?;
            sync_all_users_state(&dir_sync, pool).await?;
            sync_all_users_groups(&dir_sync, pool).await?;
        }
        Err(err) => {
            error!("Failed to build directory sync client: {}", err);
        }
    }

    Ok(())
}
