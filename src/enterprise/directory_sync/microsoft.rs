use chrono::{TimeDelta, Utc};
use serde::Deserialize;
use tokio::time::sleep;

use super::{
    make_get_request, parse_response, DirectoryGroup, DirectorySync, DirectorySyncError,
    DirectoryUser, REQUEST_PAGINATION_SLOWDOWN,
};
use crate::enterprise::directory_sync::REQUEST_TIMEOUT;

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
const ALL_USERS_URL: &str = "https://graph.microsoft.com/v1.0/users";
const MICROSOFT_DEFAULT_SCOPE: &str = "https://graph.microsoft.com/.default";
const GRANT_TYPE: &str = "client_credentials";
const MAX_RESULTS: &str = "200";
const MAX_REQUESTS: usize = 50;
const USER_QUERY_FIELDS: &str = "accountEnabled,displayName,mail,otherMails";

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

#[derive(Deserialize, Default)]
struct GroupsResponse {
    #[serde(rename = "@odata.nextLink")]
    next_page: Option<String>,
    value: Vec<GroupDetails>,
}

impl From<GroupsResponse> for Vec<DirectoryGroup> {
    fn from(response: GroupsResponse) -> Self {
        response
            .value
            .into_iter()
            .map(|group| DirectoryGroup {
                id: group.id,
                name: group.display_name,
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct GroupMembersResponse {
    #[serde(rename = "@odata.nextLink")]
    next_page: Option<String>,
    value: Vec<User>,
}

impl From<GroupMembersResponse> for Vec<String> {
    fn from(response: GroupMembersResponse) -> Self {
        response
            .value
            .into_iter()
            .filter_map(|user| {
                if let Some(email) = user.mail {
                    Some(email)
                } else if let Some(email) = user.other_mails.into_iter().next() {
                    warn!("User {} doesn't have a primary email address set, his first additional email address will be used: {email}", user.display_name);
                    Some(email)
                } else {
                    warn!("User {} doesn't have any email address and will be skipped in synchronization.", user.display_name);
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "displayName")]
    display_name: String,
    mail: Option<String>,
    #[serde(rename = "accountEnabled")]
    account_enabled: bool,
    #[serde(rename = "otherMails")]
    other_mails: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct UsersResponse {
    #[serde(rename = "@odata.nextLink")]
    next_page: Option<String>,
    value: Vec<User>,
}

impl From<UsersResponse> for Vec<DirectoryUser> {
    fn from(response: UsersResponse) -> Self {
        response
            .value
            .into_iter()
            .filter_map(|user| {
                if let Some(email) = user.mail {
                    Some(DirectoryUser { email, active: user.account_enabled })
                } else if let Some(email) = user.other_mails.into_iter().next() {
                    warn!("User {} doesn't have a primary email address set, his first additional email address will be used: {email}", user.display_name);
                    Some(DirectoryUser { email, active: user.account_enabled })
                } else {
                    warn!("User {} doesn't have any email address and will be skipped in synchronization.", user.display_name);
                    None
                }
            })
            .collect()
    }
}

impl MicrosoftDirectorySync {
    pub(crate) const fn new(client_id: String, client_secret: String, url: String) -> Self {
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
        debug!("Split Microsoft base URL into the following parts: {parts:?}",);
        let tenant_id =
            parts
                .get(parts.len() - 2)
                .ok_or(DirectorySyncError::InvalidProviderConfiguration(format!(
                    "Couldn't extract tenant ID from the provided Microsoft API base URL: {}",
                    self.url
                )))?;
        debug!("Tenant ID extracted successfully: {tenant_id}",);
        Ok(tenant_id.to_string())
    }

    async fn refresh_access_token(&mut self) -> Result<(), DirectorySyncError> {
        debug!("Refreshing Microsoft directory sync access token.");
        let token_response = self.query_access_token().await?;
        let expires_in = TimeDelta::seconds(token_response.expires_in);
        self.access_token = Some(token_response.token);
        self.token_expiry = Some(Utc::now() + expires_in);
        debug!(
            "Microsoft directory sync access token refreshed, the new token expires at: {:?}",
            self.token_expiry
        );
        Ok(())
    }

    fn is_token_expired(&self) -> bool {
        debug!(
            "Checking if Microsoft directory sync token is expired, expiry date: {:?}",
            self.token_expiry
        );
        let result = self.token_expiry.map_or(true, |expiry| expiry < Utc::now());
        debug!("Token expiry check result: {result}");
        result
    }

    async fn query_test_connection(&self) -> Result<(), DirectorySyncError> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let response = make_get_request(
            ALL_USERS_URL,
            access_token,
            Some(&[("$top", "1"), ("$select", USER_QUERY_FIELDS)]),
        )
        .await?;
        let _result: UsersResponse =
            parse_response(response, "Failed to test connection to Microsoft API.").await?;
        Ok(())
    }

    async fn query_access_token(&self) -> Result<TokenResponse, DirectorySyncError> {
        debug!("Querying Microsoft directory sync access token.");
        let tenant_id = self.extract_tenant()?;
        let token_url = ACCESS_TOKEN_URL.replace("{tenant_id}", &tenant_id);
        let client = reqwest::Client::new();
        let response = client
            .post(&token_url)
            .form(&[
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
                ("scope", &MICROSOFT_DEFAULT_SCOPE.to_string()),
                ("grant_type", &GRANT_TYPE.to_string()),
            ])
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        let token_response: TokenResponse = response.json().await?;
        debug!("Microsoft directory sync access token queried successfully.");
        Ok(token_response)
    }

    async fn query_groups(&self) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            debug!("Microsoft directory sync access token is expired, aborting group query.");
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut combined_response = GroupsResponse::default();
        let mut url = GROUPS_URL.to_string();
        let mut query = Some([("$top", MAX_RESULTS)].as_slice());

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(&url, access_token, query).await?;
            let response: GroupsResponse =
                parse_response(response, "Failed to query Microsoft groups.").await?;
            combined_response.value.extend(response.value);

            if let Some(next_page) = response.next_page {
                url = next_page;
                query = None;
                debug!("Found next page of results, querying it: {url}");
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        Ok(combined_response)
    }

    async fn query_user_groups(&self, user_id: &str) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            debug!(
                "Microsoft directory sync access token is expired, aborting query of user groups."
            );
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = USER_GROUPS.replace("{user_id}", user_id);
        let mut combined_response = GroupsResponse::default();
        let mut query = Some([("$top", MAX_RESULTS)].as_slice());

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(&url, access_token, query).await?;
            let response: GroupsResponse =
                parse_response(response, "Failed to query user groups from Microsoft API.").await?;
            combined_response.value.extend(response.value);

            if let Some(next_page) = response.next_page {
                url = next_page;
                query = None;
                debug!("Found next page of results, querying it: {url}");
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        Ok(combined_response)
    }

    async fn query_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<GroupMembersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            debug!(
                "Microsoft directory sync access token is expired, aborting group member query."
            );
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut combined_response = GroupMembersResponse::default();
        let mut url = GROUP_MEMBERS.replace("{group_id}", &group.id);
        let mut query = Some([("$top", MAX_RESULTS), ("$select", USER_QUERY_FIELDS)].as_slice());

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(&url, access_token, query).await?;
            let response: GroupMembersResponse = parse_response(
                response,
                "Failed to query group members from Microsoft API.",
            )
            .await?;
            combined_response.value.extend(response.value);

            if let Some(next_page) = response.next_page {
                url = next_page;
                query = None;
                debug!("Found next page of results, querying it: {url}");
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        Ok(combined_response)
    }

    async fn query_all_users(&self) -> Result<UsersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            debug!("Microsoft directory sync access token is expired, aborting all users query.");
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut combined_response = UsersResponse::default();
        let mut url = ALL_USERS_URL.to_string();
        let mut query = Some([("$top", MAX_RESULTS), ("$select", USER_QUERY_FIELDS)].as_slice());

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(&url, access_token, query).await?;
            let response: UsersResponse =
                parse_response(response, "Failed to query all users in the Microsoft API.").await?;
            combined_response.value.extend(response.value);

            if let Some(next_page) = response.next_page {
                url = next_page;
                query = None;
                debug!("Found next page of results, querying it: {url}");
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        Ok(combined_response)
    }
}

impl DirectorySync for MicrosoftDirectorySync {
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Querying all groups from Microsoft API.");
        let groups = self.query_groups().await?;
        debug!("All groups queried successfully.");
        Ok(groups.into())
    }

    async fn get_user_groups(
        &self,
        user_id: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Querying groups of user: {user_id}");
        let groups = self.query_user_groups(user_id).await?;
        debug!("User groups queried successfully.");
        Ok(groups.into())
    }

    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<String>, DirectorySyncError> {
        debug!("Querying members of group: {}", group.name);
        let members = self.query_group_members(group).await?;
        debug!("Group members queried successfully.");
        Ok(members.into())
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
        let users = self.query_all_users().await?;
        debug!("All users queried successfully.");
        Ok(users.into())
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

    #[tokio::test]
    async fn test_token() {
        let mut dirsync = MicrosoftDirectorySync::new(
            "id".to_string(),
            "secret".to_string(),
            "https://login.microsoftonline.com/tenant-id-123/v2.0".to_string(),
        );

        // no token
        assert!(dirsync.is_token_expired());

        // expired token
        dirsync.access_token = Some("test_token".into());
        dirsync.token_expiry = Some(Utc::now() - TimeDelta::seconds(10000));
        assert!(dirsync.is_token_expired());

        // valid token
        dirsync.access_token = Some("test_token".into());
        dirsync.token_expiry = Some(Utc::now() + TimeDelta::seconds(10000));
        assert!(!dirsync.is_token_expired());
    }

    #[tokio::test]
    async fn test_groups_parse() {
        let groups_response = GroupsResponse {
            next_page: None,
            value: vec![
                GroupDetails {
                    display_name: "Group 1".to_string(),
                    id: "1".to_string(),
                },
                GroupDetails {
                    display_name: "Group 2".to_string(),
                    id: "2".to_string(),
                },
            ],
        };

        let groups: Vec<DirectoryGroup> = groups_response.into();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Group 1");
        assert_eq!(groups[0].id, "1");
        assert_eq!(groups[1].name, "Group 2");
        assert_eq!(groups[1].id, "2");
    }

    #[tokio::test]
    async fn test_members_parse() {
        let members_response = GroupMembersResponse {
            next_page: None,
            value: vec![
                User {
                    display_name: "User 1".to_string(),
                    mail: Some("email@email.com".to_string()),
                    account_enabled: true,
                    other_mails: vec![],
                },
                User {
                    display_name: "User 2".to_string(),
                    mail: None,
                    account_enabled: true,
                    other_mails: vec!["email2@email.com".to_string()],
                },
                User {
                    display_name: "User 3".to_string(),
                    mail: None,
                    account_enabled: true,
                    other_mails: vec![],
                },
            ],
        };

        let members: Vec<String> = members_response.into();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0], "email@email.com".to_string());
        assert_eq!(members[1], "email2@email.com".to_string());
    }

    #[tokio::test]
    async fn test_users_parse() {
        let users_response = UsersResponse {
            next_page: None,
            value: vec![
                User {
                    display_name: "User 1".to_string(),
                    mail: Some("email@email.com".to_string()),
                    account_enabled: true,
                    other_mails: vec![],
                },
                User {
                    display_name: "User 2".to_string(),
                    mail: None,
                    account_enabled: true,
                    other_mails: vec!["email2@email.com".to_string()],
                },
                User {
                    display_name: "User 3".to_string(),
                    mail: None,
                    account_enabled: true,
                    other_mails: vec![],
                },
            ],
        };

        let users: Vec<DirectoryUser> = users_response.into();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].email, "email@email.com".to_string());
        assert_eq!(users[1].email, "email2@email.com".to_string());
    }
}
