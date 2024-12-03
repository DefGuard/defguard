use std::str::FromStr;

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Url;

use crate::{db::Id, enterprise::db::models::openid_provider::OpenIdProvider};

use super::{DirectoryGroup, DirectorySync, DirectorySyncError};

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

#[derive(Debug)]
pub struct GoogleDirectorySync {
    service_account_config: ServiceAccountConfig,
    access_token: Option<AccessToken>,
    token_expiry: Option<chrono::DateTime<chrono::Utc>>,
    admin_email: String,
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
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupMembersResponse {
    members: Option<Vec<DirectoryGroupMember>>,
}

impl GoogleDirectorySync {
    pub fn new(provider_settings: &OpenIdProvider<Id>) -> Self {
        let service_account_config = ServiceAccountConfig {
            private_key: provider_settings
                .google_service_account_key
                .clone()
                .unwrap(),
            client_email: provider_settings
                .google_service_account_email
                .clone()
                .unwrap(),
        };
        let admin_email = provider_settings.admin_email.clone().unwrap();

        Self {
            service_account_config,
            access_token: None,
            token_expiry: None,
            admin_email,
        }
    }

    pub async fn refresh_access_token(&mut self) -> Result<(), DirectorySyncError> {
        let token = self.get_access_token().await?;
        println!("TOKEN: {:?}", token);
        let expires_in = chrono::Duration::seconds(token.expires_in);
        self.access_token = Some(token);
        self.token_expiry = Some(chrono::Utc::now() + expires_in);

        Ok(())
    }

    // No token = expired token
    pub fn is_token_expired(&self) -> bool {
        self.token_expiry
            .map(|expiry| expiry < chrono::Utc::now())
            .unwrap_or(true)
    }

    // #[cfg(not(test))]
    async fn query_user_groups(&self, user_id: &str) -> Result<GroupsResponse, DirectorySyncError> {
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
            .send()
            .await?;

        Ok(response.json::<GroupsResponse>().await?)
    }

    // #[cfg(not(test))]
    async fn query_groups(&self) -> Result<GroupsResponse, DirectorySyncError> {
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
            .send()
            .await?;

        Ok(response.json::<GroupsResponse>().await?)
    }

    // #[cfg(not(test))]
    async fn query_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<GroupMembersResponse, DirectorySyncError> {
        let access_token: &AccessToken = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;

        let url_str = format!(
            "https://admin.googleapis.com/admin/directory/v1/groups/{}/members",
            group.id
        );

        let url = Url::from_str(&url_str).expect("Invalid GROUP_MEMBERS_URL has been set.");

        let client = reqwest::Client::builder().build()?;

        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", access_token.token))
            .send()
            .await?;

        let response = response.json::<GroupMembersResponse>().await?;

        if response.members.is_none() {
            debug!("Received no 'members' field in response, assuming there are no members in the group {}", group.name);
        }

        Ok(response)
    }

    // #[cfg(not(test))]
    async fn get_access_token(&self) -> Result<AccessToken, DirectorySyncError> {
        let claims = Claims::new(
            &self.service_account_config.client_email,
            // FIXME: What should we put here?
            &self.admin_email,
        );

        let key = EncodingKey::from_rsa_pem(self.service_account_config.private_key.as_bytes())?;
        let token = encode(&Header::new(Algorithm::RS256), &claims, &key)?;

        let mut url: Url = ACCESS_TOKEN_URL
            .parse()
            .expect("Invalid ACCESS_TOKEN_URL has been set.");
        url.query_pairs_mut()
            .append_pair("grant_type", GRANT_TYPE)
            .append_pair("assertion", &token);

        let client = reqwest::Client::builder().build()?;
        let response = client.post(url).header("content-length", 0).send().await?;
        println!("{:?}", response);
        let response = response.json::<AccessToken>().await?;
        Ok(response)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Group {
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupsResponse {
    groups: Vec<DirectoryGroup>,
}

impl DirectorySync for GoogleDirectorySync {
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        Ok(self.query_groups().await?.groups)
    }

    async fn get_user_groups(&self, user_id: &str) -> Result<Vec<String>, DirectorySyncError> {
        Ok(self
            .query_user_groups(user_id)
            .await?
            .groups
            .into_iter()
            .map(|group| group.name)
            .collect())
    }

    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<String>, DirectorySyncError> {
        Ok(self
            .query_group_members(group)
            .await?
            .members
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.email)
            .collect())
    }

    async fn prepare(&mut self) -> Result<(), DirectorySyncError> {
        if self.is_token_expired() {
            debug!("Access token is expired, refreshing.");
            self.refresh_access_token().await?;
            debug!("Access token refreshed.");
        } else {
            debug!("Access token is still valid, skipping refresh.");
        }
        Ok(())
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
