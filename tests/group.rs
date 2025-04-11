pub mod common;

use common::setup_pool;
use defguard::handlers::{Auth, EditGroupInfo, GroupInfo};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use self::common::make_test_client;

#[sqlx::test]
async fn test_create_group(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create new group.
    let data = EditGroupInfo::new("hogwards", vec!["hpotter".into()], false);
    let response = client.post("/api/v1/group").json(&data).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Try to create the same group again.
    let response = client.post("/api/v1/group").json(&data).send().await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    // Delete the group.
    let response = client.delete("/api/v1/group/hogwards").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Try to delete again.
    let response = client.delete("/api/v1/group/hogwards").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_modify_group(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create new group.
    let data = EditGroupInfo::new("hogwards", vec!["hpotter".into()], false);
    let response = client.post("/api/v1/group").json(&data).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Rename group.
    let data = EditGroupInfo::new("gryffindor", Vec::new(), false);
    let response = client
        .put("/api/v1/group/hogwards")
        .json(&data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Try to get the group by its old name.
    let response = client.get("/api/v1/group/hogwards").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Get group info.
    let response = client.get("/api/v1/group/gryffindor").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let group_info: GroupInfo = response.json().await;
    assert_eq!(group_info.name, "gryffindor");
}

#[sqlx::test]
async fn test_modify_group_members(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create new group.
    let data = EditGroupInfo::new("hogwards", vec!["hpotter".into()], false);
    let response = client.post("/api/v1/group").json(&data).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Get group info.
    let response = client.get("/api/v1/group/hogwards").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let group_info: GroupInfo = response.json().await;
    assert_eq!(group_info.members, vec!["hpotter".to_string()]);

    // Change group members.
    let data = EditGroupInfo::new("hogwards", Vec::new(), false);
    let response = client
        .put("/api/v1/group/hogwards")
        .json(&data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Get group info.
    let response = client.get("/api/v1/group/hogwards").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let group_info: GroupInfo = response.json().await;
    assert!(group_info.members.is_empty());
}

#[sqlx::test]
async fn test_modify_group_no_locations_in_request(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Create new group.
    let data = json!({
        "name": "hogwards",
        "members": [
            "hpotter",
            "admin"
        ],
        "is_admin": false
    });
    let response = client.post("/api/v1/group").json(&data).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // Rename group.
    let data = json!({
        "name": "gryffindor",
        "members": [
            "hpotter",
        ],
        "is_admin": false
    });
    let response = client
        .put("/api/v1/group/hogwards")
        .json(&data)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // Try to get the group by its old name.
    let response = client.get("/api/v1/group/hogwards").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Get group info.
    let response = client.get("/api/v1/group/gryffindor").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let group_info: GroupInfo = response.json().await;
    assert_eq!(group_info.name, "gryffindor");
    assert_eq!(group_info.members, vec!["hpotter"]);
}

#[sqlx::test]
async fn test_remove_last_admin_group(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Get group info.
    let response = client.get("/api/v1/group/admin").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let group_info: GroupInfo = response.json().await;
    assert_eq!(group_info.members, vec!["admin".to_string()]);

    let response = client.delete("/api/v1/group/admin").send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_modify_last_admin_group(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

    // Authorize as an administrator.
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // Get group info.
    let response = client.get("/api/v1/group/admin").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let group_info: GroupInfo = response.json().await;
    assert_eq!(group_info.members, vec!["admin".to_string()]);
    // try to remove admin status from the last group
    let data = json!({
        "name": "admin",
        "members": [
            "admin",
        ],
        "is_admin": false
    });
    let response = client.put("/api/v1/group/admin").json(&data).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
