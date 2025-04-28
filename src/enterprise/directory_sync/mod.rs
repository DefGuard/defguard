use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use paste::paste;
use reqwest::header::AUTHORIZATION;
use sqlx::{error::Error as SqlxError, PgPool};
use thiserror::Error;
use tokio::sync::broadcast::Sender;

#[cfg(not(test))]
use super::is_enterprise_enabled;
use super::{
    db::models::openid_provider::{DirectorySyncTarget, OpenIdProvider},
    ldap::utils::ldap_update_users_state,
};
use crate::{
    db::{GatewayEvent, Group, Id, User},
    enterprise::{
        db::models::openid_provider::DirectorySyncUserBehavior,
        ldap::utils::{ldap_add_users_to_groups, ldap_delete_users, ldap_remove_users_from_groups},
    },
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_PAGINATION_SLOWDOWN: Duration = Duration::from_millis(100);

#[derive(Debug, Error)]
pub enum DirectorySyncError {
    #[error("Database error: {0}")]
    DbError(#[from] SqlxError),
    #[error("Access token has expired or is not present. An issue may have occured while trying to obtain a new one.")]
    AccessTokenExpired,
    #[error("Processing a request to the provider's API failed: {0}")]
    RequestError(String),
    #[error(
        "Failed to build a JWT token, required for communicating with the provider's API: {0}"
    )]
    JWTError(#[from] jsonwebtoken::errors::Error),
    #[error("The selected provider {0} is not supported for directory sync")]
    UnsupportedProvider(String),
    #[error("Directory sync is not configured")]
    NotConfigured,
    #[error("Couldn't map provider's group to a Defguard group as it doesn't exist. There may be an issue with automatic group creation. Error details: {0}")]
    DefGuardGroupNotFound(String),
    #[error("The provided provider configuration is invalid: {0}")]
    InvalidProviderConfiguration(String),
    #[error("Couldn't construct URL from the given string: {0}")]
    InvalidUrl(String),
    #[error("Failed to update network state: {0}")]
    NetworkUpdateError(String),
    #[error("Failed to update user state: {0}")]
    UserUpdateError(String),
    #[error("Failed to find user: {0}")]
    UserNotFound(String),
    #[error(
        "Found multiple users with given parameters ({0}) but expected one. Won't proceed further."
    )]
    MultipleUsersFound(String),
}

impl From<reqwest::Error> for DirectorySyncError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_decode() {
            Self::RequestError(format!("There was an error while trying to decode provider's response, it may be malformed: {err}"))
        } else if err.is_timeout() {
            Self::RequestError(format!(
                "The request to the provider's API timed out: {err}"
            ))
        } else {
            Self::RequestError(err.to_string())
        }
    }
}

pub mod google;
pub mod microsoft;
pub mod okta;
#[cfg(test)]
pub mod testprovider;

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

#[trait_variant::make(Send)]
#[trait_variant::make(Sync)]
trait DirectorySync {
    /// Get all groups in a directory
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError>;

    /// Get all groups a user is a member of
    async fn get_user_groups(
        &self,
        user_id: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError>;

    /// Get all members of a group
    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<String>, DirectorySyncError>;

    /// Prepare the directory sync client, e.g. get an access token
    async fn prepare(&mut self) -> Result<(), DirectorySyncError>;

    /// Get all users in the directory
    async fn get_all_users(&self) -> Result<Vec<DirectoryUser>, DirectorySyncError>;

    /// Tests the connection to the directory
    async fn test_connection(&self) -> Result<(), DirectorySyncError>;
}

/// This macro generates a boilerplate enum which enables a simple polymorphism for things that implement
/// the DirectorySync trait without having to resolve to fully dynamic dispatch using something like Box<dyn DirectorySync>.
///
///
/// When creating a new provider, make sure that:
/// - The provider main struct is called <PROVIDER>DirectorySync, e.g. GoogleDirectorySync
/// - The provider implements the [`DirectorySync`] trait
/// - You implemented some way to initialize the provider client and added an initialization step in the [`DirectorySyncClient::build`] function
/// - You added the provider name to the macro invocation below the macro definition
/// - You've implemented your provider logic in a file called the same as your provider but lowercase, e.g. google.rs
// If you have time to refactor the whole thing to use boxes instead, go ahead.
macro_rules! dirsync_clients {
    ($($variant:ident),*) => {
        paste! {
        pub(crate) enum DirectorySyncClient {
            $(
                $variant([< $variant:lower >]::[< $variant DirectorySync >]),
            )*
        }
        }

        impl DirectorySync for DirectorySyncClient {
            async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
                match self {
                    $(
                        DirectorySyncClient::$variant(client) => client.get_groups().await,
                    )*
                }
            }

            async fn get_user_groups(&self, user_id: &str) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
                match self {
                    $(
                        DirectorySyncClient::$variant(client) => client.get_user_groups(user_id).await,
                    )*
                }
            }

            async fn get_group_members(&self, group: &DirectoryGroup) -> Result<Vec<String>, DirectorySyncError> {
                match self {
                    $(
                        DirectorySyncClient::$variant(client) => client.get_group_members(group).await,
                    )*
                }
            }

            async fn prepare(&mut self) -> Result<(), DirectorySyncError> {
                match self {
                    $(
                        DirectorySyncClient::$variant(client) => client.prepare().await,
                    )*
                }
            }

            async fn get_all_users(&self) -> Result<Vec<DirectoryUser>, DirectorySyncError> {
                match self {
                    $(
                        DirectorySyncClient::$variant(client) => client.get_all_users().await,
                    )*
                }
            }

            async fn test_connection(&self) -> Result<(), DirectorySyncError> {
                match self {
                    $(
                        DirectorySyncClient::$variant(client) => client.test_connection().await,
                    )*
                }
            }
        }
    };
}

#[cfg(test)]
dirsync_clients!(Google, Microsoft, Okta, TestProvider);

#[cfg(not(test))]
dirsync_clients!(Google, Microsoft, Okta);

impl DirectorySyncClient {
    /// Builds the current directory sync client based on the current provider settings (provider name), if possible.
    pub(crate) async fn build(pool: &PgPool) -> Result<Self, DirectorySyncError> {
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
                        Ok(Self::Google(client))
                    }
                    _ => Err(DirectorySyncError::NotConfigured),
                }
            }
            "Microsoft" => {
                debug!("Microsoft directory sync provider selected");
                let client = microsoft::MicrosoftDirectorySync::new(
                    provider_settings.client_id,
                    provider_settings.client_secret,
                    provider_settings.base_url,
                    provider_settings.directory_sync_group_match,
                );
                debug!("Microsoft directory sync client created");
                Ok(Self::Microsoft(client))
            }
            "Okta" => {
                if let (Some(jwk), Some(client_id)) = (
                    provider_settings.okta_private_jwk.as_ref(),
                    provider_settings.okta_dirsync_client_id.as_ref(),
                ) {
                    debug!("Okta directory has all the configuration needed, proceeding with creating the sync client");
                    let client =
                        okta::OktaDirectorySync::new(jwk, client_id, &provider_settings.base_url);
                    debug!("Okta directory sync client created");
                    Ok(Self::Okta(client))
                } else {
                    Err(DirectorySyncError::InvalidProviderConfiguration(
                        "Okta provider is not configured correctly for Directory Sync. Okta private key or client id is missing."
                            .to_string(),
                    ))
                }
            }
            #[cfg(test)]
            "Test" => Ok(Self::TestProvider(testprovider::TestProviderDirectorySync)),
            _ => Err(DirectorySyncError::UnsupportedProvider(
                provider_settings.name.clone(),
            )),
        }
    }
}

async fn sync_user_groups<T: DirectorySync>(
    directory_sync: &T,
    user: &User<Id>,
    pool: &PgPool,
    wg_tx: &Sender<GatewayEvent>,
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
    let mut add_to_ldap_groups = HashSet::new();
    let mut remove_from_ldap_groups = HashSet::new();

    debug!(
        "User {} is a member of {} groups in Defguard: {:?}",
        user.email,
        current_groups.len(),
        current_group_names
    );

    for group in &directory_group_names {
        if !current_group_names.contains(group) {
            create_and_add_to_group(user, group, pool).await?;
            add_to_ldap_groups.insert(*group);
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
            remove_from_ldap_groups.insert(current_group.name.as_str());
        }
    }

    user.sync_allowed_devices(&mut transaction, wg_tx)
        .await
        .map_err(|err| {
            DirectorySyncError::NetworkUpdateError(format!(
            "Failed to sync allowed devices for user {} during directory synchronization: {err}",
            user.email
        ))
        })?;
    transaction.commit().await?;

    let mut user_groups = HashMap::new();
    user_groups.insert(user, add_to_ldap_groups);
    ldap_add_users_to_groups(user_groups, pool).await;

    let mut user_groups = HashMap::new();
    user_groups.insert(user, remove_from_ldap_groups);
    ldap_remove_users_from_groups(user_groups, pool).await;

    Ok(())
}

pub(crate) async fn test_directory_sync_connection(
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    #[cfg(not(test))]
    if !is_enterprise_enabled() {
        debug!("Enterprise is not enabled, skipping testing directory sync connection");
        return Ok(());
    }

    match DirectorySyncClient::build(pool).await {
        Ok(mut dir_sync) => {
            dir_sync.prepare().await?;
            dir_sync.test_connection().await?;
        }
        Err(err) => {
            error!("Failed to build directory sync client: {err}");
        }
    }

    Ok(())
}

/// Sync user groups with the directory if directory sync is enabled and configured, skip otherwise
pub(crate) async fn sync_user_groups_if_configured(
    user: &User<Id>,
    pool: &PgPool,
    wg_tx: &Sender<GatewayEvent>,
) -> Result<(), DirectorySyncError> {
    #[cfg(not(test))]
    if !is_enterprise_enabled() {
        debug!("Enterprise is not enabled, skipping syncing user groups");
        return Ok(());
    }

    let provider = OpenIdProvider::get_current(pool).await?;
    if !is_directory_sync_enabled(provider.as_ref()) {
        debug!("Directory sync is disabled, skipping syncing user groups");
        return Ok(());
    }

    match DirectorySyncClient::build(pool).await {
        Ok(mut dir_sync) => {
            dir_sync.prepare().await?;
            sync_user_groups(&dir_sync, user, pool, wg_tx).await?;
        }
        Err(err) => {
            error!("Failed to build directory sync client: {err}");
        }
    }

    Ok(())
}

/// Create a group if it doesn't exist and add a user to it if they are not already a member
async fn create_and_add_to_group(
    user: &User<Id>,
    group_name: &str,
    pool: &PgPool,
) -> Result<(), DirectorySyncError> {
    debug!(
        "Creating group {} if it doesn't exist and adding user {group_name} to it if they are not already a member",
        user.email
    );
    let group = if let Some(group) = Group::find_by_name(pool, group_name).await? {
        debug!("Group {group_name} already exists, skipping creation");
        group
    } else {
        debug!("Group {group_name} didn't exist, creating it now");
        let new_group = Group::new(group_name).save(pool).await?;
        debug!("Group {group_name} created");
        new_group
    };

    debug!(
        "Adding user {} to group {group_name} if they are not already a member",
        user.email
    );
    user.add_to_group(pool, &group).await?;
    debug!(
        "User {} was added to group {group_name} if they weren't already a member",
        user.email
    );
    Ok(())
}

/// Sync all users' groups with the directory
async fn sync_all_users_groups<T: DirectorySync>(
    directory_sync: &T,
    pool: &PgPool,
    wg_tx: &Sender<GatewayEvent>,
) -> Result<(), DirectorySyncError> {
    info!("Syncing all users' groups with the directory, this may take a while...");
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

    let mut affected_users = Vec::new();

    let mut transaction = pool.begin().await?;
    debug!("User-group mapping construction done, starting to apply the changes to the database");
    let mut admin_count = User::find_admins(&mut *transaction).await?.len();
    for (user, groups) in user_group_map {
        debug!("Syncing groups for user {user}");
        let Some(user) = User::find_by_email(pool, &user).await? else {
            debug!("User {user} not found in the database, skipping group sync");
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
                if current_group.is_admin {
                    if admin_count == 1 {
                        error!(
                            "User {} is the last admin in the system, can't remove them from an admin group {}",
                            user.email, current_group.name
                        );
                        continue;
                    }
                    debug!(
                            "Removing user {} from group {} as they are not a member of it in the directory",
                            user.email, current_group.name
                        );
                    user.remove_from_group(&mut *transaction, current_group)
                        .await?;
                    admin_count -= 1;
                } else {
                    debug!("Removing user {} from group {} as they are not a member of it in the directory",
                    user.email, current_group.name);
                    user.remove_from_group(&mut *transaction, current_group)
                        .await?;
                }
            }
        }

        for group in groups {
            create_and_add_to_group(&user, group, pool).await?;
        }

        user.sync_allowed_devices(&mut transaction, wg_tx).await.map_err(|err| {
            DirectorySyncError::NetworkUpdateError(format!(
                "Failed to sync allowed devices for user {} during directory synchronization: {err}",
                user.email
            ))
        })?;

        affected_users.push(user);
    }
    transaction.commit().await?;

    ldap_update_users_state(affected_users.iter_mut().collect::<Vec<_>>(), pool).await;
    info!("Syncing all users' groups done.");
    Ok(())
}

fn is_directory_sync_enabled(provider: Option<&OpenIdProvider<Id>>) -> bool {
    debug!("Checking if directory sync is enabled");
    provider.map_or_else(
        || {
            debug!("No openid provider found, directory sync is disabled");
            false
        },
        |provider_settings| {
            debug!(
                "Directory sync enabled state: {}",
                provider_settings.directory_sync_enabled
            );
            provider_settings.directory_sync_enabled
        },
    )
}

async fn sync_all_users_state<T: DirectorySync>(
    directory_sync: &T,
    pool: &PgPool,
    wg_tx: &Sender<GatewayEvent>,
) -> Result<(), DirectorySyncError> {
    info!("Syncing all users' state with the directory, this may take a while...");
    let mut transaction = pool.begin().await?;
    let all_users = directory_sync.get_all_users().await?;
    let settings = OpenIdProvider::get_current(pool)
        .await?
        .ok_or(DirectorySyncError::NotConfigured)?;

    let user_behavior = settings.directory_sync_user_behavior;
    let admin_behavior = settings.directory_sync_admin_behavior;

    let emails = all_users
        .iter()
        .map(|u| u.email.as_str())
        .collect::<Vec<&str>>();
    let missing_users = User::exclude(&mut *transaction, &emails)
        .await?
        .into_iter()
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

    let mut modified_users = Vec::new();
    let mut deleted_users = Vec::new();

    for mut user in users_to_disable {
        if user.is_active {
            debug!(
                "Disabling user {} because they are disabled in the directory",
                user.email
            );
            user.disable(&mut transaction, wg_tx).await.map_err(|err| {
                DirectorySyncError::UserUpdateError(format!(
                    "Failed to disable user {} during directory synchronization: {err}",
                    user.email
                ))
            })?;
            modified_users.push(user);
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
    // Keep the admin count to prevent deleting the last admin
    let mut admin_count = User::find_admins(&mut *transaction).await?.len();
    for mut user in missing_users {
        if user.is_admin(&mut *transaction).await? {
            match admin_behavior {
                DirectorySyncUserBehavior::Keep => {
                    debug!(
                        "Keeping admin {} despite not being present in the directory",
                        user.email
                    );
                }
                DirectorySyncUserBehavior::Disable => {
                    if user.is_active {
                        if admin_count == 1 {
                            error!(
                                "Admin {} is the last admin in the system; can't disable",
                                user.email
                            );
                            continue;
                        }
                        info!(
                            "Disabling admin {} because it is not present in the directory and
                            the admin behavior setting is set to disable",
                            user.email
                        );
                        user.disable(&mut transaction, wg_tx).await.map_err(|err| {
                            DirectorySyncError::UserUpdateError(format!(
                                "Failed to disable admin {} during directory synchronization: {err}",
                                user.email
                            ))
                        })?;
                        admin_count -= 1;
                        modified_users.push(user);
                    } else {
                        debug!(
                            "Admin {} is already disabled in Defguard, skipping",
                            user.email
                        );
                    }
                }
                DirectorySyncUserBehavior::Delete => {
                    if admin_count == 1 {
                        error!(
                            "Admin {} is the last admin in the system, can't delete them",
                            user.email
                        );
                        continue;
                    }
                    info!(
                        "Deleting admin {} because they are not present in the directory",
                        user.email
                    );
                    if user.ldap_sync_allowed(&mut *transaction).await? {
                        deleted_users.push(user.clone().as_noid());
                    }
                    user.delete_and_cleanup(&mut transaction, wg_tx)
                        .await
                        .map_err(|err| {
                            DirectorySyncError::UserUpdateError(format!(
                                "Failed to delete admin during directory synchronization: {err}"
                            ))
                        })?;
                    admin_count -= 1;
                }
            }
        } else {
            match user_behavior {
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
                        user.disable(&mut transaction, wg_tx).await.map_err(|err| {
                            DirectorySyncError::UserUpdateError(format!(
                                "Failed to disable user {} during directory synchronization: {err}",
                                user.email
                            ))
                        })?;
                        modified_users.push(user);
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
                    if user.ldap_sync_allowed(&mut *transaction).await? {
                        deleted_users.push(user.clone().as_noid());
                    }
                    user.delete_and_cleanup(&mut transaction, wg_tx)
                        .await
                        .map_err(|err| {
                            DirectorySyncError::UserUpdateError(format!(
                                "Failed to delete user during directory synchronization: {err}"
                            ))
                        })?;
                }
            }
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
        }
        debug!(
            "Enabling user {} because it is enabled in the directory and disabled in Defguard",
            user.email
        );
        user.is_active = true;
        user.save(&mut *transaction).await?;
        modified_users.push(user);
    }
    debug!("Done processing enabled users");
    transaction.commit().await?;

    ldap_delete_users(deleted_users.iter().collect::<Vec<_>>(), pool).await;
    ldap_update_users_state(modified_users.iter_mut().collect::<Vec<_>>(), pool).await;

    info!("Syncing all users' state with the directory done");

    Ok(())
}

// The default inverval for the directory sync job
const DIRECTORY_SYNC_INTERVAL: u64 = 60 * 10;

/// Used to inform the utility thread how often it should perform the directory sync job. See [`run_utility_thread`] for more details.
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

// Performs the directory sync job. This function is called by the utility thread.
pub(crate) async fn do_directory_sync(
    pool: &PgPool,
    wireguard_tx: &Sender<GatewayEvent>,
) -> Result<(), DirectorySyncError> {
    #[cfg(not(test))]
    if !is_enterprise_enabled() {
        debug!("Enterprise is not enabled, skipping performing directory sync");
        return Ok(());
    }

    // TODO: Reduce the amount of times those settings are retrieved in the whole directory sync process
    let provider = OpenIdProvider::get_current(pool).await?;

    if !is_directory_sync_enabled(provider.as_ref()) {
        debug!("Directory sync is disabled, skipping performing directory sync");
        return Ok(());
    }

    let sync_target = provider
        .ok_or(DirectorySyncError::NotConfigured)?
        .directory_sync_target;

    match DirectorySyncClient::build(pool).await {
        Ok(mut dir_sync) => {
            // TODO: Directory sync's access token is dropped every time, find a way to preserve it
            // Same goes for Etags, those could be used to reduce the amount of data transferred. Some way
            // of preserving them should be implemented.
            dir_sync.prepare().await?;
            if matches!(
                sync_target,
                DirectorySyncTarget::All | DirectorySyncTarget::Users
            ) {
                sync_all_users_state(&dir_sync, pool, wireguard_tx).await?;
            }
            if matches!(
                sync_target,
                DirectorySyncTarget::All | DirectorySyncTarget::Groups
            ) {
                sync_all_users_groups(&dir_sync, pool, wireguard_tx).await?;
            }
        }
        Err(err) => {
            error!("Failed to build directory sync client: {err}");
        }
    }

    Ok(())
}

// Helpers shared between the directory sync providers
//

/// Parse a reqwest response and return the JSON body if the response is OK, otherwise map an error to a DirectorySyncError::RequestError
/// The context_message is used to provide more context to the error message.
async fn parse_response<T>(
    response: reqwest::Response,
    context_message: &str,
) -> Result<T, DirectorySyncError>
where
    T: serde::de::DeserializeOwned,
{
    let status = &response.status();
    match status {
        &reqwest::StatusCode::OK => {
            let json: serde_json::Value = response.json().await?;
            Ok(serde_json::from_value(json).map_err(|err| {
                DirectorySyncError::RequestError(format!("{context_message} Error: {err}"))
            })?)
        }
        _ => Err(DirectorySyncError::RequestError(format!(
            "{context_message} Code returned: {status}. Details: {}",
            response.text().await?
        ))),
    }
}

/// Make a GET request to the given URL with the given token and query parameters
async fn make_get_request(
    url: &str,
    token: &str,
    query: Option<&[(&str, &str)]>,
) -> Result<reqwest::Response, DirectorySyncError> {
    let client = reqwest::Client::new();
    let query = query.unwrap_or_default();
    let response = client
        .get(url)
        .query(query)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?;
    Ok(response)
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use ipnetwork::IpNetwork;
    use secrecy::ExposeSecret;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use tokio::sync::broadcast;

    use super::*;
    use crate::{
        config::DefGuardConfig,
        db::{
            models::{device::DeviceType, settings::initialize_current_settings},
            setup_pool, Device, Session, SessionState, Settings, WireguardNetwork,
        },
        enterprise::db::models::openid_provider::DirectorySyncTarget,
        SERVER_CONFIG,
    };

    async fn get_test_network(pool: &PgPool) -> WireguardNetwork<Id> {
        WireguardNetwork::find_by_name(pool, "test")
            .await
            .unwrap()
            .unwrap()
            .pop()
            .unwrap()
    }

    async fn make_test_provider(
        pool: &PgPool,
        user_behavior: DirectorySyncUserBehavior,
        admin_behavior: DirectorySyncUserBehavior,
        target: DirectorySyncTarget,
    ) -> OpenIdProvider<Id> {
        Settings::init_defaults(pool).await.unwrap();
        initialize_current_settings(pool).await.unwrap();

        let current = OpenIdProvider::get_current(pool).await.unwrap();

        if let Some(provider) = current {
            provider.delete(pool).await.unwrap();
        }

        WireguardNetwork::new(
            "test".to_string(),
            vec![IpNetwork::from_str("10.10.10.1/24").unwrap()],
            1234,
            "123.123.123.123".to_string(),
            None,
            vec![],
            false,
            32,
            32,
            false,
            false,
        )
        .unwrap()
        .save(pool)
        .await
        .unwrap();

        OpenIdProvider::new(
            "Test".to_string(),
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
            target,
            None,
            None,
            vec![],
        )
        .save(pool)
        .await
        .unwrap()
    }

    async fn make_test_user_and_device(name: &str, pool: &PgPool) -> User<Id> {
        let user = User::new(
            name,
            None,
            "lastname",
            "firstname",
            format!("{name}@email.com").as_str(),
            None,
        )
        .save(pool)
        .await
        .unwrap();

        let dev = Device::new(
            format!("{name}-device"),
            format!("{name}-key"),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(pool)
        .await
        .unwrap();

        let mut transaction = pool.begin().await.unwrap();
        dev.add_to_all_networks(&mut transaction).await.unwrap();
        transaction.commit().await.unwrap();

        user
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
    async fn test_users_state_keep_both(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user1 = make_test_user_and_device("user1", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_admin(&pool, &user1).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        sync_all_users_state(&client, &pool, &wg_tx).await.unwrap();

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // No events
        assert!(wg_rx.try_recv().is_err());
    }

    // Delete users, keep admins
    #[sqlx::test]
    async fn test_users_state_delete_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        let user2 = make_test_user_and_device("user2", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_admin(&pool, &user1).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        sync_all_users_state(&client, &pool, &wg_tx).await.unwrap();

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_none());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        let event = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event {
            assert_eq!(dev.device.user_id, user2.id);
        } else {
            panic!("Expected a DeviceDeleted event");
        }
    }
    #[sqlx::test]
    async fn test_users_state_delete_admins(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
        User::init_admin_user(&pool, config.default_admin_password.expose_secret())
            .await
            .unwrap();

        let _ = make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        let user3 = make_test_user_and_device("user3", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_admin(&pool, &user1).await;
        make_admin(&pool, &user3).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());
        sync_all_users_state(&client, &pool, &wg_tx).await.unwrap();

        assert!(
            get_test_user(&pool, "user1").await.is_none()
                || get_test_user(&pool, "user3").await.is_none()
        );
        assert!(
            get_test_user(&pool, "user1").await.is_some()
                || get_test_user(&pool, "user3").await.is_some()
        );
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // Check that we received a device deleted event for whichever admin was removed
        let event = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event {
            assert!(dev.device.user_id == user1.id || dev.device.user_id == user3.id);
        } else {
            panic!("Expected a DeviceDeleted event");
        }
    }

    #[sqlx::test]
    async fn test_users_state_delete_both(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
        )
        .await;
        User::init_admin_user(&pool, config.default_admin_password.expose_secret())
            .await
            .unwrap();
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        let user2 = make_test_user_and_device("user2", &pool).await;
        let user3 = make_test_user_and_device("user3", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_admin(&pool, &user1).await;
        make_admin(&pool, &user3).await;

        assert!(get_test_user(&pool, "user1").await.is_some());
        assert!(get_test_user(&pool, "user2").await.is_some());
        assert!(get_test_user(&pool, "testuser").await.is_some());
        sync_all_users_state(&client, &pool, &wg_tx).await.unwrap();

        assert!(
            get_test_user(&pool, "user1").await.is_none()
                || get_test_user(&pool, "user3").await.is_none()
        );
        assert!(
            get_test_user(&pool, "user1").await.is_some()
                || get_test_user(&pool, "user3").await.is_some()
        );
        assert!(get_test_user(&pool, "user2").await.is_none());
        assert!(get_test_user(&pool, "testuser").await.is_some());

        // Check for device deletion events
        let event1 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event1 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user2.id
                    || dev.device.user_id == user3.id
            );
        } else {
            panic!("Expected a DeviceDeleted event");
        }

        let event2 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event2 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user2.id
                    || dev.device.user_id == user3.id
            );
        } else {
            panic!("Expected a DeviceDeleted event");
        }
    }

    #[sqlx::test]
    async fn test_users_state_disable_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Disable,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("testuserdisabled", &pool).await;
        make_admin(&pool, &user1).await;

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();
        let disabled_user_session = Session::new(
            testuserdisabled.id,
            SessionState::PasswordVerified,
            "127.0.0.1".into(),
            None,
        );
        disabled_user_session.save(&pool).await.unwrap();
        assert!(Session::find_by_id(&pool, &disabled_user_session.id)
            .await
            .unwrap()
            .is_some());

        assert!(user1.is_active);
        assert!(user2.is_active);
        assert!(testuser.is_active);
        assert!(testuserdisabled.is_active);

        sync_all_users_state(&client, &pool, &wg_tx).await.unwrap();

        // Check for device disconnection events
        let event1 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event1 {
            assert!(dev.device.user_id == user2.id || dev.device.user_id == testuserdisabled.id);
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let event2 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event2 {
            assert!(dev.device.user_id == user2.id || dev.device.user_id == testuserdisabled.id);
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(Session::find_by_id(&pool, &disabled_user_session.id)
            .await
            .unwrap()
            .is_none());
        assert!(user1.is_active);
        assert!(!user2.is_active);
        assert!(testuser.is_active);
        assert!(!testuserdisabled.is_active);
    }
    #[sqlx::test]
    async fn test_users_state_disable_admins(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16); // Added mut wg_rx
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Keep,
            DirectorySyncUserBehavior::Disable,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        let user1 = make_test_user_and_device("user1", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        let user3 = make_test_user_and_device("user3", &pool).await;
        make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("testuserdisabled", &pool).await;
        make_admin(&pool, &user1).await;
        make_admin(&pool, &user3).await;

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(user1.is_active);
        assert!(user2.is_active);
        assert!(user3.is_active);
        assert!(testuser.is_active);
        assert!(testuserdisabled.is_active);

        sync_all_users_state(&client, &pool, &wg_tx).await.unwrap();

        // Check for device disconnection events
        let event1 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event1 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user3.id
                    || dev.device.user_id == testuserdisabled.id
            );
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let event2 = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event2 {
            assert!(
                dev.device.user_id == user1.id
                    || dev.device.user_id == user3.id
                    || dev.device.user_id == testuserdisabled.id
            );
        } else {
            panic!("Expected a DeviceDisconnected event");
        }

        let user1 = get_test_user(&pool, "user1").await.unwrap();
        let user2 = get_test_user(&pool, "user2").await.unwrap();
        let user3 = get_test_user(&pool, "user3").await.unwrap();
        let testuser = get_test_user(&pool, "testuser").await.unwrap();
        let testuserdisabled = get_test_user(&pool, "testuserdisabled").await.unwrap();

        assert!(!user1.is_active || !user3.is_active);
        assert!(user1.is_active || user3.is_active);
        assert!(user2.is_active);
        assert!(testuser.is_active);
        assert!(!testuserdisabled.is_active);
    }

    #[sqlx::test]
    async fn test_users_groups(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("testuser2", &pool).await;
        make_test_user_and_device("testuserdisabled", &pool).await;
        sync_all_users_groups(&client, &pool, &wg_tx).await.unwrap();

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

        let group_present =
            |groups: &Vec<Group<Id>>, name: &str| groups.iter().any(|g| g.name == name);

        assert!(group_present(&testuser_groups, "group1"));
        assert!(group_present(&testuser_groups, "group2"));
        assert!(group_present(&testuser_groups, "group3"));

        assert!(group_present(&testuser2_groups, "group1"));
        assert!(group_present(&testuser2_groups, "group2"));
        assert!(group_present(&testuser2_groups, "group3"));

        assert!(group_present(&testuserdisabled_groups, "group1"));
        assert!(group_present(&testuserdisabled_groups, "group2"));
        assert!(group_present(&testuserdisabled_groups, "group3"));
    }

    #[sqlx::test]
    async fn test_sync_user_groups(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        sync_user_groups_if_configured(&user, &pool, &wg_tx)
            .await
            .unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 1);
        let group = Group::find_by_name(&pool, "group1").await.unwrap().unwrap();
        assert_eq!(user_groups[0].id, group.id);
    }

    #[sqlx::test]
    async fn test_sync_target_users(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::Users,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        do_directory_sync(&pool, &wg_tx).await.unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
    }

    #[sqlx::test]
    async fn test_sync_target_all(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, mut wg_rx) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
        )
        .await;
        let network = get_test_network(&pool).await;
        let mut transaction = pool.begin().await.unwrap();
        let group = Group::new("group1".to_string())
            .save(&mut *transaction)
            .await
            .unwrap();
        network
            .set_allowed_groups(&mut transaction, vec![group.name])
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        let user2_pre_sync = make_test_user_and_device("user2", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        do_directory_sync(&pool, &wg_tx).await.unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 3);
        let user2 = get_test_user(&pool, "user2").await;
        assert!(user2.is_none());
        let mut transaction = pool.begin().await.unwrap();
        user.sync_allowed_devices(&mut transaction, &wg_tx)
            .await
            .unwrap();
        transaction.commit().await.unwrap();
        let event = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceDeleted(dev)) = event {
            assert_eq!(dev.device.user_id, user2_pre_sync.id);
        } else {
            panic!("Expected a DeviceDeleted event");
        }
        let event = wg_rx.try_recv();
        if let Ok(GatewayEvent::DeviceCreated(dev)) = event {
            assert_eq!(dev.device.user_id, user.id);
        } else {
            panic!("Expected a DeviceDeleted event");
        }
    }

    #[sqlx::test]
    async fn test_sync_target_groups(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::Groups,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();
        let user = make_test_user_and_device("testuser", &pool).await;
        make_test_user_and_device("user2", &pool).await;
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 0);
        do_directory_sync(&pool, &wg_tx).await.unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 3);
        let user2 = get_test_user(&pool, "user2").await;
        assert!(user2.is_some());
    }

    #[sqlx::test]
    async fn test_sync_unassign_last_admin_group(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        // Make one admin and check if he's deleted
        let user = make_test_user_and_device("testuser", &pool).await;
        let admin_grp = Group::find_by_name(&pool, "admin").await.unwrap().unwrap();
        user.add_to_group(&pool, &admin_grp).await.unwrap();
        let user_groups = user.member_of(&pool).await.unwrap();
        assert_eq!(user_groups.len(), 1);
        assert!(user.is_admin(&pool).await.unwrap());

        do_directory_sync(&pool, &wg_tx).await.unwrap();

        // He should still be an admin as it's the last one
        assert!(user.is_admin(&pool).await.unwrap());

        // Make another admin and check if one of them is deleted
        let user2 = make_test_user_and_device("testuser2", &pool).await;
        user2.add_to_group(&pool, &admin_grp).await.unwrap();

        do_directory_sync(&pool, &wg_tx).await.unwrap();

        let admins = User::find_admins(&pool).await.unwrap();
        // There should be only one admin left
        assert_eq!(admins.len(), 1);

        let defguard_user = make_test_user_and_device("defguard", &pool).await;
        make_admin(&pool, &defguard_user).await;

        do_directory_sync(&pool, &wg_tx).await.unwrap();
    }

    #[sqlx::test]
    async fn test_sync_delete_last_admin_user(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let config = DefGuardConfig::new_test_config();
        let _ = SERVER_CONFIG.set(config.clone());
        let (wg_tx, _) = broadcast::channel::<GatewayEvent>(16);
        make_test_provider(
            &pool,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncUserBehavior::Delete,
            DirectorySyncTarget::All,
        )
        .await;
        let mut client = DirectorySyncClient::build(&pool).await.unwrap();
        client.prepare().await.unwrap();

        // a user that's not in the directory
        let defguard_user = make_test_user_and_device("defguard", &pool).await;
        make_admin(&pool, &defguard_user).await;
        assert!(defguard_user.is_admin(&pool).await.unwrap());

        do_directory_sync(&pool, &wg_tx).await.unwrap();

        // The user should still be an admin
        assert!(defguard_user.is_admin(&pool).await.unwrap());

        // remove his admin status
        let admin_grp = Group::find_by_name(&pool, "admin").await.unwrap().unwrap();
        defguard_user
            .remove_from_group(&pool, &admin_grp)
            .await
            .unwrap();

        do_directory_sync(&pool, &wg_tx).await.unwrap();
        let user = User::find_by_username(&pool, "defguard").await.unwrap();
        assert!(user.is_none());
    }
}
