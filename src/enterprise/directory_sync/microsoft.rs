use std::time::Duration;

use chrono::{TimeDelta, Utc};
use reqwest::{header::AUTHORIZATION, Url};
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

#[cfg(not(test))]
const ACCESS_TOKEN_URL: &str = "https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token";
#[cfg(not(test))]
const GROUPS_URL: &str = "https://graph.microsoft.com/v1.0/groups?$top=999";
#[cfg(not(test))]
const USER_GROUPS: &str = "https://graph.microsoft.com/v1.0/users/{user_id}/memberOf?$top=999";
#[cfg(not(test))]
const GROUP_MEMBERS: &str = "https://graph.microsoft.com/v1.0/groups/{group_id}/members?$select=accountEnabled,displayName,mail,otherMails&$top=999";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const ALL_USERS_URL: &str =
    "https://graph.microsoft.com/v1.0/users?$select=accountEnabled,displayName,mail,otherMails&$top=999";
#[cfg(not(test))]
const MICROSOFT_DEFAULT_SCOPE: &str = "https://graph.microsoft.com/.default";

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
    value: Vec<GroupDetails>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupMembersResponse {
    value: Vec<User>,
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

#[derive(Debug, Serialize, Deserialize)]
struct UsersResponse {
    value: Vec<User>,
}

async fn make_get_request(
    url: Url,
    token: String,
) -> Result<reqwest::Response, DirectorySyncError> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await?;
    Ok(response)
}

#[cfg(not(test))]
impl MicrosoftDirectorySync {
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
                ("grant_type", &"client_credentials".to_string()),
            ])
            .send()
            .await?;
        let token_response: TokenResponse = response.json().await?;
        debug!("Microsoft directory sync access token queried successfully.");
        Ok(token_response)
    }

    async fn query_groups(&self) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let url = Url::parse(GROUPS_URL)
            .map_err(|err| DirectorySyncError::InvalidUrl(err.to_string()))?;
        let response = make_get_request(url, access_token.to_string()).await?;
        parse_response(response, "Failed to query all Microsoft groups.").await
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
        let response = make_get_request(url, access_token.to_string()).await?;
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
        let response = make_get_request(url, access_token.to_string()).await?;
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
        let response = make_get_request(url, access_token.to_string()).await?;
        parse_response(response, "Failed to query all users in the Microsoft API.").await
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
        debug!("Microsoft directory sync access token refreshed.");
        Ok(())
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

    async fn query_test_connection(&self) -> Result<(), DirectorySyncError> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let url = Url::parse(&format!("{ALL_USERS_URL}?$top=1"))
            .map_err(|err| DirectorySyncError::InvalidUrl(err.to_string()))?;
        let response = make_get_request(url, access_token.to_string()).await?;
        let _result: UsersResponse =
            parse_response(response, "Failed to test connection to Microsoft API.").await?;
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
        debug!("Querying groups of user: {user_id}");
        let groups = self
            .query_user_groups(user_id)
            .await?
            .value
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
                    Some(DirectoryUser { email, active: user.account_enabled })
                } else if let Some(mail) = user.other_mails.first() {
                    warn!("User {} doesn't have a primary email address set, his first additional email address will be used: {mail}", user.display_name);
                    Some(DirectoryUser { email: mail.clone(), active: user.account_enabled })
                } else {
                    warn!("User {} doesn't have any email address and will be skipped in synchronization.", user.display_name);
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
impl MicrosoftDirectorySync {
    async fn query_user_groups(
        &self,
        _user_id: &str,
    ) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let _access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;

        Ok(GroupsResponse {
            value: vec![GroupDetails {
                display_name: "group1".into(),
                id: "1".into(),
            }],
        })
    }

    async fn query_groups(&self) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        let _access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;

        Ok(GroupsResponse {
            value: vec![
                GroupDetails {
                    display_name: "group1".into(),
                    id: "1".into(),
                },
                GroupDetails {
                    display_name: "group2".into(),
                    id: "2".into(),
                },
                GroupDetails {
                    display_name: "group3".into(),
                    id: "3".into(),
                },
            ],
        })
    }

    async fn query_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<GroupMembersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let _access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;

        Ok(GroupMembersResponse {
            value: vec![
                User {
                    display_name: "testuser".into(),
                    mail: Some("testuser@email.com".into()),
                    account_enabled: true,
                    other_mails: vec![],
                },
                User {
                    display_name: "testuserdisabled".into(),
                    mail: Some("testuserdisabled@email.com".into()),
                    account_enabled: false,
                    other_mails: vec![],
                },
                User {
                    display_name: "testuser2".into(),
                    mail: Some(
                        "testuser2@email.com
                    "
                        .into(),
                    ),
                    account_enabled: true,
                    other_mails: vec![],
                },
            ],
        })
    }

    async fn query_access_token(&self) -> Result<TokenResponse, DirectorySyncError> {
        Ok(TokenResponse {
            token: "test_token_refreshed".into(),
            expires_in: 3600,
        })
    }

    async fn query_all_users(&self) -> Result<UsersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let _access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        Ok(UsersResponse {
            value: vec![
                User {
                    display_name: "testuser".into(),
                    mail: Some("testuser@email.com".into()),
                    account_enabled: true,
                    other_mails: vec![],
                },
                User {
                    display_name: "testuserdisabled".into(),
                    mail: Some("testuserdisabled@email.com".into()),
                    account_enabled: false,
                    other_mails: vec![],
                },
                User {
                    display_name: "testuser2".into(),
                    mail: Some("testuser2@email.com".into()),
                    account_enabled: true,
                    other_mails: vec![],
                },
            ],
        })
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

        // no token
        dirsync.access_token = Some("test_token".into());
        dirsync.token_expiry = Some(Utc::now() - TimeDelta::seconds(10000));
        dirsync.refresh_access_token().await.unwrap();
        assert!(!dirsync.is_token_expired());
        assert_eq!(dirsync.access_token, Some("test_token_refreshed".into()));
    }

    #[tokio::test]
    async fn test_all_users() {
        let mut dirsync = MicrosoftDirectorySync::new(
            "id".to_string(),
            "secret".to_string(),
            "https://login.microsoftonline.com/tenant-id-123/v2.0".to_string(),
        );
        dirsync.refresh_access_token().await.unwrap();

        let users = dirsync.get_all_users().await.unwrap();

        assert_eq!(users.len(), 3);
        assert_eq!(users[1].email, "testuserdisabled@email.com");
        assert!(!users[1].active);
    }

    #[tokio::test]
    async fn test_groups() {
        let mut dirsync = MicrosoftDirectorySync::new(
            "id".to_string(),
            "secret".to_string(),
            "https://login.microsoftonline.com/tenant-id-123/v2.0".to_string(),
        );
        dirsync.refresh_access_token().await.unwrap();

        let groups = dirsync.get_groups().await.unwrap();

        assert_eq!(groups.len(), 3);

        for (i, group) in groups.iter().enumerate().take(3) {
            assert_eq!(group.id, (i + 1).to_string());
            assert_eq!(group.name, format!("group{}", i + 1));
        }
    }

    #[tokio::test]
    async fn test_user_groups() {
        let mut dirsync = MicrosoftDirectorySync::new(
            "id".to_string(),
            "secret".to_string(),
            "https://login.microsoftonline.com/tenant-id-123/v2.0".to_string(),
        );
        dirsync.refresh_access_token().await.unwrap();

        let groups = dirsync.get_user_groups("testuser").await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "1");
        assert_eq!(groups[0].name, "group1");
    }

    #[tokio::test]
    async fn test_group_members() {
        let mut dirsync = MicrosoftDirectorySync::new(
            "id".to_string(),
            "secret".to_string(),
            "https://login.microsoftonline.com/tenant-id-123/v2.0".to_string(),
        );
        dirsync.refresh_access_token().await.unwrap();

        let groups = dirsync.get_groups().await.unwrap();
        let members = dirsync.get_group_members(&groups[0]).await.unwrap();

        assert_eq!(members.len(), 3);
        assert_eq!(members[0], "testuser@email.com");
    }
}
