//! Drives every directory sync provider (Google, Okta, JumpCloud, Microsoft) through the
//! shared `DirectorySync` trait interface, each backed by its own wiremock `MockServer` and
//! real-response fixtures. Verifies that the common dispatch (`DirectorySyncClient`) works
//! consistently across providers.

use std::collections::HashSet;

use wiremock::{
    Mock, MockServer,
    matchers::{method, path, query_param, query_param_is_missing},
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

/// Replicates the `directory_sync_user_groups` filter from [`super::do_directory_sync`]:
/// resolve the configured group names to their members and return the set of emails
/// allowed to sync.
async fn allowed_emails_for_group_filter(
    client: &DirectorySyncClient,
    user_groups_filter: &[String],
) -> HashSet<String> {
    let all_users = client.get_all_users().await.unwrap();
    let groups = client.get_groups().await.unwrap();
    let mut emails = HashSet::new();
    for group in groups
        .iter()
        .filter(|group| user_groups_filter.contains(&group.name))
    {
        let members = client
            .get_group_members(group, Some(&all_users))
            .await
            .unwrap();
        emails.extend(members);
    }
    emails
}

/// Every provider's fixtures agree on two directory users: "jane.doe@example.com", a member
/// of the "Engineering" group, and "john.smith@example.com", who isn't. Verifies that limiting
/// sync to "Engineering" allows jane.doe and excludes john.smith for all four providers.
#[tokio::test]
async fn test_group_limiting_all_providers() {
    let engineering = ["Engineering".to_owned()];

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
        .and(query_param_is_missing("pageToken"))
        .respond_with(google::response_from_fixture("users_page1.json"))
        .mount(&google_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users"))
        .and(query_param("pageToken", "EAIaBhACGgtGkAAfirstpage"))
        .respond_with(google::response_from_fixture("users_page2.json"))
        .mount(&google_server)
        .await;
    let google_client =
        DirectorySyncClient::Google(google::dirsync_with_mock_server(&google_server));
    let allowed = allowed_emails_for_group_filter(&google_client, &engineering).await;
    assert_eq!(
        allowed,
        HashSet::from(["jane.doe@example.com".to_owned()]),
        "Google: expected only jane.doe to be allowed by the Engineering group filter"
    );

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
        .and(query_param_is_missing("after"))
        .respond_with(okta::response_from_fixture("users_page1.json"))
        .mount(&okta_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/users"))
        .and(query_param("after", "page1"))
        .respond_with(okta::response_from_fixture("users_page2.json"))
        .mount(&okta_server)
        .await;
    let okta_client = DirectorySyncClient::Okta(okta::dirsync_with_mock_server(&okta_server));
    let allowed = allowed_emails_for_group_filter(&okta_client, &engineering).await;
    assert_eq!(
        allowed,
        HashSet::from(["jane.doe@example.com".to_owned()]),
        "Okta: expected only jane.doe to be allowed by the Engineering group filter"
    );

    // Microsoft
    let microsoft_server = MockServer::start().await;
    let microsoft_uri = microsoft_server.uri();
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
        .and(query_param_is_missing("$skiptoken"))
        .respond_with(microsoft::response_from_fixture_with_mock_uri(
            "users_page1.json",
            &microsoft_uri,
        ))
        .mount(&microsoft_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/users"))
        .and(query_param("$skiptoken", "firstpage"))
        .respond_with(microsoft::response_from_fixture("users_page2.json"))
        .mount(&microsoft_server)
        .await;
    let microsoft_client =
        DirectorySyncClient::Microsoft(microsoft::dirsync_with_mock_server(&microsoft_server));
    let allowed = allowed_emails_for_group_filter(&microsoft_client, &engineering).await;
    assert_eq!(
        allowed,
        HashSet::from(["jane.doe@example.com".to_owned()]),
        "Microsoft: expected only jane.doe to be allowed by the Engineering group filter"
    );

    // JumpCloud
    let jumpcloud_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups"))
        .respond_with(jumpcloud::response_from_fixture("groups_response.json"))
        .mount(&jumpcloud_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups/group1/members"))
        .and(query_param_is_missing("skip"))
        .respond_with(jumpcloud::response_from_fixture(
            "group_members_response.json",
        ))
        .mount(&jumpcloud_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/usergroups/group1/members"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string("[]"),
        )
        .mount(&jumpcloud_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/systemusers"))
        .and(query_param_is_missing("skip"))
        .respond_with(jumpcloud::response_from_fixture("users_page1.json"))
        .mount(&jumpcloud_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/systemusers"))
        .and(query_param("skip", "100"))
        .respond_with(jumpcloud::response_from_fixture("users_page2.json"))
        .mount(&jumpcloud_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/systemusers"))
        .respond_with(jumpcloud::response_from_fixture("users_empty.json"))
        .mount(&jumpcloud_server)
        .await;
    let jumpcloud_client =
        DirectorySyncClient::JumpCloud(jumpcloud::dirsync_with_mock_server(&jumpcloud_server));
    let allowed = allowed_emails_for_group_filter(&jumpcloud_client, &engineering).await;
    assert_eq!(
        allowed,
        HashSet::from(["jane.doe@example.com".to_owned()]),
        "JumpCloud: expected only jane.doe to be allowed by the Engineering group filter \
         (john.smith is a group member but inactive, so get_group_members excludes him)"
    );
}
