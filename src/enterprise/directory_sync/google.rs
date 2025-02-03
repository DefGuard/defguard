use std::collections::HashMap;
#[cfg(not(test))]
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
#[cfg(not(test))]
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
#[cfg(not(test))]
use tokio::time::sleep;

#[cfg(not(test))]
use super::REQUEST_TIMEOUT;
use super::{
    make_get_request, parse_response, DirectoryGroup, DirectorySync, DirectorySyncError,
    DirectoryUser,
};

#[cfg(not(test))]
const SCOPES: &str = "openid email profile https://www.googleapis.com/auth/admin.directory.customer.readonly https://www.googleapis.com/auth/admin.directory.group.readonly https://www.googleapis.com/auth/admin.directory.user.readonly";
const ACCESS_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
#[cfg(not(test))]
const GROUPS_URL: &str = "https://admin.googleapis.com/admin/directory/v1/groups";
#[cfg(not(test))]
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";
#[cfg(not(test))]
const AUD: &str = "https://oauth2.googleapis.com/token";
const ALL_USERS_URL: &str = "https://admin.googleapis.com/admin/directory/v1/users";
#[cfg(not(test))]
const MAX_REQUESTS: usize = 50;
const MAX_RESULTS: &str = "200";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    sub: String,
    exp: i64,
    iat: i64,
}

#[cfg(not(test))]
impl Claims {
    #[must_use]
    #[cfg(not(test))]
    fn new(iss: &str, sub: &str) -> Self {
        let now = Utc::now();
        let now_timestamp = now.timestamp();
        let exp = now_timestamp + 3600;
        Self {
            iss: iss.into(),
            scope: SCOPES.into(),
            aud: AUD.to_string(),
            sub: sub.into(),
            exp,
            iat: now_timestamp,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceAccountConfig {
    private_key: String,
    client_email: String,
}

#[allow(dead_code)]
pub(crate) struct GoogleDirectorySync {
    service_account_config: ServiceAccountConfig,
    access_token: Option<String>,
    token_expiry: Option<DateTime<Utc>>,
    admin_email: String,
}

///
/// Google Directory API responses
///

#[derive(Debug, Serialize, Deserialize)]
struct AccessTokenResponse {
    #[serde(rename = "access_token")]
    token: String,
    expires_in: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupMember {
    email: String,
    status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct GroupMembersResponse {
    members: Option<Vec<GroupMember>>,
    #[serde(rename = "nextPageToken")]
    page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "primaryEmail")]
    primary_email: String,
    suspended: bool,
}

impl From<User> for DirectoryUser {
    fn from(val: User) -> Self {
        DirectoryUser {
            email: val.primary_email,
            active: !val.suspended,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct UsersResponse {
    users: Vec<User>,
    #[serde(rename = "nextPageToken")]
    page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct GroupsResponse {
    groups: Vec<DirectoryGroup>,
    #[serde(rename = "nextPageToken")]
    page_token: Option<String>,
}

impl GoogleDirectorySync {
    #[must_use]
    pub fn new(private_key: &str, client_email: &str, admin_email: &str) -> Self {
        Self {
            service_account_config: ServiceAccountConfig {
                private_key: private_key.into(),
                client_email: client_email.into(),
            },
            access_token: None,
            token_expiry: None,
            admin_email: admin_email.into(),
        }
    }

    pub async fn refresh_access_token(&mut self) -> Result<(), DirectorySyncError> {
        let token_response = self.query_access_token().await?;
        let expires_in = TimeDelta::seconds(token_response.expires_in);
        self.access_token = Some(token_response.token);
        self.token_expiry = Some(Utc::now() + expires_in);
        Ok(())
    }

    pub fn is_token_expired(&self) -> bool {
        debug!("Checking if Google directory sync token is expired");
        // No token = expired token
        self.token_expiry.map_or(true, |expiry| expiry < Utc::now())
    }

    async fn query_test_connection(&self) -> Result<(), DirectorySyncError> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let response = make_get_request(
            ALL_USERS_URL,
            access_token,
            Some(&[
                ("customer", "my_customer"),
                ("maxResults", MAX_RESULTS),
                ("showDeleted", "false"),
            ]),
        )
        .await?;
        let _result: UsersResponse =
            parse_response(response, "Failed to test connection to Google API.").await?;
        Ok(())
    }

    async fn query_user_groups(&self, user_id: &str) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        #[cfg_attr(test, allow(unused))]
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        #[cfg_attr(test, allow(unused_assignments))]
        let mut combined_response = GroupsResponse::default();
        #[cfg_attr(test, allow(unused, unused_mut))]
        let mut query = HashMap::from([
            ("userKey".to_string(), user_id.to_string()),
            ("maxResults".to_string(), MAX_RESULTS.to_string()),
        ]);

        #[cfg(not(test))]
        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(
                GROUPS_URL,
                access_token,
                Some(
                    &query
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect::<Vec<_>>(),
                ),
            )
            .await?;
            let response: GroupsResponse =
                parse_response(response, "Failed to query user groups from Google API.").await?;

            if combined_response.groups.is_empty() {
                combined_response.groups = response.groups;
            } else {
                combined_response.groups.extend(response.groups);
            }

            if let Some(next_page_token) = response.page_token {
                debug!("Found next page of results, using the following token to query it: {next_page_token}");
                query.insert("pageToken".to_string(), next_page_token);
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        #[cfg(test)]
        {
            combined_response = GroupsResponse {
                groups: vec![DirectoryGroup {
                    id: "1".into(),
                    name: "group1".into(),
                }],
                page_token: None,
            };
        }

        Ok(combined_response)
    }

    async fn query_groups(&self) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        #[cfg_attr(test, allow(unused))]
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        #[cfg_attr(test, allow(unused_assignments))]
        let mut combined_response = GroupsResponse::default();
        #[cfg_attr(test, allow(unused, unused_mut))]
        let mut query = HashMap::from([
            ("customer".to_string(), "my_customer".to_string()),
            ("maxResults".to_string(), MAX_RESULTS.to_string()),
        ]);

        #[cfg(not(test))]
        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(
                GROUPS_URL,
                access_token,
                Some(
                    &query
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect::<Vec<_>>(),
                ),
            )
            .await?;
            let response: GroupsResponse =
                parse_response(response, "Failed to query groups from Google API.").await?;

            if combined_response.groups.is_empty() {
                combined_response.groups = response.groups;
            } else {
                combined_response.groups.extend(response.groups);
            }

            if let Some(next_page_token) = response.page_token {
                debug!("Found next page of results, using the following token to query it: {next_page_token}");
                query.insert("pageToken".to_string(), next_page_token);
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        #[cfg(test)]
        {
            combined_response = GroupsResponse {
                groups: vec![
                    DirectoryGroup {
                        id: "1".into(),
                        name: "group1".into(),
                    },
                    DirectoryGroup {
                        id: "2".into(),
                        name: "group2".into(),
                    },
                    DirectoryGroup {
                        id: "3".into(),
                        name: "group3".into(),
                    },
                ],
                page_token: None,
            };
        }

        Ok(combined_response)
    }

    async fn query_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<GroupMembersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        #[cfg_attr(test, allow(unused))]
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;

        #[cfg_attr(test, allow(unused))]
        let url = format!(
            "https://admin.googleapis.com/admin/directory/v1/groups/{}/members",
            group.id
        );
        #[cfg_attr(test, allow(unused_assignments))]
        let mut combined_response = GroupMembersResponse::default();
        #[cfg_attr(test, allow(unused, unused_mut))]
        let mut query = HashMap::from([
            ("includeDerivedMembership".to_string(), "true".to_string()),
            ("maxResults".to_string(), MAX_RESULTS.to_string()),
        ]);

        #[cfg(not(test))]
        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(
                &url,
                access_token,
                Some(
                    &query
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect::<Vec<_>>(),
                ),
            )
            .await?;
            let response: GroupMembersResponse =
                parse_response(response, "Failed to query group members from Google API.").await?;

            if combined_response.members.is_none() {
                combined_response.members = response.members;
            } else {
                combined_response.members = combined_response.members.map(|mut members| {
                    members.extend(response.members.unwrap_or_default());
                    members
                });
            }

            if let Some(next_page_token) = response.page_token {
                debug!("Found next page of results, using the following token to query it: {next_page_token}");
                query.insert("pageToken".to_string(), next_page_token);
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        #[cfg(test)]
        {
            combined_response = GroupMembersResponse {
                members: Some(vec![
                    GroupMember {
                        email: "testuser@email.com".into(),
                        status: Some("ACTIVE".into()),
                    },
                    GroupMember {
                        email: "testuserdisabled@email.com".into(),
                        status: Some("SUSPENDED".into()),
                    },
                    GroupMember {
                        email: "testuser2@email.com".into(),
                        status: Some("ACTIVE".into()),
                    },
                ]),
                page_token: None,
            };
        }

        Ok(combined_response)
    }

    #[cfg(not(test))]
    fn build_token(&self) -> Result<String, DirectorySyncError> {
        let claims = Claims::new(&self.service_account_config.client_email, &self.admin_email);
        let key = EncodingKey::from_rsa_pem(self.service_account_config.private_key.as_bytes())?;
        let token = encode(&Header::new(Algorithm::RS256), &claims, &key)?;
        Ok(token)
    }

    #[cfg(not(test))]
    async fn query_access_token(&self) -> Result<AccessTokenResponse, DirectorySyncError> {
        let token = self.build_token()?;
        let client = reqwest::Client::new();
        let response = client
            .post(ACCESS_TOKEN_URL)
            .query(&[("grant_type", GRANT_TYPE), ("assertion", &token)])
            .header(reqwest::header::CONTENT_LENGTH, 0)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to get access token from Google API.").await
    }

    #[cfg(test)]
    async fn query_access_token(&self) -> Result<AccessTokenResponse, DirectorySyncError> {
        use reqwest::Url;
        let _url: Url = ACCESS_TOKEN_URL
            .parse()
            .expect("Invalid ACCESS_TOKEN_URL has been set.");
        Ok(AccessTokenResponse {
            token: "test_token_refreshed".into(),
            expires_in: 3600,
        })
    }

    async fn query_all_users(&self) -> Result<UsersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        #[cfg_attr(test, allow(unused))]
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        #[cfg_attr(test, allow(unused_assignments))]
        let mut combined_response = UsersResponse::default();
        #[cfg_attr(test, allow(unused, unused_mut))]
        let mut query = HashMap::from([
            ("customer".to_string(), "my_customer".to_string()),
            ("maxResults".to_string(), MAX_RESULTS.to_string()),
            ("showDeleted".to_string(), "false".to_string()),
        ]);

        #[cfg(not(test))]
        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(
                ALL_USERS_URL,
                access_token,
                Some(
                    &query
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .collect::<Vec<_>>(),
                ),
            )
            .await?;
            let response: UsersResponse =
                parse_response(response, "Failed to query all users in the Google API.").await?;

            if combined_response.users.is_empty() {
                combined_response.users = response.users;
            } else {
                combined_response.users.extend(response.users);
            }

            if let Some(next_page_token) = response.page_token {
                debug!("Found next page of results, using the following token to query it: {next_page_token}");
                query.insert("pageToken".to_string(), next_page_token);
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        #[cfg(test)]
        {
            combined_response = UsersResponse {
                users: vec![
                    User {
                        primary_email: "testuser@email.com".into(),
                        suspended: false,
                    },
                    User {
                        primary_email: "testuserdisabled@email.com".into(),
                        suspended: true,
                    },
                    User {
                        primary_email: "testuser2@email.com".into(),
                        suspended: false,
                    },
                ],
                page_token: None,
            }
        }

        Ok(combined_response)
    }
}

impl DirectorySync for GoogleDirectorySync {
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Getting all groups");
        let response = self.query_groups().await?;
        debug!("Got all groups response");
        Ok(response.groups)
    }

    async fn get_user_groups(
        &self,
        user_id: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Getting groups of user {user_id}");
        let response = self.query_user_groups(user_id).await?;
        debug!("Got groups response for user {user_id}");
        Ok(response.groups)
    }

    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<String>, DirectorySyncError> {
        debug!("Getting group members of group {}", group.name);
        let response = self.query_group_members(group).await?;
        debug!("Got group members response for group {}", group.name);
        Ok(response
            .members
            .unwrap_or_default()
            .into_iter()
            // There may be arbitrary members in the group, we want only one that are also directory members
            // Members without a status field don't belong to the directory
            .filter(|m| m.status.is_some())
            .map(|m| m.email)
            .collect())
    }

    async fn prepare(&mut self) -> Result<(), DirectorySyncError> {
        debug!("Preparing Google directory sync...");
        if self.is_token_expired() {
            debug!("Access token is expired, refreshing.");
            self.refresh_access_token().await?;
            debug!("Access token refreshed.");
        } else {
            debug!("Access token is still valid, skipping refresh.");
        }
        debug!("Google directory sync prepared.");
        Ok(())
    }

    async fn get_all_users(&self) -> Result<Vec<DirectoryUser>, DirectorySyncError> {
        debug!("Getting all users");
        let response = self.query_all_users().await?;
        debug!("Got all users response");
        Ok(response.users.into_iter().map(Into::into).collect())
    }

    async fn test_connection(&self) -> Result<(), DirectorySyncError> {
        debug!("Testing connection to Google API.");
        self.query_test_connection().await?;
        info!("Successfully tested connection to Google API, connection is working.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token() {
        let mut dirsync = GoogleDirectorySync::new("private_key", "client_email", "admin_email");

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
        let mut dirsync = GoogleDirectorySync::new("private_key", "client_email", "admin_email");
        dirsync.refresh_access_token().await.unwrap();

        let users = dirsync.get_all_users().await.unwrap();

        assert_eq!(users.len(), 3);
        assert_eq!(users[1].email, "testuserdisabled@email.com");
        assert!(!users[1].active);
    }

    #[tokio::test]
    async fn test_groups() {
        let mut dirsync = GoogleDirectorySync::new("private_key", "client_email", "admin_email");
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
        let mut dirsync = GoogleDirectorySync::new("private_key", "client_email", "admin_email");
        dirsync.refresh_access_token().await.unwrap();

        let groups = dirsync.get_user_groups("testuser").await.unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, "1");
        assert_eq!(groups[0].name, "group1");
    }

    #[tokio::test]
    async fn test_group_members() {
        let mut dirsync = GoogleDirectorySync::new("private_key", "client_email", "admin_email");
        dirsync.refresh_access_token().await.unwrap();

        let groups = dirsync.get_groups().await.unwrap();
        let members = dirsync.get_group_members(&groups[0]).await.unwrap();

        assert_eq!(members.len(), 3);
        assert_eq!(members[0], "testuser@email.com");
    }
}
