use wiremock::{
    Mock, MockServer,
    matchers::{method, path, query_param, query_param_is_missing},
};

use super::*;

#[tokio::test]
async fn test_refresh_access_token() {
    let mock_server = MockServer::start().await;
    // Real response shape from Okta OAuth 2.0 token endpoint.
    Mock::given(method("POST"))
        .and(path("/oauth2/v1/token"))
        .respond_with(response_from_fixture("token_response.json"))
        .mount(&mock_server)
        .await;

    let mut dirsync =
        OktaDirectorySync::new(TEST_JWK_PRIVATE_KEY, "test_client_id", &mock_server.uri());

    dirsync.refresh_access_token().await.unwrap();

    assert_eq!(
        dirsync.access_token.as_deref(),
        Some(
            "eyJhbGciOiJSUzI1NiIsImtpZCI6IlRlc3RAMjAyNCJ9.eyJpc3MiOiJodHRwczovL3RyaWFsLW9rdGEuY29tIiwiYXV0IjoiaW50ZXJuYWwiLCJhdWQiOiJodHRwczovL3RyaWFsLW9rdGEuY29tL2FwaS92MS91c2VycyIsInN1YiI6ImM0MDFuMjYwbnNvZkpiMDBk"
        )
    );
    assert!(!dirsync.is_token_expired());
}

#[tokio::test]
async fn test_get_all_users_paginates() {
    let mock_server = MockServer::start().await;
    let server_uri = mock_server.uri();
    // Real response shape from Okta users.list; Link header uses RFC 5988 format
    // with `rel="next"` pointing to the next page URL.
    let link_header = format!("<{server_uri}/api/v1/users?limit=200&after=page1>; rel=\"next\"");
    Mock::given(method("GET"))
        .and(path("/api/v1/users"))
        .and(query_param_is_missing("after"))
        .respond_with(response_from_fixture("users_page1.json").append_header("link", link_header))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/users"))
        .and(query_param("after", "page1"))
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
    // Real response shape from Okta groups.list.
    Mock::given(method("GET"))
        .and(path("/api/v1/groups"))
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
    // Real response shape from Okta groups/{groupId}/users.
    Mock::given(method("GET"))
        .and(path("/api/v1/groups/00gjitxyt9yJW2FKR0g7/users"))
        .respond_with(response_from_fixture("group_members_response.json"))
        .mount(&mock_server)
        .await;

    let dirsync = dirsync_with_mock_server(&mock_server);
    let group = DirectoryGroup {
        id: "00gjitxyt9yJW2FKR0g7".into(),
        name: "Engineering".into(),
    };
    let members = dirsync.get_group_members(&group, None).await.unwrap();

    assert_eq!(members, ["jane.doe@example.com".to_string()]);
}

#[tokio::test]
async fn test_test_connection() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/users"))
        .respond_with(response_from_fixture("users_page1.json"))
        .mount(&mock_server)
        .await;

    let dirsync = dirsync_with_mock_server(&mock_server);
    dirsync.test_connection().await.unwrap();
}

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
        "<https://trial-0000000.okta.com/api/v1/users?after=4&limit=200>; rel=\"next\"".to_owned();
    let next_link = extract_next_link(Some(&link_header)).unwrap();
    assert_eq!(
        next_link,
        Some("https://trial-0000000.okta.com/api/v1/users?after=4&limit=200".to_owned())
    );

    let next_link = extract_next_link(None).unwrap();
    assert_eq!(next_link, None);

    let link_header = "invalid".to_owned();
    let next_link = extract_next_link(Some(&link_header));
    assert!(next_link.is_err());

    let link_header = "<https://trial-0000000.okta.com/api/v1/users?after=4&limit=200>; rel=\"next\", <https://trial-0000000.okta.com/api/v1/users?after=4&limit=200>; rel=\"prev\"".to_owned();
    let next_link = extract_next_link(Some(&link_header)).unwrap();
    assert_eq!(
        next_link,
        Some("https://trial-0000000.okta.com/api/v1/users?after=4&limit=200".to_owned())
    );
}

#[tokio::test]
async fn test_group_parse() {
    let group = Group {
        id: "test_id".to_owned(),
        profile: GroupProfile {
            name: "test_name".to_owned(),
        },
    };
    let dir_group: DirectoryGroup = group.into();
    assert_eq!(dir_group.id, "test_id");
    assert_eq!(dir_group.name, "test_name");
}

#[tokio::test]
async fn test_user_parse() {
    let user = User {
        id: "test_id".to_owned(),
        status: "ACTIVE".to_owned(),
        profile: UserProfile {
            email: "test_email".to_owned(),
        },
    };

    let dir_user: DirectoryUser = user.into();
    assert_eq!(dir_user.email, "test_email");
    assert_eq!(dir_user.id, Some("test_id".to_owned()));
    assert!(dir_user.active);

    let user = User {
        id: "test_id".to_owned(),
        status: "INACTIVE".to_owned(),
        profile: UserProfile {
            email: "test_email".to_owned(),
        },
    };

    let dir_user: DirectoryUser = user.into();
    assert_eq!(dir_user.email, "test_email");
    assert!(!dir_user.active);
}
