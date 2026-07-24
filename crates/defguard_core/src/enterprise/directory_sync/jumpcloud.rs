use std::collections::HashMap;

use tokio::time::sleep;

use super::{
    DirectoryGroup, DirectorySync, DirectorySyncError, DirectoryUser, REQUEST_PAGINATION_SLOWDOWN,
    parse_response,
};

const DEFAULT_API_HOST: &str = "console.jumpcloud.com";
const EU_API_HOST: &str = "console.eu.jumpcloud.com";
const IN_API_HOST: &str = "console.in.jumpcloud.com";
const MAX_REQUESTS: usize = 50;
const MAX_RESULTS: usize = 100;
const API_KEY_HEADER: &str = "x-api-key";

/// Picks the JumpCloud API host based on the OIDC base URL the admin configured.
pub(crate) fn api_host_for(base_url: &str) -> &'static str {
    if base_url.contains("eu.jumpcloud") {
        return EU_API_HOST;
    }
    if base_url.contains("in.jumpcloud") {
        return IN_API_HOST;
    }
    DEFAULT_API_HOST
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
enum UserState {
    Staged,
    Activated,
    Suspended,
}

#[derive(Debug, Deserialize)]
struct User {
    email: String,
    activated: bool,
    account_locked: bool,
    id: String,
    state: UserState,
}

impl From<User> for DirectoryUser {
    fn from(user: User) -> Self {
        Self {
            email: user.email,
            active: user.activated && !user.account_locked && user.state == UserState::Activated,
            id: Some(user.id),
            // TODO: currently not supported for Jumpcloud
            user_details: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct UsersResponse {
    results: Vec<User>,
    #[serde(rename = "totalCount")]
    total_count: usize,
}

impl From<UsersResponse> for Vec<DirectoryUser> {
    fn from(response: UsersResponse) -> Self {
        response.results.into_iter().map(Into::into).collect()
    }
}

#[derive(Debug, Deserialize)]
struct GroupsResponse {
    results: Vec<DirectoryGroup>,
}

impl From<GroupsResponse> for Vec<DirectoryGroup> {
    fn from(response: GroupsResponse) -> Self {
        response.results
    }
}

#[derive(Debug, Deserialize)]
struct LdapGroup {
    name: String,
}

#[derive(Debug, Deserialize)]
struct CompiledAttributes {
    #[serde(rename = "ldapGroups")]
    ldap_groups: Vec<LdapGroup>,
}

#[derive(Debug, Deserialize)]
struct UserGroup {
    id: String,
    #[serde(rename = "compiledAttributes")]
    compiled_attributes: CompiledAttributes,
}

impl From<UserGroup> for DirectoryGroup {
    fn from(group: UserGroup) -> Self {
        let name = group.compiled_attributes.ldap_groups.first().map_or_else(
            || {
                debug!(
                    "Group {} has no LDAP groups, using ID as name fallback",
                    group.id
                );
                group.id.clone()
            },
            |g| g.name.clone(),
        );
        Self { id: group.id, name }
    }
}

#[derive(Debug, Deserialize)]
struct GroupMember {
    id: String,
    #[serde(rename = "type")]
    member_type: String,
}

#[derive(Debug, Deserialize)]
struct GroupMemberThing {
    to: GroupMember,
}

pub(crate) struct JumpCloudDirectorySync {
    api_key: String,
    api_host: &'static str,
    base_url_override: Option<String>,
}

impl JumpCloudDirectorySync {
    #[must_use]
    pub fn new(api_key: String, api_host: &'static str) -> Self {
        debug!(
            "Initializing JumpCloud directory sync with API key length: {}",
            api_key.len()
        );
        Self {
            api_key,
            api_host,
            base_url_override: None,
        }
    }

    /// Overrides the JumpCloud API base URL so tests can point it at a mock server.
    #[cfg(test)]
    fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url_override = Some(base_url.into());
        self
    }

    fn groups_url(&self) -> String {
        if let Some(override_url) = &self.base_url_override {
            format!("{override_url}/api/v2/usergroups")
        } else {
            format!("https://{}/api/v2/usergroups", self.api_host)
        }
    }

    fn all_users_url(&self) -> String {
        if let Some(override_url) = &self.base_url_override {
            format!("{override_url}/api/systemusers")
        } else {
            format!("https://{}/api/systemusers", self.api_host)
        }
    }

    fn user_groups_url(&self, user_id: &str) -> String {
        if let Some(override_url) = &self.base_url_override {
            format!("{override_url}/api/v2/users/{user_id}/memberof")
        } else {
            format!("https://{}/api/v2/users/{user_id}/memberof", self.api_host)
        }
    }

    fn user_group_members_url(&self, group_id: &str) -> String {
        if let Some(override_url) = &self.base_url_override {
            format!("{override_url}/api/v2/usergroups/{group_id}/members")
        } else {
            format!(
                "https://{}/api/v2/usergroups/{group_id}/members",
                self.api_host
            )
        }
    }

    async fn query_group_members(
        &self,
        group: &DirectoryGroup,
    ) -> Result<Vec<GroupMemberThing>, DirectorySyncError> {
        debug!(
            "Starting to query members for group: {} (ID: {})",
            group.name, group.id
        );
        let client = reqwest::Client::new();
        let url = self.user_group_members_url(&group.id);
        let mut query = HashMap::from([("limit", MAX_RESULTS.to_string())]);

        debug!("Requesting group members from URL: {url}");
        debug!("Initial query parameters: {query:?}");

        let response = client
            .get(&url)
            .header(API_KEY_HEADER, &self.api_key)
            .query(&query)
            .send()
            .await?;

        debug!(
            "Initial response status for group {}: {}",
            group.id,
            response.status()
        );
        let mut all_members_response: Vec<GroupMemberThing> = parse_response(
            response,
            "Failed to query group members from JumpCloud API.",
        )
        .await?;

        debug!(
            "Initial batch fetched {} members for group {}",
            all_members_response.len(),
            group.id
        );

        for i in 1..MAX_REQUESTS {
            let skip_value = i * MAX_RESULTS;
            query.insert("skip", skip_value.to_string());

            debug!(
                "Requesting page {} (skip: {skip_value}) for group {} members",
                i + 1,
                group.id
            );

            let response = client
                .get(&url)
                .header(API_KEY_HEADER, &self.api_key)
                .query(&query)
                .send()
                .await?;

            debug!(
                "Page {} response status for group {}: {}",
                i + 1,
                group.id,
                response.status()
            );
            let members_response: Vec<GroupMemberThing> = parse_response(
                response,
                "Failed to query group members from JumpCloud API.",
            )
            .await?;

            debug!(
                "Page {} returned {} members for group {}",
                i + 1,
                members_response.len(),
                group.id
            );

            if members_response.is_empty() {
                debug!(
                    "No more members found for group {}, stopping pagination",
                    group.id
                );
                break;
            }
            all_members_response.extend(members_response);
            debug!(
                "Total members accumulated so far for group {}: {}",
                group.id,
                all_members_response.len()
            );

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        debug!(
            "Total members fetched for group {}: {}",
            group.id,
            all_members_response.len()
        );
        Ok(all_members_response)
    }

    async fn query_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Starting to query groups from JumpCloud API");
        let client = reqwest::Client::new();

        let mut query = HashMap::from([("limit", MAX_RESULTS.to_string())]);
        debug!("Initial query parameters: {query:?}");

        debug!("Sending initial request to: {}", self.groups_url());
        let response = client
            .get(self.groups_url())
            .header(API_KEY_HEADER, &self.api_key)
            .query(&query)
            .send()
            .await?;

        debug!("Initial response status: {}", response.status());
        let mut all_groups_response: Vec<DirectoryGroup> =
            parse_response(response, "Failed to query groups from JumpCloud API.").await?;

        debug!("Initial batch fetched {} groups", all_groups_response.len());

        for i in 1..MAX_REQUESTS {
            let skip_value = i * MAX_RESULTS;
            query.insert("skip", skip_value.to_string());

            debug!(
                "Requesting page {} (skip: {skip_value}) from JumpCloud API",
                i + 1
            );

            let response = client
                .get(self.groups_url())
                .header(API_KEY_HEADER, &self.api_key)
                .query(&query)
                .send()
                .await?;

            debug!("Page {} response status: {}", i + 1, response.status());
            let groups_response: Vec<DirectoryGroup> =
                parse_response(response, "Failed to query groups from JumpCloud API.").await?;

            debug!("Page {} returned {} groups", i + 1, groups_response.len());

            if groups_response.is_empty() {
                debug!("No more groups found, stopping pagination");
                break;
            }
            all_groups_response.extend(groups_response);
            debug!(
                "Total groups accumulated so far: {}",
                all_groups_response.len()
            );

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        debug!("Total groups fetched: {}", all_groups_response.len());
        Ok(all_groups_response)
    }

    async fn query_all_users(&self) -> Result<UsersResponse, DirectorySyncError> {
        debug!("Starting to query all users from JumpCloud API");
        let client = reqwest::Client::new();

        let mut query = HashMap::from([("limit", MAX_RESULTS.to_string())]);
        debug!("Initial query parameters for users: {query:?}");
        debug!("Sending initial request to: {}", self.all_users_url());

        let response = client
            .get(self.all_users_url())
            .header(API_KEY_HEADER, &self.api_key)
            .query(&query)
            .send()
            .await?;

        debug!("Initial users response status: {}", response.status());
        let mut all_users_response: UsersResponse =
            parse_response(response, "Failed to query users from JumpCloud API.").await?;

        debug!(
            "Initial batch fetched {} users (total_count: {})",
            all_users_response.results.len(),
            all_users_response.total_count
        );

        for i in 1..MAX_REQUESTS {
            let skip_value = i * MAX_RESULTS;
            query.insert("skip", skip_value.to_string());

            debug!("Requesting page {} (skip: {skip_value}) for users", i + 1);

            let response = client
                .get(self.all_users_url())
                .header(API_KEY_HEADER, &self.api_key)
                .query(&query)
                .send()
                .await?;

            debug!(
                "Page {} response status for users: {}",
                i + 1,
                response.status()
            );
            let users_response: UsersResponse =
                parse_response(response, "Failed to query users from JumpCloud API.").await?;

            debug!(
                "Page {} returned {} users",
                i + 1,
                users_response.results.len()
            );

            if users_response.results.is_empty() {
                debug!("No more users found, stopping pagination");
                break;
            }
            all_users_response.results.extend(users_response.results);
            debug!(
                "Total users accumulated so far: {}",
                all_users_response.results.len()
            );

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        debug!(
            "Total users fetched: {} (final total_count: {})",
            all_users_response.results.len(),
            all_users_response.total_count
        );
        Ok(all_users_response)
    }

    async fn query_user_groups(&self, user_id: &str) -> Result<Vec<UserGroup>, DirectorySyncError> {
        debug!("Starting to query groups for user: {user_id}");
        let client = reqwest::Client::new();
        let url = self.user_groups_url(user_id);

        let mut query = HashMap::from([("limit", MAX_RESULTS.to_string())]);
        debug!("Requesting user groups from URL: {url}");
        debug!("Initial query parameters for user groups: {query:?}");

        let response = client
            .get(&url)
            .header(API_KEY_HEADER, &self.api_key)
            .query(&query)
            .send()
            .await?;

        debug!(
            "Initial response status for user {user_id} groups: {}",
            response.status()
        );
        let mut all_groups_response: Vec<UserGroup> =
            parse_response(response, "Failed to query user groups from JumpCloud API.").await?;

        debug!(
            "Initial batch fetched {} groups for user {user_id}",
            all_groups_response.len()
        );

        for i in 1..MAX_REQUESTS {
            let skip_value = i * MAX_RESULTS;
            query.insert("skip", skip_value.to_string());

            debug!(
                "Requesting page {} (skip: {}) for user {user_id} groups",
                i + 1,
                skip_value,
            );

            let response = client
                .get(&url)
                .header(API_KEY_HEADER, &self.api_key)
                .query(&query)
                .send()
                .await?;

            debug!(
                "Page {} response status for user {user_id} groups: {}",
                i + 1,
                response.status()
            );
            let groups_response: Vec<UserGroup> =
                parse_response(response, "Failed to query user groups from JumpCloud API.").await?;

            debug!(
                "Page {} returned {} groups for user {user_id}",
                i + 1,
                groups_response.len(),
            );

            if groups_response.is_empty() {
                debug!("No more groups found for user {user_id}, stopping pagination");
                break;
            }
            all_groups_response.extend(groups_response);
            debug!(
                "Total groups accumulated so far for user {user_id}: {}",
                all_groups_response.len()
            );

            sleep(REQUEST_PAGINATION_SLOWDOWN).await;
        }

        debug!(
            "Total groups fetched for user {user_id}: {}",
            all_groups_response.len()
        );
        Ok(all_groups_response)
    }

    async fn query_test_connection(&self) -> Result<(), DirectorySyncError> {
        debug!("Testing connection to JumpCloud API");
        let client = reqwest::Client::new();
        debug!("Sending test request to: {}", self.all_users_url());

        let response = client
            .get(self.all_users_url())
            .header(API_KEY_HEADER, &self.api_key)
            .send()
            .await?;

        debug!("Test connection response status: {}", response.status());
        let _: UsersResponse =
            parse_response(response, "Failed to test connection to JumpCloud API.").await?;
        debug!("Test connection successful - API key is valid and endpoint is accessible");
        Ok(())
    }

    async fn get_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<DirectoryUser>, DirectorySyncError> {
        debug!("Starting search for user by email: {email}");
        let client = reqwest::Client::new();

        let filter = format!("email:$eq:{email}");

        debug!("Querying JumpCloud for user with email: {email}");
        debug!("Using filter: {filter}");
        debug!("Sending request to: {}", self.all_users_url());

        let response = client
            .get(self.all_users_url())
            .header(API_KEY_HEADER, &self.api_key)
            .query(&[("filter", &filter)])
            .send()
            .await?;

        debug!("User search response status: {}", response.status());

        if response.status().is_success() {
            let mut users: UsersResponse =
                parse_response(response, "Failed to query user by email.").await?;

            debug!(
                "User search returned {} users (total_count: {})",
                users.results.len(),
                users.total_count
            );

            if users.total_count > 1 {
                warn!(
                    "Multiple users found with email: {} (count: {})",
                    email, users.total_count
                );
                return Err(DirectorySyncError::MultipleUsersFound(format!(
                    "Multiple users found with email: {email}."
                )));
            }

            if let Some(user) = users.results.pop() {
                debug!(
                    "Found user: {} (ID: {}, activated: {}, locked: {}, state: {:?})",
                    user.email, user.id, user.activated, user.account_locked, user.state
                );
                Ok(Some(user.into()))
            } else {
                debug!("No user found with email: {}", email);
                Ok(None)
            }
        } else {
            error!(
                "Failed to query user by email: {}. Status: {}",
                email,
                response.status()
            );
            Err(DirectorySyncError::RequestError(format!(
                "Failed to query user by email: {}. Status: {}. Details: {}",
                email,
                response.status(),
                response
                    .text()
                    .await
                    .unwrap_or_else(|_| "No details".to_owned())
            )))
        }
    }
}

impl DirectorySync for JumpCloudDirectorySync {
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Getting all groups");
        let response = self.query_groups().await?;
        debug!("Got all groups response");
        Ok(response)
    }

    async fn get_user_groups(
        &self,
        user_email: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        debug!("Getting groups of user {user_email}");
        if let Some(user) = self.get_user_by_email(user_email).await?
            && let Some(user_id) = user.id
        {
            let response = self.query_user_groups(&user_id).await?;
            debug!("Got groups response for user {user_id}");
            return Ok(response.into_iter().map(Into::into).collect());
        }

        debug!("No user found with email {user_email}, returning an error.");
        Err(DirectorySyncError::UserNotFound(user_email.to_owned()))
    }

    async fn get_group_members(
        &self,
        group: &DirectoryGroup,
        all_users_helper: Option<&[DirectoryUser]>,
    ) -> Result<Vec<String>, DirectorySyncError> {
        debug!("Getting group members of group {}", group.name);

        let users: Vec<DirectoryUser>;

        // extract all_users_helper, if its empty, return an error
        let all_users = if let Some(users) = all_users_helper {
            debug!("Using provided all users helper");
            users
        } else {
            debug!("No all users helper provided, forcing a query for all users as a fallback.");
            users = self.query_all_users().await?.into();
            &users
        };

        let member_response = self
            .query_group_members(group)
            .await?
            .into_iter()
            .filter(|m| m.to.member_type == "user")
            .collect::<Vec<_>>();

        let mut members = Vec::new();
        for member in member_response {
            if let Some(user) = all_users
                .iter()
                .find(|u| u.id.as_deref() == Some(&member.to.id) && u.active)
            {
                members.push(user.email.clone());
            } else {
                debug!(
                    "Skipping member with ID {} in group {} as they are not found in all users",
                    member.to.id, group.name
                );
            }
        }
        debug!(
            "Got group members response for group {}. Extracting their email addresses...",
            group.name
        );
        Ok(members)
    }

    async fn prepare(&mut self) -> Result<(), DirectorySyncError> {
        debug!("JumpCloud does not require any preparation steps, skipping.");
        Ok(())
    }

    async fn get_all_users(&self) -> Result<Vec<DirectoryUser>, DirectorySyncError> {
        debug!("Getting all users");
        let response = self.query_all_users().await?;
        debug!("Got all users response");
        Ok(response.into())
    }

    async fn test_connection(&self) -> Result<(), DirectorySyncError> {
        debug!("Testing connection to JumpCloud API.");
        self.query_test_connection().await?;
        info!("Successfully tested connection to JumpCloud API, connection is working.");
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn response_from_fixture(name: &str) -> wiremock::ResponseTemplate {
    let body = match name {
        "users_page1.json" => include_str!("fixtures/jumpcloud/users_page1.json"),
        "users_page2.json" => include_str!("fixtures/jumpcloud/users_page2.json"),
        "users_empty.json" => include_str!("fixtures/jumpcloud/users_empty.json"),
        "groups_response.json" => include_str!("fixtures/jumpcloud/groups_response.json"),
        "group_members_response.json" => {
            include_str!("fixtures/jumpcloud/group_members_response.json")
        }
        "user_groups_response.json" => {
            include_str!("fixtures/jumpcloud/user_groups_response.json")
        }
        other => panic!("unknown fixture: {other}"),
    };
    wiremock::ResponseTemplate::new(200)
        .insert_header("content-type", "application/json")
        .set_body_string(body)
}

#[cfg(test)]
pub(crate) fn dirsync_with_mock_server(
    mock_server: &wiremock::MockServer,
) -> JumpCloudDirectorySync {
    JumpCloudDirectorySync::new("test_api_key".into(), "console.jumpcloud.com")
        .with_base_url(&mock_server.uri())
}

#[cfg(test)]
#[path = "provider_tests/jumpcloud.rs"]
mod tests;
