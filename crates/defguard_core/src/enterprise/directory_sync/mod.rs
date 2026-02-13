use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    time::Duration,
};

use defguard_common::db::{
    Id,
    models::{Settings, group::Group, user::User},
};
use paste::paste;
use reqwest::header::AUTHORIZATION;
use sqlx::{PgConnection, PgPool, error::Error as SqlxError};
use thiserror::Error;
use tokio::sync::broadcast::Sender;

use super::{
    db::models::openid_provider::{DirectorySyncTarget, OpenIdProvider},
    ldap::utils::ldap_update_users_state,
};
#[cfg(not(test))]
use crate::enterprise::is_business_license_active;
use crate::{
    enterprise::{
        db::models::openid_provider::DirectorySyncUserBehavior,
        handlers::openid_login::prune_username,
        ldap::{
            model::ldap_sync_allowed_for_user,
            utils::{ldap_add_users_to_groups, ldap_delete_users, ldap_remove_users_from_groups},
        },
    },
    grpc::GatewayEvent,
    handlers::user::check_username,
    user_management::{delete_user_and_cleanup_devices, disable_user, sync_allowed_user_devices},
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_PAGINATION_SLOWDOWN: Duration = Duration::from_millis(100);

#[derive(Debug, Error)]
pub enum DirectorySyncError {
    #[error("Database error: {0}")]
    DbError(#[from] SqlxError),
    #[error(
        "Access token has expired or is not present. An issue may have occured while trying to obtain a new one."
    )]
    AccessTokenExpired,
    #[error("Processing a request to the provider's API failed: {0}")]
    RequestError(String),
    #[error("Failed to build a JWT token, required for communicating with the provider's API: {0}")]
    JWTError(#[from] jsonwebtoken::errors::Error),
    #[error("The selected provider {0} is not supported for directory sync")]
    UnsupportedProvider(String),
    #[error("Directory sync is not configured")]
    NotConfigured,
    #[error(
        "Couldn't map provider's group to a Defguard group as it doesn't exist. There may be an issue with automatic group creation. Error details: {0}"
    )]
    DefGuardGroupNotFound(String),
    #[error("The provided provider configuration is invalid: {0}")]
    InvalidProviderConfiguration(String),
    #[error("Couldn't construct URL from the given string: {0}")]
    InvalidUrl(String),
    #[error("Failed to update network state: {0}")]
    NetworkUpdateError(String),
    #[error("Failed to update user state: {0}")]
    UserUpdateError(String),
    #[error("Failed to create user: {0}")]
    UserCreateError(String),
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
            Self::RequestError(format!(
                "There was an error while trying to decode provider's response, it may be malformed: {err}"
            ))
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
pub mod jumpcloud;
pub mod microsoft;
pub mod okta;
#[cfg(test)]
pub mod testprovider;
#[cfg(test)]
pub mod tests;

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryGroup {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryUser {
    pub id: Option<String>,
    pub email: String,
    // Users may be disabled/suspended in the directory
    pub active: bool,
    // Currently only supported for Microsoft Entra
    user_details: Option<DirectoryUserDetails>,
}

// additional user details required for user creation
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryUserDetails {
    last_name: String,
    first_name: String,
    phone_number: Option<String>,
}

#[trait_variant::make(Send)]
#[trait_variant::make(Sync)]
trait DirectorySync {
    /// Get all groups in a directory
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError>;

    /// Get all groups a user is a member of
    async fn get_user_groups(
        &self,
        user_email: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError>;

    /// Get all members of a group, returns a list of user emails
    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
        // Some providers (JumpCloud) doesn't return emails of group members, just ids.
        // In such cases, we can use the list of all users in the directory to map ids to emails.
        all_users_helper: Option<&[DirectoryUser]>,
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

            async fn get_user_groups(&self, user_email: &str) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
                match self {
                    $(
                        DirectorySyncClient::$variant(client) => client.get_user_groups(user_email).await,
                    )*
                }
            }

            async fn get_group_members(&self, group: &DirectoryGroup,
                all_users_helper: Option<&[DirectoryUser]>,
            ) -> Result<Vec<String>, DirectorySyncError> {
                match self {
                    $(
                        DirectorySyncClient::$variant(client) => client.get_group_members(group, all_users_helper).await,
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
dirsync_clients!(Google, Microsoft, Okta, TestProvider, JumpCloud);

#[cfg(not(test))]
dirsync_clients!(Google, Microsoft, Okta, JumpCloud);

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
                        debug!(
                            "Google directory has all the configuration needed, proceeding with \
                            creating the sync client"
                        );
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
                    debug!(
                        "Okta directory has all the configuration needed, proceeding with creating \
                        the sync client"
                    );
                    let client =
                        okta::OktaDirectorySync::new(jwk, client_id, &provider_settings.base_url);
                    debug!("Okta directory sync client created");
                    Ok(Self::Okta(client))
                } else {
                    Err(DirectorySyncError::InvalidProviderConfiguration(
                        "Okta provider is not configured correctly for Directory Sync. Okta \
                            private key or client id is missing."
                            .to_string(),
                    ))
                }
            }
            "JumpCloud" => {
                debug!("JumpCloud directory sync provider selected");
                if let Some(key) = provider_settings.jumpcloud_api_key.as_ref() {
                    debug!(
                        "JumpCloud directory has all the configuration needed, proceeding with \
                        creating the sync client"
                    );
                    let client = jumpcloud::JumpCloudDirectorySync::new(key.clone());
                    debug!("JumpCloud directory sync client created");
                    Ok(Self::JumpCloud(client))
                } else {
                    Err(DirectorySyncError::NotConfigured)
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

    sync_allowed_user_devices(user, &mut transaction, wg_tx)
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
    if !is_business_license_active() {
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
pub async fn sync_user_groups_if_configured(
    user: &User<Id>,
    pool: &PgPool,
    wg_tx: &Sender<GatewayEvent>,
) -> Result<(), DirectorySyncError> {
    #[cfg(not(test))]
    if !is_business_license_active() {
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
        "Creating group {} if it doesn't exist and adding user {group_name} to it if they are not \
        already a member",
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
    all_users: Option<&[DirectoryUser]>,
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
        match directory_sync.get_group_members(group, all_users).await {
            Ok(members) => {
                debug!(
                    "Group {} has {} members in the directory, adding them to the user-group \
                    mapping",
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
                            "User {} is the last admin in the system, can't remove them from an \
                            admin group {}",
                            user.email, current_group.name
                        );
                        continue;
                    }
                    debug!(
                        "Removing user {} from group {} as they are not a member of it in the \
                        directory",
                        user.email, current_group.name
                    );
                    user.remove_from_group(&mut *transaction, current_group)
                        .await?;
                    admin_count -= 1;
                } else {
                    debug!(
                        "Removing user {} from group {} as they are not a member of it in the \
                        directory",
                        user.email, current_group.name
                    );
                    user.remove_from_group(&mut *transaction, current_group)
                        .await?;
                }
            }
        }

        for group in groups {
            create_and_add_to_group(&user, group, pool).await?;
        }

        sync_allowed_user_devices(&user, &mut transaction, wg_tx).await.map_err(|err| {
            DirectorySyncError::NetworkUpdateError(format!(
                "Failed to sync allowed devices for user {} during directory synchronization: {err}",
                user.email
            ))
        })?;

        affected_users.push(user);
    }
    transaction.commit().await?;

    Box::pin(ldap_update_users_state(
        affected_users.iter_mut().collect::<Vec<_>>(),
        pool,
    ))
    .await;
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

async fn sync_all_users_state(
    pool: &PgPool,
    wg_tx: &Sender<GatewayEvent>,
    all_users: &[DirectoryUser],
) -> Result<(), DirectorySyncError> {
    info!("Syncing all users' state with the directory, this may take a while...");
    let mut transaction = pool.begin().await?;
    let settings = OpenIdProvider::get_current(pool)
        .await?
        .ok_or(DirectorySyncError::NotConfigured)?;

    // prepare relevant settings
    let user_behavior = settings.directory_sync_user_behavior;
    let admin_behavior = settings.directory_sync_admin_behavior;
    let prefetch_users = settings.prefetch_users;

    // split directory users into separate lists for active and inactive users
    let (active_directory_users, inactive_directory_users): (Vec<_>, Vec<_>) =
        all_users.iter().partition(|user| user.active);

    // prepare a list of user emails for matching users between directory and Defguard
    let all_directory_emails = all_users
        .iter()
        .map(|u| u.email.as_str())
        .collect::<Vec<&str>>();

    // setup Vecs for tracking user updates
    let mut modified_users = Vec::new();
    let mut deleted_users = Vec::new();
    let mut created_users = Vec::new();

    sync_inactive_directory_users(
        &mut transaction,
        &inactive_directory_users,
        &mut modified_users,
        wg_tx,
    )
    .await?;

    sync_active_directory_users(
        &mut transaction,
        &active_directory_users,
        &mut modified_users,
    )
    .await?;

    // TODO: prefetching users is currently only supported for Microsoft Entra
    if prefetch_users && ["Microsoft", "Test"].contains(&settings.name.as_str()) {
        // get emails of all directory users who already exist in Defguard
        let existing_users =
            User::find_many_by_emails(&mut *transaction, &all_directory_emails).await?;
        let existing_user_emails: Vec<&str> = existing_users
            .iter()
            .map(|user| user.email.as_str())
            .collect();

        // find all directory users not present in Defguard
        let missing_defguard_users: Vec<_> = all_users
            .iter()
            .filter(|user| !existing_user_emails.contains(&user.email.as_str()))
            .collect();

        let core_settings = Settings::get_current_settings();

        // create missing users
        for directory_user in missing_defguard_users {
            match &directory_user.user_details {
                None => {
                    error!(
                        "Missing directory user details for user {directory_user:?}. Unable to \
                        create missing Defguard user."
                    );
                }
                Some(details) => {
                    debug!(
                        "User {directory_user:?} exists in directory but not in Defguard. Creating \
                        new Defguard user.",
                    );

                    // Extract the username from the email address
                    let email = directory_user.email.clone();
                    let username =
                        email
                            .split('@')
                            .next()
                            .ok_or(DirectorySyncError::UserCreateError(format!(
                                "Failed to extract username from email address {email}"
                            )))?;
                    let username = prune_username(username, core_settings.openid_username_handling);
                    check_username(&username).map_err(|err| {
                        DirectorySyncError::UserCreateError(format!(
                            "Username {username} validation failed: {err:?}"
                        ))
                    })?;

                    // Check if user with the same username already exists (usernames are unique).
                    if User::find_by_username(pool, &username).await?.is_some() {
                        return Err(DirectorySyncError::UserCreateError(format!(
                            "User with username {username} already exists"
                        )));
                    }

                    let mut user = User::new(
                        username,
                        None,
                        details.last_name.clone(),
                        details.first_name.clone(),
                        directory_user.email.clone(),
                        details.phone_number.clone(),
                    );
                    user.openid_sub.clone_from(&directory_user.id);
                    let new_user = user.save(&mut *transaction).await?;
                    created_users.push(new_user);
                }
            }
        }
    }

    // get all users present in Defguard but not in directory
    let missing_directory_users = User::exclude(&mut *transaction, &all_directory_emails)
        .await?
        .into_iter()
        .collect::<Vec<User<Id>>>();

    debug!(
        "There are {} users missing from the directory but present in Defguard, deciding what to \
        do next based on the following settings: user action: {user_behavior}, admin action: \
        {admin_behavior}",
        missing_directory_users.len(),
    );
    // Keep the admin count to prevent deleting the last admin
    let mut admin_count = User::find_admins(&mut *transaction).await?.len();
    for mut user in missing_directory_users {
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
                        disable_user(&mut user, &mut transaction, wg_tx).await.map_err(|err| {
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
                    if ldap_sync_allowed_for_user(&user, &mut *transaction).await? {
                        deleted_users.push(user.clone().as_noid());
                    }
                    delete_user_and_cleanup_devices(user, &mut transaction, wg_tx)
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
                            "Disabling user {} because they are not present in the directory and \
                            the user behavior setting is set to disable",
                            user.email
                        );
                        disable_user(&mut user, &mut transaction, wg_tx).await.map_err(|err| {
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
                    if ldap_sync_allowed_for_user(&user, &mut *transaction).await? {
                        deleted_users.push(user.clone().as_noid());
                    }
                    delete_user_and_cleanup_devices(user, &mut transaction, wg_tx)
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

    transaction.commit().await?;

    // trigger LDAP sync
    ldap_delete_users(deleted_users.iter().collect::<Vec<_>>(), pool).await;
    Box::pin(ldap_update_users_state(
        modified_users.iter_mut().collect::<Vec<_>>(),
        pool,
    ))
    .await;
    Box::pin(ldap_update_users_state(
        created_users.iter_mut().collect::<Vec<_>>(),
        pool,
    ))
    .await;

    info!("Syncing all users' state with the directory done");

    Ok(())
}

async fn sync_inactive_directory_users(
    transaction: &mut PgConnection,
    inactive_directory_users: &[&DirectoryUser],
    modified_users: &mut Vec<User<Id>>,
    wg_tx: &Sender<GatewayEvent>,
) -> Result<(), DirectorySyncError> {
    // find all active Defguard users disabled in directory
    let disabled_users_emails = inactive_directory_users
        .iter()
        .map(|u| u.email.as_str())
        .collect::<Vec<&str>>();
    let users_to_disable: Vec<User<Id>> =
        User::find_many_by_emails(&mut *transaction, &disabled_users_emails)
            .await?
            .into_iter()
            .filter(|user| user.is_active)
            .collect();

    debug!(
        "There are {} active Defguard users disabled in the directory. Disabling them in Defguard.",
        users_to_disable.len()
    );

    for mut user in users_to_disable {
        if user.is_active {
            debug!(
                "Disabling user {} because they are disabled in the directory",
                user.email
            );
            disable_user(&mut user, transaction, wg_tx)
                .await
                .map_err(|err| {
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
    debug!("Done processing disabled directory users");

    Ok(())
}

async fn sync_active_directory_users(
    transaction: &mut PgConnection,
    active_directory_users: &[&DirectoryUser],
    modified_users: &mut Vec<User<Id>>,
) -> Result<(), DirectorySyncError> {
    // find all inactive Defguard users enabled in directory
    let enabled_users_emails = active_directory_users
        .iter()
        .map(|u| u.email.as_str())
        .collect::<Vec<&str>>();
    let users_to_enable: Vec<User<Id>> =
        User::find_many_by_emails(&mut *transaction, &enabled_users_emails)
            .await?
            .into_iter()
            .filter(|user| !user.is_active)
            .collect();

    debug!(
        "There are {} inactive Defguard users enabled in the directory. Enabling them in Defguard.",
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
    debug!("Done processing active directory users");

    Ok(())
}

// The default inverval for the directory sync job
const DIRECTORY_SYNC_INTERVAL: u64 = 60 * 10;

/// Used to inform the utility thread how often it should perform the directory sync job.
/// See [`run_utility_thread`] for more details.
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
    if !is_business_license_active() {
        debug!("Enterprise is not enabled, skipping performing directory sync");
        return Ok(());
    }

    // TODO: Reduce the amount of times those settings are retrieved in the whole directory sync
    // process.
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
            // Same goes for Etags, those could be used to reduce the amount of data transferred.
            // Some way of preserving them should be implemented.
            dir_sync.prepare().await?;

            // This is an optimization, both sync_all_users_state and sync_all_users_groups depend
            // on it so we might as well get all users once and pass it to both functions.
            let mut all_users = None;

            if matches!(
                sync_target,
                DirectorySyncTarget::All | DirectorySyncTarget::Users
            ) {
                let users = dir_sync.get_all_users().await?;
                sync_all_users_state(pool, wireguard_tx, &users).await?;
                all_users = Some(users);
            }
            if matches!(
                sync_target,
                DirectorySyncTarget::All | DirectorySyncTarget::Groups
            ) {
                // Sometimes we don't even need to query all users, this is an optimization to
                // reduce the amount of data transferred.
                let users_to_pass = match dir_sync {
                    DirectorySyncClient::JumpCloud(_) => {
                        if all_users.is_none() {
                            // JumpCloud doesn't return emails of group members, so we need to pass
                            // all users to the get_user_groups method to map ids to emails.
                            Some(dir_sync.get_all_users().await?)
                        } else {
                            all_users
                        }
                    }
                    _ => None, // No need to pass all users for other providers, for the time being.
                };
                sync_all_users_groups(&dir_sync, pool, wireguard_tx, users_to_pass.as_deref())
                    .await?;
            }
        }
        Err(err) => {
            error!("Failed to build directory sync client: {err}");
        }
    }

    Ok(())
}

// Helpers shared between the directory sync providers

/// Parse a reqwest response and return the JSON body if the response is OK, otherwise map an error
/// to a DirectorySyncError::RequestError.
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
        .await;
    match response {
        Ok(response) => {
            if response.status().is_success() {
                Ok(response)
            } else {
                Err(DirectorySyncError::RequestError(format!(
                    "Failed to make GET request to {url}. Status code: {}. Details: {}",
                    response.status(),
                    response.text().await?
                )))
            }
        }
        Err(err) => Err(DirectorySyncError::RequestError(format!(
            "Failed to make GET request to {url}: {err}"
        ))),
    }
}
