use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use tokio::time::sleep;

use super::{
    DirectoryGroup, DirectorySync, DirectorySyncError, DirectoryUser, REQUEST_PAGINATION_SLOWDOWN,
    REQUEST_TIMEOUT, make_get_request, parse_response,
};

const SCOPES: &str = "openid email profile https://www.googleapis.com/auth/admin.directory.customer.readonly https://www.googleapis.com/auth/admin.directory.group.readonly https://www.googleapis.com/auth/admin.directory.user.readonly";
const ACCESS_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GROUPS_URL: &str = "https://admin.googleapis.com/admin/directory/v1/groups";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";
const AUD: &str = "https://oauth2.googleapis.com/token";
const ALL_USERS_URL: &str = "https://admin.googleapis.com/admin/directory/v1/users";
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

impl Claims {
    #[must_use]
    fn new(iss: &str, sub: &str) -> Self {
        let now = Utc::now();
        let now_timestamp = now.timestamp();
        let exp = now_timestamp + 3600;
        Self {
            iss: iss.into(),
            scope: SCOPES.into(),
            aud: AUD.to_owned(),
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

pub(crate) struct GoogleDirectorySync {
    service_account_config: ServiceAccountConfig,
    access_token: Option<String>,
    token_expiry: Option<DateTime<Utc>>,
    admin_email: String,
    access_token_url: String,
    groups_url: String,
    all_users_url: String,
}

/// Google Directory API responses

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

impl From<GroupMembersResponse> for Vec<String> {
    fn from(val: GroupMembersResponse) -> Self {
        val.members
            .unwrap_or_default()
            .into_iter()
            // There may be arbitrary members in the group, we want only one that are also directory members
            // Members without a status field don't belong to the directory
            .filter(|m| m.status.is_some())
            .map(|m| m.email)
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct User {
    #[serde(rename = "primaryEmail")]
    primary_email: String,
    suspended: bool,
}

impl From<User> for DirectoryUser {
    fn from(val: User) -> Self {
        Self {
            email: val.primary_email,
            active: !val.suspended,
            id: None,
            // TODO: currently not supported for Google
            user_details: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct UsersResponse {
    users: Vec<User>,
    #[serde(rename = "nextPageToken")]
    page_token: Option<String>,
}

impl From<UsersResponse> for Vec<DirectoryUser> {
    fn from(val: UsersResponse) -> Self {
        val.users.into_iter().map(Into::into).collect()
    }
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
            access_token_url: ACCESS_TOKEN_URL.into(),
            groups_url: GROUPS_URL.into(),
            all_users_url: ALL_USERS_URL.into(),
        }
    }

    /// Overrides the Google API URLs so tests can point them at a mock server.
    #[cfg(test)]
    fn with_urls(mut self, access_token_url: &str, groups_url: &str, all_users_url: &str) -> Self {
        self.access_token_url = access_token_url.into();
        self.groups_url = groups_url.into();
        self.all_users_url = all_users_url.into();
        self
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
        self.token_expiry.is_none_or(|expiry| expiry < Utc::now())
    }

    async fn query_test_connection(&self) -> Result<(), DirectorySyncError> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let response = make_get_request(
            &self.all_users_url,
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
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut combined_response = GroupsResponse::default();
        let mut query = HashMap::from([
            ("userKey".to_owned(), user_id.to_owned()),
            ("maxResults".to_owned(), MAX_RESULTS.to_owned()),
        ]);

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(
                &self.groups_url,
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
                debug!(
                    "Found next page of results, using the following token to query it: {next_page_token}"
                );
                query.insert("pageToken".to_owned(), next_page_token);
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        Ok(combined_response)
    }

    async fn query_groups(&self) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut combined_response = GroupsResponse::default();
        let mut query = HashMap::from([
            ("customer".to_owned(), "my_customer".to_owned()),
            ("maxResults".to_owned(), MAX_RESULTS.to_owned()),
        ]);

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(
                &self.groups_url,
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
                debug!(
                    "Found next page of results, using the following token to query it: {next_page_token}"
                );
                query.insert("pageToken".to_owned(), next_page_token);
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
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;

        let url = format!("{}/{}/members", self.groups_url, group.id);
        let mut combined_response = GroupMembersResponse::default();
        let mut query = HashMap::from([
            ("includeDerivedMembership".to_owned(), "true".to_owned()),
            ("maxResults".to_owned(), MAX_RESULTS.to_owned()),
        ]);

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
                debug!(
                    "Found next page of results, using the following token to query it: {next_page_token}"
                );
                query.insert("pageToken".to_owned(), next_page_token);
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        Ok(combined_response)
    }

    fn build_token(&self) -> Result<String, DirectorySyncError> {
        let claims = Claims::new(&self.service_account_config.client_email, &self.admin_email);
        let key = EncodingKey::from_rsa_pem(self.service_account_config.private_key.as_bytes())?;
        let token = encode(&Header::new(Algorithm::RS256), &claims, &key)?;
        Ok(token)
    }

    async fn query_access_token(&self) -> Result<AccessTokenResponse, DirectorySyncError> {
        let token = self.build_token()?;
        let client = reqwest::Client::new();
        let response = client
            .post(&self.access_token_url)
            .query(&[("grant_type", GRANT_TYPE), ("assertion", &token)])
            .header(reqwest::header::CONTENT_LENGTH, 0)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to get access token from Google API.").await
    }

    async fn query_all_users(&self) -> Result<UsersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut combined_response = UsersResponse::default();
        let mut query = HashMap::from([
            ("customer".to_owned(), "my_customer".to_owned()),
            ("maxResults".to_owned(), MAX_RESULTS.to_owned()),
            ("showDeleted".to_owned(), "false".to_owned()),
        ]);

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(
                &self.all_users_url,
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
                debug!(
                    "Found next page of results, using the following token to query it: {next_page_token}"
                );
                query.insert("pageToken".to_owned(), next_page_token);
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
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
        user_email: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Getting groups of user {user_email}");
        let response = self.query_user_groups(user_email).await?;
        debug!("Got groups response for user {user_email}");
        Ok(response.groups)
    }

    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
        _all_users_helper: Option<&[DirectoryUser]>,
    ) -> Result<Vec<String>, DirectorySyncError> {
        debug!("Getting group members of group {}", group.name);
        let response = self.query_group_members(group).await?;
        debug!(
            "Got group members response for group {}. Extracting their email addresses...",
            group.name
        );
        Ok(response.into())
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
        Ok(response.into())
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
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path, query_param, query_param_is_missing},
    };

    use super::*;

    // Test-only key, unrelated to any real account; only needed for `build_token` to
    // produce a syntactically valid JWT since Google's endpoints are mocked out.
    const TEST_RSA_PRIVATE_KEY: &str = include_str!("fixtures/google/test_private_key.pem");

    /// Loads a fixture from `fixtures/google/` (real Google Directory API response shapes)
    /// and wraps it in a 200 JSON response.
    fn response_from_fixture(name: &str) -> ResponseTemplate {
        let body = match name {
            "token_response.json" => include_str!("fixtures/google/token_response.json"),
            "users_page1.json" => include_str!("fixtures/google/users_page1.json"),
            "users_page2.json" => include_str!("fixtures/google/users_page2.json"),
            "users_empty.json" => include_str!("fixtures/google/users_empty.json"),
            "groups_response.json" => include_str!("fixtures/google/groups_response.json"),
            "members_response.json" => include_str!("fixtures/google/members_response.json"),
            other => panic!("unknown fixture: {other}"),
        };
        ResponseTemplate::new(200)
            .insert_header("content-type", "application/json")
            .set_body_string(body)
    }

    fn dirsync_with_mock_server(mock_server: &MockServer) -> GoogleDirectorySync {
        let mut dirsync = GoogleDirectorySync::new("private_key", "client_email", "admin_email")
            .with_urls(
                &format!("{}/token", mock_server.uri()),
                &format!("{}/groups", mock_server.uri()),
                &format!("{}/users", mock_server.uri()),
            );
        dirsync.access_token = Some("test_token".into());
        dirsync.token_expiry = Some(Utc::now() + TimeDelta::seconds(3600));
        dirsync
    }

    #[tokio::test]
    async fn test_refresh_access_token() {
        let mock_server = MockServer::start().await;
        // Real response shape from https://oauth2.googleapis.com/token (jwt-bearer grant).
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(response_from_fixture("token_response.json"))
            .mount(&mock_server)
            .await;

        let mut dirsync = GoogleDirectorySync::new("private_key", "client_email", "admin_email")
            .with_urls(&format!("{}/token", mock_server.uri()), "", "");
        dirsync.service_account_config.private_key = TEST_RSA_PRIVATE_KEY.into();

        dirsync.refresh_access_token().await.unwrap();

        assert_eq!(
            dirsync.access_token.as_deref(),
            Some("ya29.c.b0Aaekm1KfR7fake_opaque_token_value")
        );
        assert!(!dirsync.is_token_expired());
    }

    #[tokio::test]
    async fn test_get_all_users_paginates() {
        let mock_server = MockServer::start().await;
        // Real response shape from admin#directory#users (Directory API users.list).
        Mock::given(method("GET"))
            .and(path("/users"))
            .and(query_param_is_missing("pageToken"))
            .respond_with(response_from_fixture("users_page1.json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/users"))
            .and(query_param("pageToken", "EAIaBhACGgtGkAAfirstpage"))
            .respond_with(response_from_fixture("users_page2.json"))
            .mount(&mock_server)
            .await;

        let dirsync = dirsync_with_mock_server(&mock_server);
        let users = dirsync.get_all_users().await.unwrap();

        assert_eq!(users.len(), 2);
        assert!(
            users
                .iter()
                .any(|u| u.email == "jane.doe@example.com" && u.active)
        );
        assert!(
            users
                .iter()
                .any(|u| u.email == "john.smith@example.com" && !u.active)
        );
    }

    #[tokio::test]
    async fn test_get_groups() {
        let mock_server = MockServer::start().await;
        // Real response shape from admin#directory#groups (Directory API groups.list).
        Mock::given(method("GET"))
            .and(path("/groups"))
            .respond_with(response_from_fixture("groups_response.json"))
            .mount(&mock_server)
            .await;

        let dirsync = dirsync_with_mock_server(&mock_server);
        let groups = dirsync.get_groups().await.unwrap();

        assert_eq!(groups.len(), 2);
        assert!(groups.iter().any(|g| g.name == "Engineering"));
        assert!(groups.iter().any(|g| g.name == "Sales"));
    }

    #[tokio::test]
    async fn test_get_group_members() {
        let mock_server = MockServer::start().await;
        // Real response shape from admin#directory#members (Directory API members.list).
        Mock::given(method("GET"))
            .and(path("/groups/01302m9251m2vt3/members"))
            .respond_with(response_from_fixture("members_response.json"))
            .mount(&mock_server)
            .await;

        let dirsync = dirsync_with_mock_server(&mock_server);
        let group = DirectoryGroup {
            id: "01302m9251m2vt3".into(),
            name: "Engineering".into(),
        };
        let members = dirsync.get_group_members(&group, None).await.unwrap();

        assert_eq!(members, vec!["jane.doe@example.com".to_string()]);
    }

    #[tokio::test]
    async fn test_test_connection() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users"))
            .respond_with(response_from_fixture("users_empty.json"))
            .mount(&mock_server)
            .await;

        let dirsync = dirsync_with_mock_server(&mock_server);
        dirsync.test_connection().await.unwrap();
    }

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
    }

    #[tokio::test]
    async fn test_group_members_parse() {
        let response = GroupMembersResponse {
            members: Some(vec![
                GroupMember {
                    email: "email@email.com".into(),
                    status: Some("active".into()),
                },
                GroupMember {
                    email: "email2@email.com".into(),
                    status: Some("active".into()),
                },
                GroupMember {
                    email: "email3@email.com".into(),
                    status: Some("suspended".into()),
                },
                GroupMember {
                    email: "email4@email.com".into(),
                    status: None,
                },
            ]),
            page_token: None,
        };

        let members: Vec<String> = response.into();
        assert_eq!(members.len(), 3);
        assert!(members.contains(&"email@email.com".into()));
        assert!(members.contains(&"email2@email.com".into()));
        assert!(members.contains(&"email3@email.com".into()));
    }

    #[tokio::test]
    async fn test_all_users_parse() {
        let response = UsersResponse {
            users: vec![
                User {
                    primary_email: "email@email.com".into(),
                    suspended: false,
                },
                User {
                    primary_email: "email2@email.com".into(),
                    suspended: true,
                },
                User {
                    primary_email: "email3@email.com".into(),
                    suspended: false,
                },
            ],
            page_token: None,
        };

        let users: Vec<DirectoryUser> = response.into();
        assert_eq!(users.len(), 3);
        let disabled_user = users
            .iter()
            .find(|u| u.email == "email2@email.com")
            .unwrap();
        assert!(!disabled_user.active);
    }
}
