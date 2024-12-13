use std::{str::FromStr, time::Duration};

use super::{DirectoryGroup, DirectorySync, DirectorySyncError, DirectoryUser};
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Url;

const SCOPES: &str = "openid email profile https://www.googleapis.com/auth/admin.directory.customer.readonly https://www.googleapis.com/auth/admin.directory.group.readonly https://www.googleapis.com/auth/admin.directory.user.readonly";
const ACCESS_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GROUPS_URL: &str = "https://admin.googleapis.com/admin/directory/v1/groups";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";
const AUD: &str = "https://oauth2.googleapis.com/token";
const ALL_USERS_URL: &str = "https://admin.googleapis.com/admin/directory/v1/users";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

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
    fn new(iss: &str, sub: &str) -> Self {
        let now = chrono::Utc::now();
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

#[derive(Debug)]
pub struct GoogleDirectorySync {
    service_account_config: ServiceAccountConfig,
    access_token: Option<String>,
    token_expiry: Option<chrono::DateTime<chrono::Utc>>,
    admin_email: String,
}

///
/// Google Directory API responses
///
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

#[derive(Debug, Serialize, Deserialize)]
struct GroupMembersResponse {
    members: Option<Vec<GroupMember>>,
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

#[derive(Debug, Serialize, Deserialize)]
struct UsersResponse {
    users: Vec<User>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupsResponse {
    groups: Vec<DirectoryGroup>,
}

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
        &reqwest::StatusCode::OK => Ok(response.json().await?),
        _ => Err(DirectorySyncError::RequestError(format!(
            "{} Code returned: {}. Details: {}",
            context_message,
            status,
            response.text().await?
        ))),
    }
}

impl GoogleDirectorySync {
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
        let expires_in = chrono::Duration::seconds(token_response.expires_in);
        self.access_token = Some(token_response.token);
        self.token_expiry = Some(Utc::now() + expires_in);
        Ok(())
    }

    pub fn is_token_expired(&self) -> bool {
        debug!("Checking if Google directory sync token is expired");
        self.token_expiry
            .map(|expiry| expiry < Utc::now())
            // No token = expired token
            .unwrap_or(true)
    }

    #[cfg(not(test))]
    async fn query_user_groups(&self, user_id: &str) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = Url::from_str(GROUPS_URL).unwrap();

        url.query_pairs_mut()
            .append_pair("userKey", user_id)
            .append_pair("maxResults", "500");

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", access_token),
            )
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to query user groups from Google API.").await
    }

    #[cfg(not(test))]
    async fn query_groups(&self) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = Url::from_str(GROUPS_URL).unwrap();

        url.query_pairs_mut()
            .append_pair("customer", "my_customer")
            .append_pair("maxResults", "500");

        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to query groups from Google API.").await
    }

    #[cfg(not(test))]
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

        let url_str = format!(
            "https://admin.googleapis.com/admin/directory/v1/groups/{}/members",
            group.id
        );
        let mut url = Url::from_str(&url_str).unwrap();
        url.query_pairs_mut()
            .append_pair("includeDerivedMembership", "true")
            .append_pair("maxResults", "500");
        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to query group members from Google API.").await
    }

    fn build_token(&self) -> Result<String, DirectorySyncError> {
        let claims = Claims::new(&self.service_account_config.client_email, &self.admin_email);
        let key = EncodingKey::from_rsa_pem(self.service_account_config.private_key.as_bytes())?;
        let token = encode(&Header::new(Algorithm::RS256), &claims, &key)?;
        Ok(token)
    }

    #[cfg(not(test))]
    async fn query_access_token(&self) -> Result<AccessTokenResponse, DirectorySyncError> {
        let token = self.build_token()?;
        let mut url = Url::parse(ACCESS_TOKEN_URL).unwrap();
        url.query_pairs_mut()
            .append_pair("grant_type", GRANT_TYPE)
            .append_pair("assertion", &token);
        let client = reqwest::Client::builder().build()?;
        let response = client
            .post(url)
            .header(reqwest::header::CONTENT_LENGTH, 0)
            .send()
            .await?;
        parse_response(response, "Failed to get access token from Google API.").await
    }

    #[cfg(not(test))]
    async fn query_all_users(&self) -> Result<UsersResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = Url::from_str(ALL_USERS_URL).unwrap();
        url.query_pairs_mut()
            .append_pair("customer", "my_customer")
            .append_pair("maxResults", "500")
            .append_pair("showDeleted", "false");
        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        parse_response(response, "Failed to query all users in the Google API.").await
    }

    async fn query_test_connection(&self) -> Result<(), DirectorySyncError> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = Url::from_str(ALL_USERS_URL).unwrap();
        url.query_pairs_mut()
            .append_pair("customer", "my_customer")
            .append_pair("maxResults", "1")
            .append_pair("showDeleted", "false");
        let client = reqwest::Client::builder().build()?;
        let result = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token))
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await?;
        let _result: UsersResponse =
            parse_response(result, "Failed to test connection to Google API.").await?;
        Ok(())
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
        Ok(response.users.into_iter().map(|u| u.into()).collect())
    }

    async fn test_connection(&self) -> Result<(), DirectorySyncError> {
        self.query_test_connection().await?;
        Ok(())
    }
}

#[cfg(test)]
impl GoogleDirectorySync {
    async fn query_user_groups(&self, user_id: &str) -> Result<GroupsResponse, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        let _access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = Url::from_str(GROUPS_URL).expect("Invalid USER_GROUPS_URL has been set.");

        url.query_pairs_mut()
            .append_pair("userKey", user_id)
            .append_pair("max_results", "999");

        Ok(GroupsResponse {
            groups: vec![DirectoryGroup {
                id: "1".into(),
                name: "group1".into(),
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
        let mut url = Url::from_str(GROUPS_URL).expect("Invalid USER_GROUPS_URL has been set.");

        url.query_pairs_mut()
            .append_pair("customer", "my_customer")
            .append_pair("max_results", "999");

        Ok(GroupsResponse {
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

        let url_str = format!(
            "https://admin.googleapis.com/admin/directory/v1/groups/{}/members",
            group.id
        );
        let mut url = Url::from_str(&url_str).expect("Invalid GROUP_MEMBERS_URL has been set.");
        url.query_pairs_mut()
            .append_pair("includeDerivedMembership", "true");

        Ok(GroupMembersResponse {
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
        })
    }

    async fn query_access_token(&self) -> Result<AccessTokenResponse, DirectorySyncError> {
        let mut url: Url = ACCESS_TOKEN_URL
            .parse()
            .expect("Invalid ACCESS_TOKEN_URL has been set.");
        url.query_pairs_mut()
            .append_pair("grant_type", GRANT_TYPE)
            .append_pair("assertion", "test_assertion");
        Ok(AccessTokenResponse {
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
        let mut url = Url::from_str("https://admin.googleapis.com/admin/directory/v1/users")
            .expect("Invalid USERS_URL has been set.");
        url.query_pairs_mut().append_pair("customer", "my_customer");

        Ok(UsersResponse {
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
        })
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
        dirsync.token_expiry = Some(chrono::Utc::now() - chrono::Duration::seconds(10000));
        assert!(dirsync.is_token_expired());

        // valid token
        dirsync.access_token = Some("test_token".into());
        dirsync.token_expiry = Some(chrono::Utc::now() + chrono::Duration::seconds(10000));
        assert!(!dirsync.is_token_expired());

        // no token
        dirsync.access_token = Some("test_token".into());
        dirsync.token_expiry = Some(chrono::Utc::now() - chrono::Duration::seconds(10000));
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

        for i in 0..3 {
            assert_eq!(groups[i].id, (i + 1).to_string());
            assert_eq!(groups[i].name, format!("group{}", i + 1));
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
