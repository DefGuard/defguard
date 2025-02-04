use std::str::FromStr;
use std::time::Duration;

use chrono::{DateTime, TimeDelta, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use parse_link_header::parse_with_rel;
use tokio::time::sleep;

use super::{parse_response, DirectoryGroup, DirectorySync, DirectorySyncError, DirectoryUser};
use crate::enterprise::directory_sync::make_get_request;

// Okta suggests using the maximum limit of 200 for the number of results per page.
// If this is an issue, we would need to add resource pagination.
const ACCESS_TOKEN_URL: &str = "{BASE_URL}/oauth2/v1/token";
const GROUPS_URL: &str = "{BASE_URL}/api/v1/groups";
const GRANT_TYPE: &str = "client_credentials";
const CLIENT_ASSERTION_TYPE: &str = "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";
const TOKEN_SCOPE: &str = "okta.users.read okta.groups.read";
const ALL_USERS_URL: &str = "{BASE_URL}/api/v1/users";
const GROUP_MEMBERS: &str = "{BASE_URL}/api/v1/groups/{GROUP_ID}/users";
const USER_GROUPS: &str = "{BASE_URL}/api/v1/users/{USER_ID}/groups";
const MAX_RESULTS: &str = "200";
const MAX_REQUESTS: usize = 50;

pub fn extract_next_link(
    link_header: Option<&String>,
) -> Result<Option<String>, DirectorySyncError> {
    if let Some(header) = link_header {
        let mut res = parse_with_rel(header).map_err(|e| {
            DirectorySyncError::InvalidUrl(format!("Failed to parse link header: {e:?}"))
        })?;
        Ok(res.remove("next").map(|x| x.raw_uri))
    } else {
        Ok(None)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    sub: String,
    exp: i64,
    iat: i64,
}

impl Claims {
    #[must_use]
    fn new(client_id: &str, base_url: &str) -> Self {
        let now = Utc::now();
        let now_timestamp = now.timestamp();
        let exp = now_timestamp + 3600;
        Self {
            iss: client_id.into(),
            aud: ACCESS_TOKEN_URL.replace("{BASE_URL}", base_url),
            sub: client_id.into(),
            exp,
            iat: now_timestamp,
        }
    }
}

#[allow(dead_code)]
pub struct OktaDirectorySync {
    access_token: Option<String>,
    token_expiry: Option<DateTime<Utc>>,
    jwk_private_key: String,
    client_id: String,
    base_url: String,
}

///
/// Okta Directory API responses
///

#[derive(Debug, Deserialize)]
pub struct AccessTokenResponse {
    #[serde(rename = "access_token")]
    token: String,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
struct UserProfile {
    email: String,
}

#[derive(Debug, Deserialize)]
struct User {
    status: String,
    profile: UserProfile,
}

impl From<User> for DirectoryUser {
    fn from(val: User) -> Self {
        Self {
            email: val.profile.email,
            active: ACTIVE_STATUS.contains(&val.status.as_str()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GroupProfile {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Group {
    id: String,
    profile: GroupProfile,
}

impl From<Group> for DirectoryGroup {
    fn from(val: Group) -> Self {
        Self {
            id: val.id,
            name: val.profile.name,
        }
    }
}

// The status may be:
// "ACTIVE" "DEPROVISIONED" "LOCKED_OUT" "PASSWORD_EXPIRED" "PROVISIONED" "RECOVERY" "STAGED" "SUSPENDED"
// We currently consider only ACTIVE users as active. Change this if needed.
const ACTIVE_STATUS: [&str; 1] = ["ACTIVE"];

impl OktaDirectorySync {
    #[must_use]
    pub fn new(private_key: &str, client_id: &str, base_url: &str) -> Self {
        // Remove the trailing slash just to make sure
        let trimmed = base_url.trim_end_matches('/');
        Self {
            client_id: client_id.to_string(),
            jwk_private_key: private_key.to_string(),
            base_url: trimmed.to_string(),
            access_token: None,
            token_expiry: None,
        }
    }

    pub async fn refresh_access_token(&mut self) -> Result<(), DirectorySyncError> {
        debug!("Refreshing Okta directory sync access token");
        let token_response = self.query_access_token().await?;
        let expires_in = TimeDelta::seconds(token_response.expires_in);
        debug!(
            "Access token refreshed, the new token expires in {} seconds",
            token_response.expires_in
        );
        self.access_token = Some(token_response.token);
        self.token_expiry = Some(Utc::now() + expires_in);
        Ok(())
    }

    pub fn is_token_expired(&self) -> bool {
        debug!("Checking if Okta directory sync token is expired");
        // No token = expired token
        let result = self.token_expiry.map_or(true, |expiry| expiry < Utc::now());
        debug!("Token is expired: {}", result);
        result
    }

    async fn query_test_connection(&self) -> Result<(), DirectorySyncError> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let response = make_get_request(
            &ALL_USERS_URL.replace("{BASE_URL}", &self.base_url),
            access_token,
            Some(&[("limit", "1")]),
        )
        .await?;
        let _result: Vec<User> =
            parse_response(response, "Failed to test connection to Okta API.").await?;
        Ok(())
    }

    async fn query_user_groups(&self, user_id: &str) -> Result<Vec<Group>, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = USER_GROUPS
            .replace("{BASE_URL}", &self.base_url)
            .replace("{USER_ID}", user_id);
        let mut combined_response: Vec<Group> = Vec::new();
        let mut query = Some([("limit", MAX_RESULTS)].as_slice());

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(&url, access_token, query).await?;
            let link_header = {
                let links = response
                    .headers()
                    .get_all("link")
                    .iter()
                    .filter_map(|link| link.to_str().ok())
                    .collect::<Vec<&str>>();

                (!links.is_empty()).then(|| links.join(", "))
            };
            let result: Vec<Group> =
                parse_response(response, "Failed to query user groups in the Okta API.").await?;
            combined_response.extend(result);

            if let Some(next_link) = extract_next_link(link_header.as_ref())? {
                url = next_link;
                // Query is already appended to the URL we received from the link header
                query = None;
                debug!("Found next page of results, querying it: {url}");
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        Ok(combined_response)
    }

    async fn query_groups(&self) -> Result<Vec<Group>, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = GROUPS_URL.replace("{BASE_URL}", &self.base_url);
        let mut combined_response: Vec<Group> = Vec::new();
        let mut query = Some([("limit", MAX_RESULTS)].as_slice());

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(&url, access_token, query).await?;
            let link_header = {
                let links = response
                    .headers()
                    .get_all("link")
                    .iter()
                    .filter_map(|link| link.to_str().ok())
                    .collect::<Vec<&str>>();

                (!links.is_empty()).then(|| links.join(", "))
            };
            let result: Vec<Group> =
                parse_response(response, "Failed to query groups in the Okta API.").await?;
            combined_response.extend(result);

            if let Some(next_link) = extract_next_link(link_header.as_ref())? {
                url = next_link;
                // Query is already appended to the URL we received from the link header
                query = None;
                debug!("Found next page of results, querying it: {url}");
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        Ok(combined_response)
    }

    async fn query_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<User>, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = GROUP_MEMBERS
            .replace("{BASE_URL}", &self.base_url)
            .replace("{GROUP_ID}", &group.id);
        let mut combined_response: Vec<User> = Vec::new();
        let mut query = Some([("limit", MAX_RESULTS)].as_slice());

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(&url, access_token, query).await?;
            let link_header = {
                let links = response
                    .headers()
                    .get_all("link")
                    .iter()
                    .filter_map(|link| link.to_str().ok())
                    .collect::<Vec<&str>>();

                (!links.is_empty()).then(|| links.join(", "))
            };
            let result: Vec<User> =
                parse_response(response, "Failed to query group members in the Okta API.").await?;
            combined_response.extend(result);

            if let Some(next_link) = extract_next_link(link_header.as_ref())? {
                url = next_link;
                // Query is already appended to the URL we received from the link header
                query = None;
                debug!("Found next page of results, querying it: {url}");
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        Ok(combined_response)
    }

    fn build_token(&self) -> Result<String, DirectorySyncError> {
        debug!("Building Okta directory sync auth token");
        let claims = Claims::new(&self.client_id, &self.base_url);
        debug!("Using the following token claims: {:?}", claims);
        // Users provide a JWK format private key. The jsonwebtoken library currently doesn't support
        // converting JWK to PEM or encoding key so the jsonwebkey library is used to convert the key
        // to a PEM format.
        debug!("Building Okta directory sync encoding key");
        let jwk = jsonwebkey::JsonWebKey::from_str(&self.jwk_private_key)
            .map_err(|e| DirectorySyncError::InvalidProviderConfiguration(e.to_string()))?;
        let kid = jwk
            .key_id
            .ok_or(DirectorySyncError::InvalidProviderConfiguration(
                "Missing key id in the provided JSON key".to_string(),
            ))?;
        let encoding_key_pem = jwk
            .key
            .try_to_pem()
            .map_err(|e| DirectorySyncError::InvalidProviderConfiguration(e.to_string()))?;
        let key = EncodingKey::from_rsa_pem(encoding_key_pem.as_bytes())?;
        debug!("Successfully built Okta directory sync encoding key for encoding the auth token");
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid);
        let token = encode(&header, &claims, &key)?;
        debug!("Successfully built Okta directory sync auth token");
        Ok(token)
    }

    async fn query_access_token(&self) -> Result<AccessTokenResponse, DirectorySyncError> {
        let token = self.build_token()?;
        let client = reqwest::Client::new();
        let response = client
            .post(ACCESS_TOKEN_URL.replace("{BASE_URL}", &self.base_url))
            .form(&[
                ("grant_type", GRANT_TYPE),
                ("client_assertion_type", CLIENT_ASSERTION_TYPE),
                ("client_assertion", &token),
                ("scope", TOKEN_SCOPE),
            ])
            .send()
            .await?;
        parse_response(response, "Failed to get access token from Okta API.").await
    }

    async fn query_all_users(&self) -> Result<Vec<User>, DirectorySyncError> {
        if self.is_token_expired() {
            return Err(DirectorySyncError::AccessTokenExpired);
        }
        let access_token = self
            .access_token
            .as_ref()
            .ok_or(DirectorySyncError::AccessTokenExpired)?;
        let mut url = ALL_USERS_URL.replace("{BASE_URL}", &self.base_url);
        let mut query = Some([("limit", MAX_RESULTS)].as_slice());
        let mut combined_response: Vec<User> = Vec::new();

        for _ in 0..MAX_REQUESTS {
            let response = make_get_request(&url, access_token, query).await?;
            let link_header = {
                let links = response
                    .headers()
                    .get_all("link")
                    .iter()
                    .filter_map(|link| link.to_str().ok())
                    .collect::<Vec<&str>>();

                (!links.is_empty()).then(|| links.join(", "))
            };
            let result: Vec<User> =
                parse_response(response, "Failed to query all users in the Okta API.").await?;
            combined_response.extend(result);
            if let Some(next_link) = extract_next_link(link_header.as_ref())? {
                url = next_link;
                // Query is already appended to the URL we received from the link header
                query = None;
                debug!("Found next page of results, querying it: {url}");
            } else {
                debug!("No more pages of results found, finishing query.");
                break;
            }

            sleep(Duration::from_millis(100)).await;
        }

        Ok(combined_response)
    }
}

impl DirectorySync for OktaDirectorySync {
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Getting all groups");
        let response = self.query_groups().await?;
        debug!("Got all groups response");
        Ok(response.into_iter().map(Into::into).collect())
    }

    async fn get_user_groups(
        &self,
        user_id: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Getting groups of user {user_id}");
        let response = self.query_user_groups(user_id).await?;
        debug!("Got groups response for user {user_id}");
        Ok(response.into_iter().map(Into::into).collect())
    }

    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<String>, DirectorySyncError> {
        debug!("Getting group members of group {}", group.name);
        let response = self.query_group_members(group).await?;
        debug!("Got group members response for group {}", group.name);
        Ok(response
            .into_iter()
            .map(|user| user.profile.email)
            .collect())
    }

    async fn prepare(&mut self) -> Result<(), DirectorySyncError> {
        debug!("Preparing Okta directory sync...");
        if self.is_token_expired() {
            debug!("Access token is expired, refreshing.");
            self.refresh_access_token().await?;
            debug!("Access token refreshed.");
        } else {
            debug!("Access token is still valid, skipping refresh.");
        }
        debug!("Okta directory sync prepared.");
        Ok(())
    }

    async fn get_all_users(&self) -> Result<Vec<DirectoryUser>, DirectorySyncError> {
        debug!("Getting all users");
        let response: Vec<User> = self.query_all_users().await?;
        debug!("Got all users response");
        Ok(response.into_iter().map(Into::into).collect())
    }

    async fn test_connection(&self) -> Result<(), DirectorySyncError> {
        debug!("Testing connection to Okta API.");
        self.query_test_connection().await?;
        info!("Successfully tested connection to Okta API, connection is working.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token() {
        let mut dirsync =
            OktaDirectorySync::new("private_key", "client_id", "https://trial-0000000.okta.com");

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
    async fn test_header() {
        let link_header =
            "<https://trial-0000000.okta.com/api/v1/users?after=4&limit=200>; rel=\"next\""
                .to_string();
        let next_link = extract_next_link(Some(&link_header)).unwrap();
        assert_eq!(
            next_link,
            Some("https://trial-0000000.okta.com/api/v1/users?after=4&limit=200".to_string())
        );

        let next_link = extract_next_link(None).unwrap();
        assert_eq!(next_link, None);

        let link_header = "invalid".to_string();
        let next_link = extract_next_link(Some(&link_header));
        assert!(next_link.is_err());

        let link_header = "<https://trial-0000000.okta.com/api/v1/users?after=4&limit=200>; rel=\"next\", <https://trial-0000000.okta.com/api/v1/users?after=4&limit=200>; rel=\"prev\"".to_string();
        let next_link = extract_next_link(Some(&link_header)).unwrap();
        assert_eq!(
            next_link,
            Some("https://trial-0000000.okta.com/api/v1/users?after=4&limit=200".to_string())
        );
    }

    #[tokio::test]
    async fn test_group_parse() {
        let group = Group {
            id: "test_id".to_string(),
            profile: GroupProfile {
                name: "test_name".to_string(),
            },
        };
        let dir_group: DirectoryGroup = group.into();
        assert_eq!(dir_group.id, "test_id");
        assert_eq!(dir_group.name, "test_name");
    }

    #[tokio::test]
    async fn test_user_parse() {
        let user = User {
            status: "ACTIVE".to_string(),
            profile: UserProfile {
                email: "test_email".to_string(),
            },
        };

        let dir_user: DirectoryUser = user.into();
        assert_eq!(dir_user.email, "test_email");
        assert!(dir_user.active);

        let user = User {
            status: "INACTIVE".to_string(),
            profile: UserProfile {
                email: "test_email".to_string(),
            },
        };

        let dir_user: DirectoryUser = user.into();
        assert_eq!(dir_user.email, "test_email");
        assert!(!dir_user.active);
    }
}
