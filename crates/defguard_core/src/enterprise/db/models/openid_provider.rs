use std::fmt;

use model_derive::Model;
use sqlx::{query, query_as, Error as SqlxError, PgPool, Type};

use crate::db::{Id, NoId};

// The behavior when a user is deleted from the directory
// Keep: Keep the user, despite being deleted from the external provider's directory
// Disable: Disable the user
// Delete: Delete the user
#[derive(Clone, Deserialize, Serialize, PartialEq, Type, Debug)]
#[sqlx(type_name = "dirsync_user_behavior", rename_all = "snake_case")]
pub enum DirectorySyncUserBehavior {
    Keep,
    Disable,
    Delete,
}

impl fmt::Display for DirectorySyncUserBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DirectorySyncUserBehavior::Keep => "keep",
                DirectorySyncUserBehavior::Disable => "disable",
                DirectorySyncUserBehavior::Delete => "delete",
            }
        )
    }
}

impl From<String> for DirectorySyncUserBehavior {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "keep" => DirectorySyncUserBehavior::Keep,
            "disable" => DirectorySyncUserBehavior::Disable,
            "delete" => DirectorySyncUserBehavior::Delete,
            _ => {
                warn!("Unknown directory sync user behavior passed: {}", s);
                DirectorySyncUserBehavior::Keep
            }
        }
    }
}

// What to sync from the directory
// All: Sync both users and groups
// Users: Sync only users and their state
// Groups: Sync only groups (members without their state)
#[derive(Clone, Deserialize, Serialize, PartialEq, Type, Debug)]
#[sqlx(type_name = "dirsync_target", rename_all = "snake_case")]
pub enum DirectorySyncTarget {
    All,
    Users,
    Groups,
}

impl fmt::Display for DirectorySyncTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                DirectorySyncTarget::All => "all",
                DirectorySyncTarget::Users => "users",
                DirectorySyncTarget::Groups => "groups",
            }
        )
    }
}

impl From<String> for DirectorySyncTarget {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "all" => DirectorySyncTarget::All,
            "users" => DirectorySyncTarget::Users,
            "groups" => DirectorySyncTarget::Groups,
            _ => {
                warn!("Unknown directory sync target passed: {}", s);
                DirectorySyncTarget::All
            }
        }
    }
}

#[derive(Clone, Deserialize, Model, Serialize)]
pub struct OpenIdProvider<I = NoId> {
    pub id: I,
    pub name: String,
    pub base_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub display_name: Option<String>,
    // Specific stuff for Google
    pub google_service_account_key: Option<String>,
    pub google_service_account_email: Option<String>,
    pub admin_email: Option<String>,
    pub directory_sync_enabled: bool,
    // How often to sync the directory in seconds
    pub directory_sync_interval: i32,
    #[model(enum)]
    pub directory_sync_user_behavior: DirectorySyncUserBehavior,
    #[model(enum)]
    pub directory_sync_admin_behavior: DirectorySyncUserBehavior,
    #[model(enum)]
    pub directory_sync_target: DirectorySyncTarget,
    // Specific stuff for Okta
    pub okta_private_jwk: Option<String>,
    // The client ID of the directory sync app specifically
    pub okta_dirsync_client_id: Option<String>,
    #[model(ref)]
    // The groups to sync from the directory, exact match
    pub directory_sync_group_match: Vec<String>,
}

impl OpenIdProvider {
    #[must_use]
    pub fn new<S: Into<String>>(
        name: S,
        base_url: S,
        client_id: S,
        client_secret: S,
        display_name: Option<String>,
        google_service_account_key: Option<String>,
        google_service_account_email: Option<String>,
        admin_email: Option<String>,
        directory_sync_enabled: bool,
        directory_sync_interval: i32,
        directory_sync_user_behavior: DirectorySyncUserBehavior,
        directory_sync_admin_behavior: DirectorySyncUserBehavior,
        directory_sync_target: DirectorySyncTarget,
        okta_private_jwk: Option<String>,
        okta_dirsync_client_id: Option<String>,
        directory_sync_group_match: Vec<String>,
    ) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            base_url: base_url.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            display_name,
            google_service_account_key,
            google_service_account_email,
            admin_email,
            directory_sync_enabled,
            directory_sync_interval,
            directory_sync_user_behavior,
            directory_sync_admin_behavior,
            directory_sync_target,
            okta_private_jwk,
            okta_dirsync_client_id,
            directory_sync_group_match,
        }
    }

    pub async fn upsert(self, pool: &PgPool) -> Result<OpenIdProvider<Id>, SqlxError> {
        if let Some(provider) = OpenIdProvider::<Id>::get_current(pool).await? {
            query!(
                "UPDATE openidprovider SET name = $1, \
                base_url = $2, client_id = $3, client_secret = $4, \
                display_name = $5, google_service_account_key = $6, google_service_account_email = $7, admin_email = $8, \
                directory_sync_enabled = $9, directory_sync_interval = $10, directory_sync_user_behavior = $11, \
                directory_sync_admin_behavior = $12, directory_sync_target = $13, \
                okta_private_jwk = $14, okta_dirsync_client_id = $15, directory_sync_group_match = $16 \
                WHERE id = $17",
                self.name,
                self.base_url,
                self.client_id,
                self.client_secret,
                self.display_name,
                self.google_service_account_key,
                self.google_service_account_email,
                self.admin_email,
                self.directory_sync_enabled,
                self.directory_sync_interval,
                self.directory_sync_user_behavior as DirectorySyncUserBehavior,
                self.directory_sync_admin_behavior as DirectorySyncUserBehavior,
                self.directory_sync_target as DirectorySyncTarget,
                self.okta_private_jwk,
                self.okta_dirsync_client_id,
                &self.directory_sync_group_match,
                provider.id,
            )
            .execute(pool)
            .await?;

            Ok(provider)
        } else {
            self.save(pool).await
        }
    }
}

impl OpenIdProvider<Id> {
    pub async fn find_by_name(pool: &PgPool, name: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id, name, base_url, client_id, client_secret, display_name, \
            google_service_account_key, google_service_account_email, admin_email, directory_sync_enabled,
            directory_sync_interval, directory_sync_user_behavior  \"directory_sync_user_behavior: DirectorySyncUserBehavior\", \
            directory_sync_admin_behavior  \"directory_sync_admin_behavior: DirectorySyncUserBehavior\", \
            directory_sync_target  \"directory_sync_target: DirectorySyncTarget\", \
            okta_private_jwk, okta_dirsync_client_id, directory_sync_group_match \
            FROM openidprovider WHERE name = $1",
            name
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn get_current(pool: &PgPool) -> Result<Option<Self>, SqlxError> {
        query_as!(
            OpenIdProvider,
            "SELECT id, name, base_url, client_id, client_secret, display_name, \
            google_service_account_key, google_service_account_email, admin_email, directory_sync_enabled, \
            directory_sync_interval, directory_sync_user_behavior \"directory_sync_user_behavior: DirectorySyncUserBehavior\", \
            directory_sync_admin_behavior  \"directory_sync_admin_behavior: DirectorySyncUserBehavior\", \
            directory_sync_target  \"directory_sync_target: DirectorySyncTarget\", \
            okta_private_jwk, okta_dirsync_client_id, directory_sync_group_match \
            FROM openidprovider LIMIT 1"
        )
        .fetch_optional(pool)
        .await
    }
}
