use std::{collections::HashMap, str::FromStr};

use super::{
    DirectoryGroup, DirectorySync, DirectorySyncError, DirectorySyncProvider, DirectoryUser,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Url;

const SCOPES: &str = "openid email profile https://www.googleapis.com/auth/admin.directory.customer.readonly https://www.googleapis.com/auth/admin.directory.orgunit.readonly https://www.googleapis.com/auth/admin.directory.group.readonly https://www.googleapis.com/auth/admin.directory.user.readonly";
const ACCESS_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GROUPS_URL: &str = "https://admin.googleapis.com/admin/directory/v1/groups";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";
const AUD: &str = "https://oauth2.googleapis.com/token";

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

#[derive(Debug, Serialize, Deserialize)]
struct ServiceAccountConfigJson {
    private_key: String,
    client_email: String,
}

impl From<&str> for ServiceAccountConfig {
    fn from(json: &str) -> Self {
        let config: ServiceAccountConfigJson = serde_json::from_str(json).unwrap();
        Self {
            private_key: config.private_key,
            client_email: config.client_email,
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
enum ETag {
    AllUsers,
    AllGroups,
    UserGroups(String),
    GroupMembers(String),
}

#[derive(Debug)]
pub struct GoogleDirectorySync {
    service_account_config: ServiceAccountConfig,
    access_token: Option<AccessToken>,
    token_expiry: Option<chrono::DateTime<chrono::Utc>>,
    admin_email: String,
    etags: HashMap<ETag, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessToken {
    #[serde(rename = "access_token")]
    token: String,
    expires_in: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DirectoryGroupMember {
    email: String,
    status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupMembersResponse {
    members: Option<Vec<DirectoryGroupMember>>,
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
            etags: HashMap::new(),
        }
    }

    fn get_etag(&self, etag: ETag) -> Option<&str> {
        self.etags.get(&etag).map(|s| s.as_str())
    }

    fn set_etag(&mut self, etag: ETag, value: String) {
        self.etags.insert(etag, value);
    }

    pub async fn refresh_access_token(&mut self) -> Result<(), DirectorySyncError> {
        let response = self.query_access_token().await?;
        let token: AccessToken = response.json().await?;
        let expires_in = chrono::Duration::seconds(token.expires_in);
        self.access_token = Some(token);
        self.token_expiry = Some(chrono::Utc::now() + expires_in);
        Ok(())
    }

    // No token = expired token
    pub fn is_token_expired(&self) -> bool {
        debug!("Checking if Google directory sync token is expired");
        self.token_expiry
            .map(|expiry| expiry < chrono::Utc::now())
            .unwrap_or(true)
    }

    // #[cfg(not(test))]
    async fn query_user_groups(
        &self,
        user_id: &str,
    ) -> Result<reqwest::Response, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        let access_token: &AccessToken = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = Url::from_str(GROUPS_URL).expect("Invalid USER_GROUPS_URL has been set.");

        url.query_pairs_mut()
            .append_pair("userKey", user_id)
            .append_pair("max_results", "999");

        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token.token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        Ok(response)
    }

    // #[cfg(not(test))]
    async fn query_groups(&self) -> Result<reqwest::Response, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }

        let access_token: &AccessToken = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = Url::from_str(GROUPS_URL).expect("Invalid USER_GROUPS_URL has been set.");

        url.query_pairs_mut()
            .append_pair("customer", "my_customer")
            .append_pair("max_results", "999");

        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token.token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        Ok(response)
    }

    // #[cfg(not(test))]
    async fn query_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<reqwest::Response, DirectorySyncError> {
        let access_token: &AccessToken = self
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
        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token.token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        Ok(response)
    }

    fn build_token(&self) -> Result<String, DirectorySyncError> {
        let claims = Claims::new(
            &self.service_account_config.client_email,
            // FIXME: What should we put here?
            &self.admin_email,
        );
        let key = EncodingKey::from_rsa_pem(self.service_account_config.private_key.as_bytes())?;
        let token = encode(&Header::new(Algorithm::RS256), &claims, &key)?;
        Ok(token)
    }

    // #[cfg(not(test))]
    async fn query_access_token(&self) -> Result<reqwest::Response, DirectorySyncError> {
        let token = self.build_token()?;
        let mut url: Url = ACCESS_TOKEN_URL
            .parse()
            .expect("Invalid ACCESS_TOKEN_URL has been set.");
        url.query_pairs_mut()
            .append_pair("grant_type", GRANT_TYPE)
            .append_pair("assertion", &token);
        let client = reqwest::Client::builder().build()?;
        let response = client.post(url).header("content-length", 0).send().await?;
        Ok(response)
    }

    // #[cfg(not(test))]
    async fn query_all_users(&self) -> Result<reqwest::Response, DirectorySyncError> {
        let access_token: &AccessToken = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = Url::from_str("https://admin.googleapis.com/admin/directory/v1/users")
            .expect("Invalid USERS_URL has been set.");
        url.query_pairs_mut().append_pair("customer", "my_customer");
        let client = reqwest::Client::builder().build()?;
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token.token))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;

        Ok(response)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Group {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupsResponse {
    etag: String,
    groups: Vec<DirectoryGroup>,
}

impl DirectorySync for GoogleDirectorySync {
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Getting all groups");
        let response: GroupsResponse = self.query_groups().await?.json().await?;
        debug!("Got all groups response");
        Ok(response.groups)
    }

    async fn get_user_groups(&self, user_id: &str) -> Result<Vec<String>, DirectorySyncError> {
        debug!("Getting groups of user {}", user_id);
        let response: GroupsResponse = self.query_user_groups(user_id).await?.json().await?;
        debug!("Got groups response for user {}", user_id);
        Ok(response
            .groups
            .into_iter()
            .map(|group| group.name)
            .collect())
    }

    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<String>, DirectorySyncError> {
        debug!("Getting group members of group {}", group.name);
        let response: GroupMembersResponse = self.query_group_members(group).await?.json().await?;
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

    fn get_provider_type(&self) -> DirectorySyncProvider {
        DirectorySyncProvider::Google
    }

    async fn get_all_users(&self) -> Result<Vec<DirectoryUser>, DirectorySyncError> {
        debug!("Getting all users");
        let json = self
            .query_all_users()
            .await?
            .json::<serde_json::Value>()
            .await?;
        println!("{:?}", json);
        let response: UsersResponse = serde_json::from_value(json).unwrap();
        debug!("Got all users response");
        Ok(response.users.into_iter().map(|u| u.into()).collect())
    }
}

// #[cfg(test)]
// impl GoogleDirectorySync {
//     async fn get_access_token(&self) -> AccessToken {
//         AccessToken {
//             token: "token".to_string(),
//             expires_in: 3600,
//         }
//     }

//     async fn query_user_groups(
//         &self,
//         _user_id: &str,
//     ) -> Result<GroupsResponse, DirectorySyncError> {
//         if self.is_token_expired() {
//             return Err(DirectorySyncError::AccessTokenExpired);
//         }

//         Ok(GroupsResponse {
//             groups: vec![
//                 Group {
//                     name: "group1".to_string(),
//                 },
//                 Group {
//                     name: "group2".to_string(),
//                 },
//                 Group {
//                     name: "group3".to_string(),
//                 },
//             ],
//         })
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[tokio::test]
//     async fn test_token() {
//         let mut sync = GoogleDirectorySync::default();
//         assert!(sync.access_token.is_none());
//         sync.refresh_access_token().await;
//         assert!(sync.access_token.is_some());
//         assert!(!sync.is_token_expired());
//     }

//     #[tokio::test]
//     async fn test_user_groups() {
//         let mut sync = GoogleDirectorySync::default();
//         sync.refresh_access_token().await;
//         let groups = sync.get_user_groups("user1").await.unwrap();
//         assert_eq!(groups, vec!["group1", "group2", "group3"]);
//     }
// }
