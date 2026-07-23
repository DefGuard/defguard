//! Drives every directory sync provider (Google, Okta, JumpCloud, Microsoft) through the
//! shared `DirectorySync` trait interface, each backed by its own wiremock `MockServer` and
//! real-response fixtures. Verifies that the common dispatch (`DirectorySyncClient`) works
//! consistently across providers.

use wiremock::{
    Mock, MockServer,
    matchers::{method, path},
};

use super::{
    DirectoryGroup, DirectorySync, DirectorySyncClient, google, jumpcloud, microsoft, okta,
};

/// Drives a provider client through the full shared `DirectorySync` interface and asserts on
/// the "Engineering" group and its "jane.doe@example.com" member, which every provider's
/// fixtures agree on.
async fn assert_provider_dirsync(
    name: &str,
    mut client: DirectorySyncClient,
    group_id: &str,
    all_users_helper: Option<&[super::DirectoryUser]>,
) {
    client
        .prepare()
        .await
        .unwrap_or_else(|err| panic!("{name}: prepare() failed: {err}"));

    client
        .test_connection()
        .await
        .unwrap_or_else(|err| panic!("{name}: test_connection() failed: {err}"));

    let groups = client
        .get_groups()
        .await
        .unwrap_or_else(|err| panic!("{name}: get_groups() failed: {err}"));
    assert!(
        groups.iter().any(|g| g.name == "Engineering"),
        "{name}: expected an 'Engineering' group, got {groups:?}"
    );

    let group = DirectoryGroup {
        id: group_id.to_owned(),
        name: "Engineering".to_owned(),
    };
    let members = client
        .get_group_members(&group, all_users_helper)
        .await
        .unwrap_or_else(|err| panic!("{name}: get_group_members() failed: {err}"));
    assert!(
        members.contains(&"jane.doe@example.com".to_string()),
        "{name}: expected jane.doe@example.com among group members, got {members:?}"
    );
}

#[tokio::test]
async fn test_all_providers() {
    // Google
    let google_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/groups"))
        .respond_with(google::response_from_fixture("groups_response.json"))
        .mount(&google_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/groups/01302m9251m2vt3/members"))
        .respond_with(google::response_from_fixture("members_response.json"))
        .mount(&google_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(google::response_from_fixture("users_empty.json"))
        .mount(&google_server)
        .await;
    let google_client =
        DirectorySyncClient::Google(google::dirsync_with_mock_server(&google_server));
    assert_provider_dirsync("Google", google_client, "01302m9251m2vt3", None).await;

    // Okta
    let okta_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/groups"))
        .respond_with(okta::response_from_fixture("groups_response.json"))
        .mount(&okta_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/groups/00gjitxyt9yJW2FKR0g7/users"))
        .respond_with(okta::response_from_fixture("group_members_response.json"))
        .mount(&okta_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/users"))
        .respond_with(okta::response_from_fixture("users_page1.json"))
        .mount(&okta_server)
        .await;
    let okta_client = DirectorySyncClient::Okta(okta::dirsync_with_mock_server(&okta_server));
    assert_provider_dirsync("Okta", okta_client, "00gjitxyt9yJW2FKR0g7", None).await;

    // Microsoft
    let microsoft_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/groups"))
        .respond_with(microsoft::response_from_fixture("groups_response.json"))
        .mount(&microsoft_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/groups/11111111-1111-1111-1111-111111111111/members"))
        .respond_with(microsoft::response_from_fixture("members_response.json"))
        .mount(&microsoft_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users"))
        .respond_with(microsoft::response_from_fixture("users_empty.json"))
        .mount(&microsoft_server)
        .await;
    let microsoft_client =
        DirectorySyncClient::Microsoft(microsoft::dirsync_with_mock_server(&microsoft_server));
    assert_provider_dirsync(
        "Microsoft",
        microsoft_client,
        "11111111-1111-1111-1111-111111111111",
        None,
    )
    .await;

    // JumpCloud (needs the all_users helper to map member ids to emails)
    let jumpcloud_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups"))
        .respond_with(jumpcloud::response_from_fixture("groups_response.json"))
        .mount(&jumpcloud_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups/group1/members"))
        .respond_with(jumpcloud::response_from_fixture(
            "group_members_response.json",
        ))
        .mount(&jumpcloud_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/systemusers"))
        .respond_with(jumpcloud::response_from_fixture("users_empty.json"))
        .mount(&jumpcloud_server)
        .await;
    let all_users = vec![
        super::DirectoryUser {
            id: Some("user123".into()),
            email: "jane.doe@example.com".into(),
            active: true,
            user_details: None,
        },
        super::DirectoryUser {
            id: Some("user456".into()),
            email: "john.smith@example.com".into(),
            active: true,
            user_details: None,
        },
    ];
    let jumpcloud_client =
        DirectorySyncClient::JumpCloud(jumpcloud::dirsync_with_mock_server(&jumpcloud_server));
    assert_provider_dirsync("JumpCloud", jumpcloud_client, "group1", Some(&all_users)).await;
}
