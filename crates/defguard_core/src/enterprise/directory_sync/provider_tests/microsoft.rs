use wiremock::{
    Mock, MockServer,
    matchers::{method, path, query_param, query_param_is_missing},
};

use super::*;

#[tokio::test]
async fn test_refresh_access_token() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(response_from_fixture("token_response.json"))
        .mount(&mock_server)
        .await;

    let mut dirsync = MicrosoftDirectorySync::new(
        "client_id".into(),
        "client_secret".into(),
        "https://login.microsoftonline.com/tenant-123/v2.0".into(),
        Vec::new(),
    )
    .with_urls(&format!("{}/token", mock_server.uri()), "");

    dirsync.refresh_access_token().await.unwrap();

    assert_eq!(dirsync.access_token.as_deref(), Some("EwAoA8l6BAAR..."));
    assert!(!dirsync.is_token_expired());
}

#[tokio::test]
async fn test_get_all_users_paginates() {
    let mock_server = MockServer::start().await;
    let mock_uri = mock_server.uri();
    Mock::given(method("GET"))
        .and(path("/users"))
        .and(query_param_is_missing("$skiptoken"))
        .respond_with(response_from_fixture_with_mock_uri(
            "users_page1.json",
            &mock_uri,
        ))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users"))
        .and(query_param("$skiptoken", "firstpage"))
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
    Mock::given(method("GET"))
        .and(path("/groups/11111111-1111-1111-1111-111111111111/members"))
        .respond_with(response_from_fixture("members_response.json"))
        .mount(&mock_server)
        .await;

    let dirsync = dirsync_with_mock_server(&mock_server);
    let group = DirectoryGroup {
        id: "11111111-1111-1111-1111-111111111111".into(),
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

#[test]
fn test_extract_tenant() {
    let provider = MicrosoftDirectorySync::new(
        "client_id".to_owned(),
        "client_secret".to_owned(),
        "https://login.microsoftonline.com/tenant-id-123/v2.0".to_owned(),
        Vec::new(),
    );
    let tenant = provider.extract_tenant().unwrap();
    assert_eq!(tenant, "tenant-id-123");
}

#[tokio::test]
async fn test_token() {
    let mut dirsync = MicrosoftDirectorySync::new(
        "id".to_owned(),
        "secret".to_owned(),
        "https://login.microsoftonline.com/tenant-id-123/v2.0".to_owned(),
        Vec::new(),
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
}

#[tokio::test]
async fn test_groups_parse() {
    let groups_response = GroupsResponse {
        next_page: None,
        value: vec![
            GroupDetails {
                display_name: Some("Group 1".to_owned()),
                id: "1".to_owned(),
            },
            GroupDetails {
                display_name: Some("Group 2".to_owned()),
                id: "2".to_owned(),
            },
        ],
    };

    let groups: Vec<DirectoryGroup> = groups_response.into();

    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].name, "Group 1");
    assert_eq!(groups[0].id, "1");
    assert_eq!(groups[1].name, "Group 2");
    assert_eq!(groups[1].id, "2");
}

#[tokio::test]
async fn test_members_parse() {
    let members_response = GroupMembersResponse {
        next_page: None,
        value: vec![
            User {
                display_name: "User 1".to_owned(),
                mail: Some("email@email.com".to_owned()),
                account_enabled: true,
                other_mails: Vec::new(),
                id: "user1-id".into(),
                given_name: Some("User".into()),
                surname: Some("One".into()),
                mobile_phone: Some("555555555".into()),
                business_phones: Vec::new(),
            },
            User {
                display_name: "User 2".to_owned(),
                mail: None,
                account_enabled: true,
                other_mails: vec!["email2@email.com".to_owned()],
                id: "user2-id".into(),
                given_name: Some("User".into()),
                surname: Some("Two".into()),
                mobile_phone: None,
                business_phones: Vec::new(),
            },
            User {
                display_name: "User 3".to_owned(),
                mail: None,
                account_enabled: true,
                other_mails: Vec::new(),
                id: "user3-id".into(),
                given_name: Some("User".into()),
                surname: Some("Three".into()),
                mobile_phone: None,
                business_phones: Vec::new(),
            },
        ],
    };

    let members: Vec<String> = members_response.into();
    assert_eq!(members.len(), 2);
    assert_eq!(members[0], "email@email.com".to_owned());
    assert_eq!(members[1], "email2@email.com".to_owned());
}

#[tokio::test]
async fn test_users_parse() {
    let users_response = UsersResponse {
        next_page: None,
        value: vec![
            User {
                display_name: "User 1".to_owned(),
                mail: Some("email@email.com".to_owned()),
                account_enabled: true,
                other_mails: Vec::new(),
                id: "user1-id".into(),
                given_name: Some("User".into()),
                surname: None,
                mobile_phone: None,
                business_phones: Vec::new(),
            },
            User {
                display_name: "User 2".to_owned(),
                mail: None,
                account_enabled: true,
                other_mails: vec!["email2@email.com".to_owned()],
                id: "user2-id".into(),
                given_name: None,
                surname: None,
                mobile_phone: Some("555555555".into()),
                business_phones: Vec::new(),
            },
            User {
                display_name: "User 3".to_owned(),
                mail: None,
                account_enabled: true,
                other_mails: Vec::new(),
                id: "user3-id".into(),
                given_name: Some("User".into()),
                surname: Some("Three".into()),
                mobile_phone: Some("555555555".into()),
                business_phones: Vec::new(),
            },
        ],
    };

    let users: Vec<DirectoryUser> = users_response.into();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].email, "email@email.com".to_owned());
    assert_eq!(users[1].email, "email2@email.com".to_owned());
}
