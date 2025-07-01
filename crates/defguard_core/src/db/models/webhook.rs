use model_derive::Model;
use sqlx::{query_as, Error as SqlxError, FromRow, PgPool};

use super::UserInfo;
use crate::db::{Id, NoId};

/// App events which triggers webhook action
#[derive(Debug)]
pub enum AppEvent {
    UserCreated(UserInfo),
    UserModified(UserInfo),
    UserDeleted(String),
    HWKeyProvision(HWKeyUserData),
}
/// User data send on HWKeyProvision AppEvent
#[derive(Debug, Serialize)]
pub struct HWKeyUserData {
    pub username: String,
    pub email: String,
    pub ssh_key: String,
    pub pgp_key: String,
    pub serial: String,
}

impl AppEvent {
    // Debug name
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::UserCreated(_) => "user created",
            Self::UserModified(_) => "user modified",
            Self::UserDeleted(_) => "user deleted",
            Self::HWKeyProvision(_) => "hwkey provisioned",
        }
    }

    /// Database column name.
    #[must_use]
    pub fn column_name(&self) -> &str {
        match self {
            Self::UserCreated(_) => "on_user_created",
            Self::UserModified(_) => "on_user_modified",
            Self::UserDeleted(_) => "on_user_deleted",
            Self::HWKeyProvision(_) => "on_hwkey_provision",
        }
    }
}

#[derive(Clone, Debug, Deserialize, FromRow, Model, Serialize)]
pub struct WebHook<I = NoId> {
    pub id: I,
    pub url: String,
    pub description: String,
    pub token: String,
    pub enabled: bool,
    pub on_user_created: bool,
    pub on_user_deleted: bool,
    pub on_user_modified: bool,
    pub on_hwkey_provision: bool,
}

impl WebHook<Id> {
    /// Fetch all enabled webhooks.
    pub async fn all_enabled(pool: &PgPool, trigger: &AppEvent) -> Result<Vec<Self>, SqlxError> {
        let column_name = trigger.column_name();
        let query = format!(
            "SELECT id, url, description, token, enabled, on_user_created, \
            on_user_deleted, on_user_modified, on_hwkey_provision FROM webhook \
            WHERE enabled AND {column_name}"
        );
        query_as(&query).fetch_all(pool).await
    }

    /// Find [`WebHook`] by URL.
    pub async fn find_by_url(pool: &PgPool, url: &str) -> Result<Option<Self>, SqlxError> {
        query_as!(
            Self,
            "SELECT id, url, description, token, enabled, on_user_created, \
            on_user_deleted, on_user_modified, on_hwkey_provision FROM webhook WHERE url = $1",
            url
        )
        .fetch_optional(pool)
        .await
    }
}
