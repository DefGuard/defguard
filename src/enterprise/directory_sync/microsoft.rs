use chrono::{TimeDelta, Utc};
use reqwest::{header::AUTHORIZATION, Url};
use std::time::Duration;

use crate::{db::Settings, enterprise::db::models::openid_provider::OpenIdProvider};
use serde::Deserialize;

use super::{parse_response, DirectoryGroup, DirectorySync, DirectorySyncError, DirectoryUser};

#[allow(dead_code)]
pub(crate) struct MicrosoftDirectorySync {
    access_token: Option<String>,
    token_expiry: Option<chrono::DateTime<Utc>>,
    client_id: String,
    client_secret: String,
    url: String,
}

const ACCESS_TOKEN_URL: &str = "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token";
const GROUPS_URL: &str = "https://graph.microsoft.com/v1.0/groups";
const USER_GROUPS: &str = "https://graph.microsoft.com/v1.0/users/{user_id}/memberOf";
const GROUP_MEMBERS: &str = "https://graph.microsoft.com/v1.0/groups/{group_id}/members";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const ALL_USERS_URL: &str = "https://graph.microsoft.com/v1.0/users";

#[derive(Deserialize)]
struct TokenResponse {
    #[serde(rename = "access_token")]
    token: String,
    expires_in: i64,
}

#[derive(Deserialize)]
struct GroupDetails {
    #[serde(rename = "displayName")]
    display_name: String,
    id: String,
}

#[derive(Deserialize)]
struct GroupsResponse {
    value: Option<Vec<GroupDetails>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupMember {
    #[serde(rename = "displayName")]
    display_name: String,
    mail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupMembersResponse {
    value: Option<Vec<GroupMember>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "displayName")]
    display_name: String,
    mail: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UsersResponse {
    value: Vec<User>,
}

impl MicrosoftDirectorySync {
    pub(crate) fn new(client_id: String, client_secret: String, url: String) -> Self {
        Self {
            access_token: None,
            client_id,
            client_secret,
            url,
            token_expiry: None,
        }
    }

    fn extract_tenant(&self) -> Result<String, DirectorySyncError> {
        debug!("Extracting tenant ID from Microsoft base URL: {}", self.url);
        let parts: Vec<&str> = self.url.split('/').collect();
        debug!(
            "Split Microsoft base URL into the following parts: {:?}",
            parts
        );
        let tenant_id =
            parts
                .get(parts.len() - 2)
                .ok_or(DirectorySyncError::InvalidProviderConfiguration(format!(
                    "Couldn't extrat tenant ID from the provided Microsoft base url: {}",
                    self.url
                )))?;
        debug!("Tenant ID extracted successfully: {}", tenant_id);
        Ok(tenant_id.to_string())
    }

    async fn query_access_token(&self) -> Result<TokenResponse, DirectorySyncError> {
        let tenant_id = self.extract_tenant()?;
        let token_url = ACCESS_TOKEN_URL.replace("{tenant_id}", &tenant_id);
        let client = reqwest::Client::new();
        let response = client
            .post(&token_url)
            .form(&[
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("scope", &"https://graph.microsoft.com/.default".to_string()),
                ("grant_type", &"client_credentials".to_string()),
            ])
            .send()
            .await?;

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }

    async fn refresh_access_token(&mut self) -> Result<(), DirectorySyncError> {
        debug!("Refreshing Microsoft directory sync access token.");
        let token_response = self.query_access_token().await?;
        let expires_in = TimeDelta::seconds(token_response.expires_in);
        self.access_token = Some(token_response.token);
        self.token_expiry = Some(Utc::now() + expires_in);
        debug!("Microsoft directory sync access token refreshed.");
        Ok(())
    }

    async fn query_groups(&self) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let client = reqwest::Client::new();
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let response = client
            .get(GROUPS_URL)
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to query all Microsoft groups.").await
    }

    fn is_token_expired(&self) -> bool {
        debug!(
            "Checking if Microsoft directory sync token is expired, expiry date: {:?}",
            self.token_expiry
        );
        let result = self.token_expiry.map_or(true, |expiry| expiry < Utc::now());
        debug!("Token expiry check result: {}", result);
        result
    }

    async fn query_user_groups(&self, user_id: &str) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let url = Url::parse(&USER_GROUPS.replace("{user_id}", user_id))
            .map_err(|err| DirectorySyncError::InvalidUrl(err.to_string()))?;
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to query user groups from Microsoft API.").await
    }

    async fn query_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<GroupMembersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;

        let url = Url::parse(&GROUP_MEMBERS.replace("{group_id}", &group.id))
            .map_err(|err| DirectorySyncError::InvalidUrl(err.to_string()))?;
        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(
            response,
            "Failed to query group members from Microsoft API.",
        )
        .await
    }

    async fn query_all_users(&self) -> Result<UsersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let url = Url::parse(ALL_USERS_URL)
            .map_err(|err| DirectorySyncError::InvalidUrl(err.to_string()))?;
        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to query all users in the Microsoft API.").await
    }

    async fn query_test_connection(&self) -> Result<(), DirectorySyncError> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let url = Url::parse(&format!("{ALL_USERS_URL}?$top=1"))
            .map_err(|err| DirectorySyncError::InvalidUrl(err.to_string()))?;
        let client = reqwest::Client::builder().build()?;
        let result = client
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        let _result: UsersResponse =
            parse_response(result, "Failed to test connection to Microsoft API.").await?;
        Ok(())
    }
}

impl DirectorySync for MicrosoftDirectorySync {
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Querying all groups from Microsoft API.");
        let groups = self
            .query_groups()
            .await?
            .value
            .unwrap_or_default()
            .into_iter()
            .map(|group| DirectoryGroup {
                id: group.id,
                name: group.display_name,
            });
        debug!("All groups queried successfully.");
        Ok(groups.collect())
    }

    async fn get_user_groups(
        &self,
        user_id: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Querying groups of user: {}", user_id);
        let groups = self
            .query_user_groups(user_id)
            .await?
            .value
            .unwrap_or_default()
            .into_iter()
            .map(|group| DirectoryGroup {
                id: group.id,
                name: group.display_name,
            });
        debug!("User groups queried successfully.");
        Ok(groups.collect())
    }

    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<String>, DirectorySyncError> {
        debug!("Querying members of group: {}", group.name);
        let members = self
            .query_group_members(group)
            .await?
            .value
            .unwrap_or_default()
            .into_iter()
            .filter_map(|user| {
                if let Some(email) = user.mail {
                    Some(email)
                } else {
                    warn!("User {} doesn't have an email address and will be skipped in synchronization.", user.display_name);
                    None
                }
            });
        debug!("Group members queried successfully.");
        Ok(members.collect())
    }

    async fn prepare(&mut self) -> Result<(), DirectorySyncError> {
        debug!("Preparing Microsoft directory sync...");
        if self.is_token_expired() {
            debug!("Access token is expired, refreshing.");
            self.refresh_access_token().await?;
            debug!("Access token refreshed.");
        } else {
            debug!("Access token is still valid, skipping refresh.");
        }
        debug!("Microsoft directory sync prepared.");
        Ok(())
    }

    async fn get_all_users(&self) -> Result<Vec<DirectoryUser>, DirectorySyncError> {
        debug!("Querying all users from Microsoft API.");
        let users = self
            .query_all_users()
            .await?
            .value
            .into_iter()
            .filter_map(|user| {
                if let Some(email) = user.mail {
                    Some(DirectoryUser { email, active: true })
                } else {
                    warn!("User {} doesn't have an email address and will be skipped in synchronization.", user.display_name);
                    None
                }
            });
        debug!("All users queried successfully.");
        Ok(users.collect())
    }

    async fn test_connection(&self) -> Result<(), DirectorySyncError> {
        debug!("Testing connection to Microsoft API.");
        self.query_test_connection().await?;
        info!("Successfully tested connection to Microsoft API, connection is working.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tenant() {
        let provider = MicrosoftDirectorySync::new(
            "client_id".to_string(),
            "client_secret".to_string(),
            "https://login.microsoftonline.com/tenant-id-123/v2.0".to_string(),
        );
        let tenant = provider.extract_tenant().unwrap();
        assert_eq!(tenant, "tenant-id-123");
    }
}
