use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param, query_param_is_missing},
};

use super::*;

#[test]
fn test_user_to_directory_user_conversions() {
    // Test active user (activated=true, account_locked=false, state=ACTIVATED)
    let active_user = User {
        email: "active@example.com".to_owned(),
        activated: true,
        account_locked: false,
        id: "user123".to_owned(),
        state: UserState::Activated,
    };
    let active_directory_user: DirectoryUser = active_user.into();
    assert_eq!(active_directory_user.email, "active@example.com");
    assert!(active_directory_user.active);
    assert_eq!(active_directory_user.id, Some("user123".to_owned()));

    // Test inactive user (activated=false)
    let inactive_user = User {
        email: "inactive@example.com".to_owned(),
        activated: false,
        account_locked: false,
        id: "user456".to_owned(),
        state: UserState::Activated,
    };
    let inactive_directory_user: DirectoryUser = inactive_user.into();
    assert_eq!(inactive_directory_user.email, "inactive@example.com");
    assert!(!inactive_directory_user.active);
    assert_eq!(inactive_directory_user.id, Some("user456".to_owned()));

    // Test locked user (account_locked=true)
    let locked_user = User {
        email: "locked@example.com".to_owned(),
        activated: true,
        account_locked: true,
        id: "user789".to_owned(),
        state: UserState::Activated,
    };
    let locked_directory_user: DirectoryUser = locked_user.into();
    assert_eq!(locked_directory_user.email, "locked@example.com");
    assert!(!locked_directory_user.active);
    assert_eq!(locked_directory_user.id, Some("user789".to_owned()));

    // Test suspended user (state=SUSPENDED)
    let suspended_user = User {
        email: "suspended@example.com".to_owned(),
        activated: true,
        account_locked: false,
        id: "user999".to_owned(),
        state: UserState::Suspended,
    };
    let suspended_directory_user: DirectoryUser = suspended_user.into();
    assert_eq!(suspended_directory_user.email, "suspended@example.com");
    assert!(!suspended_directory_user.active);
    assert_eq!(suspended_directory_user.id, Some("user999".to_owned()));

    // Test staged user (state=STAGED)
    let staged_user = User {
        email: "staged@example.com".to_owned(),
        activated: true,
        account_locked: false,
        id: "user888".to_owned(),
        state: UserState::Staged,
    };
    let staged_directory_user: DirectoryUser = staged_user.into();
    assert_eq!(staged_directory_user.email, "staged@example.com");
    assert!(!staged_directory_user.active);
    assert_eq!(staged_directory_user.id, Some("user888".to_owned()));

    // Test both inactive and locked user
    let both_user = User {
        email: "both@example.com".to_owned(),
        activated: false,
        account_locked: true,
        id: "user000".to_owned(),
        state: UserState::Activated,
    };
    let both_directory_user: DirectoryUser = both_user.into();
    assert_eq!(both_directory_user.email, "both@example.com");
    assert!(!both_directory_user.active);
    assert_eq!(both_directory_user.id, Some("user000".to_owned()));
}

#[test]
fn test_user_group_to_directory_group_conversions() {
    // Test group with LDAP groups (uses first LDAP group name)
    let group_with_ldap = UserGroup {
        id: "group123".to_owned(),
        compiled_attributes: CompiledAttributes {
            ldap_groups: vec![
                LdapGroup {
                    name: "LDAP Group Name".to_owned(),
                },
                LdapGroup {
                    name: "Second LDAP Group".to_owned(),
                },
            ],
        },
    };
    let directory_group_with_ldap: DirectoryGroup = group_with_ldap.into();
    assert_eq!(directory_group_with_ldap.id, "group123");
    assert_eq!(directory_group_with_ldap.name, "LDAP Group Name");

    // Test group with empty LDAP groups (falls back to group ID)
    let group_empty_ldap = UserGroup {
        id: "group789".to_owned(),
        compiled_attributes: CompiledAttributes {
            ldap_groups: Vec::new(),
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
        results: Vec::new(),
        total_count: 0,
    };
    let empty_directory_users: Vec<DirectoryUser> = empty_users_response.into();
    assert!(empty_directory_users.is_empty());

    // Test single user UsersResponse conversion
    let single_users_response = UsersResponse {
        results: vec![User {
            email: "single@example.com".to_owned(),
            activated: true,
            account_locked: false,
            id: "single123".to_owned(),
            state: UserState::Activated,
        }],
        total_count: 1,
    };
    let single_directory_users: Vec<DirectoryUser> = single_users_response.into();
    assert_eq!(single_directory_users.len(), 1);
    assert_eq!(single_directory_users[0].email, "single@example.com");
    assert!(single_directory_users[0].active);
    assert_eq!(single_directory_users[0].id, Some("single123".to_owned()));

    // Test multiple users with mixed states
    let multiple_users_response = UsersResponse {
        results: vec![
            User {
                email: "user1@example.com".to_owned(),
                activated: true,
                account_locked: false,
                id: "user1".to_owned(),
                state: UserState::Activated,
            },
            User {
                email: "user2@example.com".to_owned(),
                activated: false,
                account_locked: false,
                id: "user2".to_owned(),
                state: UserState::Activated,
            },
            User {
                email: "user3@example.com".to_owned(),
                activated: true,
                account_locked: true,
                id: "user3".to_owned(),
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
                id: "group1".to_owned(),
                name: "Group 1".to_owned(),
            },
            DirectoryGroup {
                id: "group2".to_owned(),
                name: "Group 2".to_owned(),
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

#[tokio::test]
async fn test_get_all_users_paginates() {
    let mock_server = MockServer::start().await;
    let empty_response = ResponseTemplate::new(200)
        .insert_header("content-type", "application/json")
        .set_body_string(r#"{"results":[],"totalCount":0}"#);

    Mock::given(method("GET"))
        .and(path("/api/systemusers"))
        .and(query_param_is_missing("skip"))
        .respond_with(response_from_fixture("users_page1.json"))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/systemusers"))
        .and(query_param("skip", "100"))
        .respond_with(response_from_fixture("users_page2.json"))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/systemusers"))
        .respond_with(empty_response)
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
    let empty_response = ResponseTemplate::new(200)
        .insert_header("content-type", "application/json")
        .set_body_string("[]");

    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups"))
        .and(query_param_is_missing("skip"))
        .respond_with(response_from_fixture("groups_response.json"))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups"))
        .respond_with(empty_response)
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
    let empty_response = ResponseTemplate::new(200)
        .insert_header("content-type", "application/json")
        .set_body_string("[]");

    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups/group1/members"))
        .and(query_param_is_missing("skip"))
        .respond_with(response_from_fixture("group_members_response.json"))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups/group1/members"))
        .respond_with(empty_response)
        .mount(&mock_server)
        .await;

    let dirsync = dirsync_with_mock_server(&mock_server);
    let group = DirectoryGroup {
        id: "group1".into(),
        name: "Engineering".into(),
    };

    let all_users = vec![
        DirectoryUser {
            email: "jane.doe@example.com".into(),
            active: true,
            id: Some("user123".into()),
            user_details: None,
        },
        DirectoryUser {
            email: "john.smith@example.com".into(),
            active: true,
            id: Some("user456".into()),
            user_details: None,
        },
    ];

    let members = dirsync
        .get_group_members(&group, Some(&all_users))
        .await
        .unwrap();

    assert_eq!(members.len(), 2);
    assert!(members.contains(&"jane.doe@example.com".to_string()));
    assert!(members.contains(&"john.smith@example.com".to_string()));
}

#[tokio::test]
async fn test_test_connection() {
    use wiremock::{
        Mock, MockServer,
        matchers::{method, path},
    };

    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/systemusers"))
        .respond_with(response_from_fixture("users_empty.json"))
        .mount(&mock_server)
        .await;

    let dirsync = dirsync_with_mock_server(&mock_server);
    dirsync.test_connection().await.unwrap();
}
