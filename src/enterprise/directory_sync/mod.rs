use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use crate::{
    db::{Group, Id, User},
    enterprise::db::models::openid_provider::DirectorySyncUserBehavior,
};
use sqlx::error::Error as SqlxError;
use thiserror::Error;

use super::{db::models::openid_provider::OpenIdProvider, is_enterprise_enabled};

#[derive(Debug, Error)]
pub enum DirectorySyncError {
    #[error("Database error: {0}")]
    DbError(#[from] SqlxError),
    #[error("Access token has expired")]
    AccessTokenExpired,
    #[error("Failed to process request: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Failed to build a JWT token: {0}")]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryUser {
    pub email: String,
    // Users may be disabled/suspended in the directory
    pub active: bool,
}

trait DirectorySync {
    /// Get all groups in a directory
    fn get_groups(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryGroup>, DirectorySyncError>> + Send;

    /// Get all groups a user is a member of
    fn get_user_groups(
        &self,
        user_id: &str,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryGroup>, DirectorySyncError>> + Send;

    /// Get all members of a group
    fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> impl std::future::Future<Output = Result<Vec<String>, DirectorySyncError>> + Send;

    /// Prepare the directory sync client, e.g. get an access token
    fn prepare(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), DirectorySyncError>> + Send;

    /// Get all users in the directory
    fn get_all_users(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<DirectoryUser>, DirectorySyncError>> + Send;
}

async fn sync_user_groups<T: DirectorySync>(
    directory_sync: &T,
    user: &User<Id>,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    info!("Syncing groups of user {} with the directory", user.email);
    let directory_groups = directory_sync.get_user_groups(&user.email).await?;
    let directory_group_names: Vec<&str> =
        directory_groups.iter().map(|g| g.name.as_str()).collect();

    debug!(
        "User {} is a member of {} groups in the directory: {:?}",
        user.email,
        directory_groups.len(),
        directory_group_names
    );

    let mut transaction = pool.begin().await?;

    let current_groups = user.member_of(&mut *transaction).await?;
    let current_group_names: Vec<&str> = current_groups.iter().map(|g| g.name.as_str()).collect();

    debug!(
        "User {} is a member of {} groups in Defguard: {:?}",
        user.email,
        current_groups.len(),
        current_group_names
    );

    for group in &directory_group_names {
        if !current_group_names.contains(group) {
            create_and_add_to_group(user, group, pool).await?;
        }
    }

    for current_group in &current_groups {
        if !directory_group_names.contains(&current_group.name.as_str()) {
            debug!(
                "Removing user {} from group {} as they are not a member of it in the directory",
                user.email, current_group.name
            );
            user.remove_from_group(&mut *transaction, current_group)
                .await?;
        }
    }

    transaction.commit().await?;

    Ok(())
}

/// Sync user groups with the directory if directory sync is enabled and configured, skip otherwise
pub(crate) async fn sync_user_groups_if_configured(
    user: &User<Id>,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    if !is_enterprise_enabled() {
        debug!("Enterprise is not enabled, skipping syncing user groups");
        return Ok(());
    }

    if !is_directory_sync_enabled(pool).await? {
        debug!("Directory sync is disabled, skipping syncing user groups");
        return Ok(());
    }

    match get_directory_sync_client(pool).await {
        Ok(mut dir_sync) => {
            dir_sync.prepare().await?;
            sync_user_groups(&dir_sync, user, pool).await?;
        }
        Err(err) => {
            error!("Failed to build directory sync client: {}", err);
        }
    }

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
async fn sync_all_users_groups<T: DirectorySync>(
    directory_sync: &T,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    info!("Syncing all users' groups, this may take a while...");
    let directory_groups = directory_sync.get_groups().await?;
    debug!("Found {} groups to sync", directory_groups.len());

    // Create a map of user: group to apply later
    // It will be used to decide what groups should be removed from the user and what should be added
    let mut user_group_map: HashMap<String, HashSet<&str>> = HashMap::new();
    debug!(
        "Beggining a construction of user-group mapping which will be applied later to Defguard"
    );
    for group in &directory_groups {
        match directory_sync.get_group_members(group).await {
            Ok(members) => {
                debug!(
                    "Group {} has {} members in the directory, adding them to the user-group mapping",
                    group.name,
                    members.len()
                );
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
    debug!("User-group mapping construction done, starting to apply the changes to the database");
    for (user, groups) in user_group_map.into_iter() {
        debug!("Syncing groups for user {}", user);
        let Some(user) = User::find_by_email(pool, &user).await? else {
            debug!(
                "User {} not found in the database, skipping group sync",
                user
            );
            continue;
        };

        let current_groups = user.member_of(&mut *transaction).await?;
        debug!(
            "User {} is a member of {} groups in Defguard: {:?}",
            user.email,
            current_groups.len(),
            current_groups
        );
        for current_group in &current_groups {
            debug!(
                "Checking if user {} is still a member of group {} in the directory",
                user.email, current_group.name
            );
            if !groups.contains(current_group.name.as_str()) {
                let group = Group::find_by_name(pool, &current_group.name).await?;
                if let Some(group) = group {
                    debug!("Removing user {} from group {} as they are not a member of it in the directory", user.email, group.name);
                    user.remove_from_group(&mut *transaction, &group).await?;
                } else {
                    warn!(
                        "Group {} not found in the database, skipping removing it from user {}",
                        current_group.name, user.email
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

async fn get_directory_sync_client(
    pool: &PgPool,
) -> Result<impl DirectorySync, DirectorySyncError> {
    debug!("Getting directory sync client");
    let provider_settings = OpenIdProvider::get_current(pool)
        .await?
        .ok_or(DirectorySyncError::NotConfigured)?;

    match provider_settings.name.as_str() {
        "Google" => {
            debug!("Google directory sync provider selected");
            match (
                provider_settings.google_service_account_key.as_ref(),
                provider_settings.google_service_account_email.as_ref(),
                provider_settings.admin_email.as_ref(),
            ) {
                (Some(key), Some(email), Some(admin_email)) => {
                    debug!("Google directory has all the configuration needed, proceeding with creating the sync client");
                    let client = google::GoogleDirectorySync::new(key, email, admin_email);
                    debug!("Google directory sync client created");
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
    debug!("Syncing all users' state with the directory");
    let mut transaction = pool.begin().await?;
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
    let missing_users = User::exclude(&mut *transaction, &emails)
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
    let users_to_disable =
        User::find_many_by_emails(&mut *transaction, &disabled_users_emails).await?;

    let enabled_users_emails = all_users
        .iter()
        .filter(|u| u.active)
        .map(|u| u.email.as_str())
        .collect::<Vec<&str>>();
    let users_to_enable =
        User::find_many_by_emails(&mut *transaction, &enabled_users_emails).await?;

    debug!(
        "There are {} disabled users in the directory, disabling them in Defguard...",
        users_to_disable.len()
    );

    for mut user in users_to_disable {
        if user.is_active {
            debug!(
                "Disabling user {} because they are disabled in the directory",
                user.email
            );
            user.is_active = false;
            user.save(&mut *transaction).await?;
        } else {
            debug!("User {} is already disabled, skipping", user.email);
        }
    }
    debug!("Done processing disabled users");

    debug!(
        "There are {} users missing from the directory but present in Defguard, \
    deciding what to do next based on the following settings: user action: {}, admin action: {}",
        missing_users.len(),
        user_behavior,
        admin_behavior
    );
    for mut user in missing_users {
        match user.is_admin(&mut *transaction).await? {
            true => match admin_behavior {
                DirectorySyncUserBehavior::Keep => {
                    debug!(
                        "Keeping admin {} despite not being present in the directory",
                        user.email
                    );
                }
                DirectorySyncUserBehavior::Disable => {
                    if user.is_active {
                        info!(
                            "Disabling admin {} because they are not present in the directory and the admin behavior setting is set to disable",
                            user.email
                        );
                        user.is_active = false;
                        user.save(&mut *transaction).await?;
                    } else {
                        debug!(
                            "Admin {} is already disabled in Defguard, skipping",
                            user.email
                        );
                    }
                }
                DirectorySyncUserBehavior::Delete => {
                    info!(
                        "Deleting admin {} because they are not in the directory",
                        user.email
                    );
                    user.delete(&mut *transaction).await?;
                }
            },
            false => match user_behavior {
                DirectorySyncUserBehavior::Keep => {
                    debug!(
                        "Keeping user {} despite not being present in the directory",
                        user.email
                    );
                }
                DirectorySyncUserBehavior::Disable => {
                    if user.is_active {
                        info!(
                            "Disabling user {} because they are not present in the directory and the user behavior setting is set to disable",
                            user.email
                        );
                        user.is_active = false;
                        user.save(&mut *transaction).await?;
                    } else {
                        debug!(
                            "User {} is already disabled in Defguard, skipping",
                            user.email
                        );
                    }
                }
                DirectorySyncUserBehavior::Delete => {
                    info!(
                        "Deleting user {} because they are not present in the directory",
                        user.email
                    );
                    user.delete(&mut *transaction).await?;
                }
            },
        }
    }
    debug!("Done processing missing users");

    debug!(
        "There are {} enabled users in the directory, enabling them in Defguard if they were previously disabled",
        users_to_enable.len()
    );
    for mut user in users_to_enable {
        if user.is_active {
            debug!("User {} is already enabled, skipping", user.email);
            continue;
        } else {
            debug!(
                "Enabling user {} because they are enabled in the directory and disabled in Defguard",
                user.email
            );
            user.is_active = true;
            user.save(&mut *transaction).await?;
        }
    }
    debug!("Done processing enabled users");
    transaction.commit().await?;
    debug!("Syncing all users' state with the directory done");
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
    if !is_enterprise_enabled() {
        debug!("Enterprise is not enabled, skipping performing directory sync");
        return Ok(());
    }

    if !is_directory_sync_enabled(pool).await? {
        debug!("Directory sync is disabled, skipping performing directory sync");
        return Ok(());
    }

    match get_directory_sync_client(pool).await {
        Ok(mut dir_sync) => {
            // TODO: Directory sync's access token is dropped every time, find a way to preserve it
            // Same goes for Etags, those could be used to reduce the amount of data transferred. Some way
            // of preserving them should be implemented.
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::{config::DefGuardConfig, SERVER_CONFIG};
    use secrecy::ExposeSecret;

    async fn make_test_provider(
        pool: &PgPool,
        user_behavior: DirectorySyncUserBehavior,
        admin_behavior: DirectorySyncUserBehavior,
    ) -> OpenIdProvider<Id> {
        let current = OpenIdProvider::get_current(pool).await.unwrap();

        if let Some(provider) = current {
            provider.delete(pool).await.unwrap();
        }

        OpenIdProvider::new(
            "Google".to_string(),
            "base_url".to_string(),
            "client_id".to_string(),
            "client_secret".to_string(),
            Some("display_name".to_string()),
            Some("google_service_account_key".to_string()),
            Some("google_service_account_email".to_string()),
            Some("admin_email".to_string()),
            true,
            60,
            user_behavior,
            admin_behavior,
        )
        .save(pool)
        .await
        .unwrap()
    }

    async fn make_test_user(name: &str, pool: &PgPool) -> User<Id> {
        User::new(
            name,
            None,
            "lastname",
            "firstname",
            format!("{}@email.com", name).as_str(),
            None,
        )
        .save(pool)
        .await
        .unwrap()
    }

    async fn get_test_user(pool: &PgPool, name: &str) -> Option<User<Id>> {
        User::find_by_username(pool, name).await.unwrap()
    }

    async fn make_admin(pool: &PgPool, user: &User<Id>) {
        let admin_group = Group::find_by_name(pool, "admin").await.unwrap().unwrap();
        user.add_to_group(pool, &admin_group).await.unwrap();
    }

    // Keep both users and admins
    #[sqlx::test]
    async fn test_users_state_keep_both(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
        )
        .await;
        let mut client = get_directory_sync_client(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user1 = make_test_user("user1", &pool).await;
        make_test_user("user2", &pool).await;
        make_test_user("testuser", &pool).await;
        make_admin(&pool, &user1).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        sync_all_users_state(&client, &pool).await.unwrap();

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());
    }

    // Delete users, keep admins
    #[sqlx::test]
    async fn test_users_state_delete_users(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Keep,
        )
        .await;
        let mut client = get_directory_sync_client(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user("user1", &pool).await;
        make_test_user("user2", &pool).await;
        make_test_user("testuser", &pool).await;
        make_admin(&pool, &user1).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        sync_all_users_state(&client, &pool).await.unwrap();

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_none());
        assert!(get_test_user(&pool, "testuser").await.is_some());
    }

    // Delete admins, keep users
    #[sqlx::test]
    async fn test_users_state_delete_admins(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        User::init_admin_user(&pool, config.default_admin_password.expose_secret())
            .await
            .unwrap();
        let _ = make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Delete,
        )
        .await;
        let mut client = get_directory_sync_client(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user("user1", &pool).await;
        make_test_user("user2", &pool).await;
        make_test_user("testuser", &pool).await;
        make_admin(&pool, &user1).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        assert!(User::find_by_email(&pool, "admin@defguard")
            .await
            .unwrap()
            .is_some());

        sync_all_users_state(&client, &pool).await.unwrap();

        assert!(get_test_user(&pool, "user1").await.is_none());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // We should never delete the main admin user
        assert!(User::find_by_email(&pool, "admin@defguard")
            .await
            .unwrap()
            .is_some());
    }

    #[sqlx::test]
    async fn test_users_state_delete_both(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
        )
        .await;
        User::init_admin_user(&pool, config.default_admin_password.expose_secret())
            .await
            .unwrap();
        let mut client = get_directory_sync_client(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user("user1", &pool).await;
        make_test_user("user2", &pool).await;
        make_test_user("testuser", &pool).await;
        make_admin(&pool, &user1).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());
        assert!(User::find_by_email(&pool, "admin@defguard")
            .await
            .unwrap()
            .is_some());

        sync_all_users_state(&client, &pool).await.unwrap();

        assert!(get_test_user(&pool, "user1").await.is_none());
        assert!(get_test_user(&pool, "user2").await.is_none());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // We should never delete the main admin user
        assert!(User::find_by_email(&pool, "admin@defguard")
            .await
            .unwrap()
            .is_some());
    }

    #[sqlx::test]
    async fn test_users_state_disable_users(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Disable,
            DirectorySyncUserBehavior::Keep,
        )
        .await;
        let mut client = get_directory_sync_client(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user("user1", &pool).await;
        make_test_user("user2", &pool).await;
        make_test_user("testuser", &pool).await;
        make_test_user("testuserdisabled", &pool).await;
        make_admin(&pool, &user1).await;

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(user1.is_active);
        assert!(user2.is_active);
        assert!(testuser.is_active);
        assert!(testuserdisabled.is_active);

        sync_all_users_state(&client, &pool).await.unwrap();

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(user1.is_active);
        assert!(!user2.is_active);
        assert!(testuser.is_active);
        assert!(!testuserdisabled.is_active);
    }

    #[sqlx::test]
    async fn test_users_state_disable_admins(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Disable,
        )
        .await;
        let mut client = get_directory_sync_client(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user("user1", &pool).await;
        make_test_user("user2", &pool).await;
        make_test_user("testuser", &pool).await;
        make_test_user("testuserdisabled", &pool).await;
        make_admin(&pool, &user1).await;

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(user1.is_active);
        assert!(user2.is_active);
        assert!(testuser.is_active);
        assert!(testuserdisabled.is_active);

        sync_all_users_state(&client, &pool).await.unwrap();

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(!user1.is_active);
        assert!(user2.is_active);
        assert!(testuser.is_active);
        assert!(!testuserdisabled.is_active);
    }

    #[sqlx::test]
    async fn test_users_groups(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
        )
        .await;
        let mut client = get_directory_sync_client(&pool).await.unwrap();
        client.prepare().await.unwrap();

        make_test_user("testuser", &pool).await;
        make_test_user("testuser2", &pool).await;
        make_test_user("testuserdisabled", &pool).await;
        sync_all_users_groups(&client, &pool).await.unwrap();

        let mut groups = Group::all(&pool).await.unwrap();

        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuser2 = get_test_user(&pool, "testuser2").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        let testuser_groups = testuser.member_of(&pool).await.unwrap();
        let testuser2_groups = testuser2.member_of(&pool).await.unwrap();
        let testuserdisabled_groups = testuserdisabled.member_of(&pool).await.unwrap();

        assert_eq!(testuser_groups.len(), 3);
        assert_eq!(testuser2_groups.len(), 3);
        assert_eq!(testuserdisabled_groups.len(), 3);
        groups.sort_by(|a, b| a.name.cmp(&b.name));

        for i in (0..3).rev() {
            assert_eq!(testuser_groups[i].name, format!("group{}", i + 1));
            assert_eq!(testuser2_groups[i].name, format!("group{}", i + 1));
            assert_eq!(testuserdisabled_groups[i].name, format!("group{}", i + 1));
        }
    }

    #[sqlx::test]
    async fn test_sync_user_groups(pool: PgPool) {
        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
        )
        .await;
        let mut client = get_directory_sync_client(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user("testuser", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        sync_user_groups_if_configured(&user, &pool).await.unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 1);
        let group = Group::find_by_name(&pool, "group1").await.unwrap().unwrap();
        assert_eq!(user_groups[0].id, group.id);
    }
}
