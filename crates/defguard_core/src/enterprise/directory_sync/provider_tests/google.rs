use wiremock::{
    Mock, MockServer,
    matchers::{method, path, query_param, query_param_is_missing},
};

use super::*;

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
            .any(|u| u.email == "jane.doe@example.com"
                && u.active
                && u.id.as_deref() == Some("108234567890123456789"))
    );
    assert!(
        users
            .iter()
            .any(|u| u.email == "john.smith@example.com"
                && !u.active
                && u.id.as_deref() == Some("108234567890123456790"))
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

    assert_eq!(members, ["jane.doe@example.com".to_string()]);
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
                id: "1".into(),
                primary_email: "email@email.com".into(),
                suspended: false,
            },
            User {
                id: "2".into(),
                primary_email: "email2@email.com".into(),
                suspended: true,
            },
            User {
                id: "3".into(),
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
    assert_eq!(disabled_user.id, Some("2".to_owned()));
}
