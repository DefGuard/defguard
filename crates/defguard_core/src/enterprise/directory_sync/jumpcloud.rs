use std::collections::HashMap;

use tokio::time::sleep;

use super::{
    DirectoryGroup, DirectorySync, DirectorySyncError, DirectoryUser, REQUEST_PAGINATION_SLOWDOWN,
    parse_response,
};

const GROUPS_URL: &str = "https://console.jumpcloud.com/api/v2/usergroups";
const ALL_USERS_URL: &str = "https://console.jumpcloud.com/api/systemusers";
const USER_GROUPS_URL: &str = "https://console.jumpcloud.com/api/v2/users/<USER_ID>/memberof";
const USER_GROUP_MEMBERS_URL: &str =
    "https://console.jumpcloud.com/api/v2/usergroups/<GROUP_ID>/members";
const MAX_REQUESTS: usize = 50;
const MAX_RESULTS: usize = 100;
const API_KEY_HEADER: &str = "x-api-key";

#[derive(Debug, Deserialize, PartialEq, Eq)]
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
        DirectoryUser {
            email: user.email,
            active: user.activated && !user.account_locked && user.state == UserState::Activated,
            id: Some(user.id),
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
        DirectoryGroup { id: group.id, name }
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
}

impl JumpCloudDirectorySync {
    #[must_use]
    pub fn new(api_key: String) -> Self {
        debug!(
            "Initializing JumpCloud directory sync with API key length: {}",
            api_key.len()
        );
        Self { api_key }
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
        let url = USER_GROUP_MEMBERS_URL.replace("<GROUP_ID>", &group.id);
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
            } else {
                all_members_response.extend(members_response);
                debug!(
                    "Total members accumulated so far for group {}: {}",
                    group.id,
                    all_members_response.len()
                );
            }

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

        debug!("Sending initial request to: {GROUPS_URL}");
        let response = client
            .get(GROUPS_URL)
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
                .get(GROUPS_URL)
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
            } else {
                all_groups_response.extend(groups_response);
                debug!(
                    "Total groups accumulated so far: {}",
                    all_groups_response.len()
                );
            }

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
        debug!("Sending initial request to: {ALL_USERS_URL}");

        let response = client
            .get(ALL_USERS_URL)
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
                .get(ALL_USERS_URL)
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
            } else {
                all_users_response.results.extend(users_response.results);
                debug!(
                    "Total users accumulated so far: {}",
                    all_users_response.results.len()
                );
            }

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
        let url = USER_GROUPS_URL.replace("<USER_ID>", user_id);

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
            } else {
                all_groups_response.extend(groups_response);
                debug!(
                    "Total groups accumulated so far for user {user_id}: {}",
                    all_groups_response.len()
                );
            }

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
        debug!("Sending test request to: {ALL_USERS_URL}");

        let response = client
            .get(ALL_USERS_URL)
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
        debug!("Sending request to: {ALL_USERS_URL}");

        let response = client
            .get(ALL_USERS_URL)
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
                    .unwrap_or_else(|_| "No details".to_string())
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
        if let Some(user) = self.get_user_by_email(user_email).await? {
            if let Some(user_id) = user.id {
                let response = self.query_user_groups(&user_id).await?;
                debug!("Got groups response for user {user_id}");
                return Ok(response.into_iter().map(Into::into).collect());
            }
        }

        debug!("No user found with email {user_email}, returning an error.");
        Err(DirectorySyncError::UserNotFound(user_email.to_string()))
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
mod tests {
    use super::*;

    #[test]
    fn test_user_to_directory_user_conversions() {
        // Test active user (activated=true, account_locked=false, state=ACTIVATED)
        let active_user = User {
            email: "active@example.com".to_string(),
            activated: true,
            account_locked: false,
            id: "user123".to_string(),
            state: UserState::Activated,
        };
        let active_directory_user: DirectoryUser = active_user.into();
        assert_eq!(active_directory_user.email, "active@example.com");
        assert!(active_directory_user.active);
        assert_eq!(active_directory_user.id, Some("user123".to_string()));

        // Test inactive user (activated=false)
        let inactive_user = User {
            email: "inactive@example.com".to_string(),
            activated: false,
            account_locked: false,
            id: "user456".to_string(),
            state: UserState::Activated,
        };
        let inactive_directory_user: DirectoryUser = inactive_user.into();
        assert_eq!(inactive_directory_user.email, "inactive@example.com");
        assert!(!inactive_directory_user.active);
        assert_eq!(inactive_directory_user.id, Some("user456".to_string()));

        // Test locked user (account_locked=true)
        let locked_user = User {
            email: "locked@example.com".to_string(),
            activated: true,
            account_locked: true,
            id: "user789".to_string(),
            state: UserState::Activated,
        };
        let locked_directory_user: DirectoryUser = locked_user.into();
        assert_eq!(locked_directory_user.email, "locked@example.com");
        assert!(!locked_directory_user.active);
        assert_eq!(locked_directory_user.id, Some("user789".to_string()));

        // Test suspended user (state=SUSPENDED)
        let suspended_user = User {
            email: "suspended@example.com".to_string(),
            activated: true,
            account_locked: false,
            id: "user999".to_string(),
            state: UserState::Suspended,
        };
        let suspended_directory_user: DirectoryUser = suspended_user.into();
        assert_eq!(suspended_directory_user.email, "suspended@example.com");
        assert!(!suspended_directory_user.active);
        assert_eq!(suspended_directory_user.id, Some("user999".to_string()));

        // Test staged user (state=STAGED)
        let staged_user = User {
            email: "staged@example.com".to_string(),
            activated: true,
            account_locked: false,
            id: "user888".to_string(),
            state: UserState::Staged,
        };
        let staged_directory_user: DirectoryUser = staged_user.into();
        assert_eq!(staged_directory_user.email, "staged@example.com");
        assert!(!staged_directory_user.active);
        assert_eq!(staged_directory_user.id, Some("user888".to_string()));

        // Test both inactive and locked user
        let both_user = User {
            email: "both@example.com".to_string(),
            activated: false,
            account_locked: true,
            id: "user000".to_string(),
            state: UserState::Activated,
        };
        let both_directory_user: DirectoryUser = both_user.into();
        assert_eq!(both_directory_user.email, "both@example.com");
        assert!(!both_directory_user.active);
        assert_eq!(both_directory_user.id, Some("user000".to_string()));
    }

    #[test]
    fn test_user_group_to_directory_group_conversions() {
        // Test group with LDAP groups (uses first LDAP group name)
        let group_with_ldap = UserGroup {
            id: "group123".to_string(),
            compiled_attributes: CompiledAttributes {
                ldap_groups: vec![
                    LdapGroup {
                        name: "LDAP Group Name".to_string(),
                    },
                    LdapGroup {
                        name: "Second LDAP Group".to_string(),
                    },
                ],
            },
        };
        let directory_group_with_ldap: DirectoryGroup = group_with_ldap.into();
        assert_eq!(directory_group_with_ldap.id, "group123");
        assert_eq!(directory_group_with_ldap.name, "LDAP Group Name");

        // Test group with empty LDAP groups (falls back to group ID)
        let group_empty_ldap = UserGroup {
            id: "group789".to_string(),
            compiled_attributes: CompiledAttributes {
                ldap_groups: vec![],
            },
        };
        let directory_group_empty_ldap: DirectoryGroup = group_empty_ldap.into();
        assert_eq!(directory_group_empty_ldap.id, "group789");
        assert_eq!(directory_group_empty_ldap.name, "group789");
    }

    #[test]
    fn test_response_collection_conversions() {
        // Test empty UsersResponse conversion
        let empty_users_response = UsersResponse {
            results: vec![],
            total_count: 0,
        };
        let empty_directory_users: Vec<DirectoryUser> = empty_users_response.into();
        assert!(empty_directory_users.is_empty());

        // Test single user UsersResponse conversion
        let single_users_response = UsersResponse {
            results: vec![User {
                email: "single@example.com".to_string(),
                activated: true,
                account_locked: false,
                id: "single123".to_string(),
                state: UserState::Activated,
            }],
            total_count: 1,
        };
        let single_directory_users: Vec<DirectoryUser> = single_users_response.into();
        assert_eq!(single_directory_users.len(), 1);
        assert_eq!(single_directory_users[0].email, "single@example.com");
        assert!(single_directory_users[0].active);
        assert_eq!(single_directory_users[0].id, Some("single123".to_string()));

        // Test multiple users with mixed states
        let multiple_users_response = UsersResponse {
            results: vec![
                User {
                    email: "user1@example.com".to_string(),
                    activated: true,
                    account_locked: false,
                    id: "user1".to_string(),
                    state: UserState::Activated,
                },
                User {
                    email: "user2@example.com".to_string(),
                    activated: false,
                    account_locked: false,
                    id: "user2".to_string(),
                    state: UserState::Activated,
                },
                User {
                    email: "user3@example.com".to_string(),
                    activated: true,
                    account_locked: true,
                    id: "user3".to_string(),
                    state: UserState::Activated,
                },
            ],
            total_count: 3,
        };
        let multiple_directory_users: Vec<DirectoryUser> = multiple_users_response.into();
        assert_eq!(multiple_directory_users.len(), 3);
        assert_eq!(multiple_directory_users[0].email, "user1@example.com");
        assert!(multiple_directory_users[0].active);
        assert_eq!(multiple_directory_users[1].email, "user2@example.com");
        assert!(!multiple_directory_users[1].active);
        assert_eq!(multiple_directory_users[2].email, "user3@example.com");
        assert!(!multiple_directory_users[2].active);

        // Test GroupsResponse conversion
        let groups_response = GroupsResponse {
            results: vec![
                DirectoryGroup {
                    id: "group1".to_string(),
                    name: "Group 1".to_string(),
                },
                DirectoryGroup {
                    id: "group2".to_string(),
                    name: "Group 2".to_string(),
                },
            ],
        };
        let directory_groups: Vec<DirectoryGroup> = groups_response.into();
        assert_eq!(directory_groups.len(), 2);
        assert_eq!(directory_groups[0].id, "group1");
        assert_eq!(directory_groups[0].name, "Group 1");
        assert_eq!(directory_groups[1].id, "group2");
        assert_eq!(directory_groups[1].name, "Group 2");
    }
}
