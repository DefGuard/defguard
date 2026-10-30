use defguard_common::db::Id;
use defguard_core::handlers::{AddUserData, EditGroupInfo};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{client::TestClient, get_db_user, make_client_with_db, setup_pool};

const HPOTTER_KEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICDhISvVRBU0s+Mmmk9YsWNbVQc6JnNyVWDX/84/gX2s hpotter";
const ADUMBLEDORE_KEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICv9cnGWcYuVpyZ6/TVDuE7/MDRaKPDgbi+LMxTX0jQG adumbledore";

async fn add_ssh_key(client: &TestClient, username: &str, key: &str) {
    let response = client
        .post(format!("/api/v1/user/{username}/auth_key"))
        .json(&json!({"key": key, "name": "laptop", "key_type": "ssh"}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

async fn set_users_active(client: &TestClient, user_ids: &[Id], active: bool) {
    let path = if active {
        "/api/v1/user/bulk-enable"
    } else {
        "/api/v1/user/bulk-disable"
    };
    let response = client
        .post(path)
        .json(&json!({ "users": user_ids }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

async fn authorized_keys(client: &TestClient, query: &str) -> String {
    let response = client
        .get(format!("/api/v1/ssh_authorized_keys?{query}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    response.text().await
}

/// Logs in as admin and sets up `hpotter` and `adumbledore`, each with an SSH key and both in
/// the `ssh-users` group.
async fn setup(options: PgConnectOptions) -> (TestClient, sqlx::PgPool) {
    let pool = setup_pool(options).await;
    let (mut client, pool) = make_client_with_db(pool).await;
    client.login_user("admin", "pass123").await;

    let new_user = AddUserData {
        username: "adumbledore".into(),
        last_name: "Dumbledore".into(),
        first_name: "Albus".into(),
        email: "a.dumbledore@hogwart.edu.uk".into(),
        phone: None,
        password: Some("Password1234543$!".into()),
    };
    let response = client.post("/api/v1/user").json(&new_user).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    add_ssh_key(&client, "hpotter", HPOTTER_KEY).await;
    add_ssh_key(&client, "adumbledore", ADUMBLEDORE_KEY).await;

    let group = EditGroupInfo::new(
        "ssh-users",
        vec!["hpotter".into(), "adumbledore".into()],
        false,
    );
    let response = client.post("/api/v1/group").json(&group).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    (client, pool)
}

#[sqlx::test]
async fn test_authorized_keys_by_username_skip_disabled_user(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (client, pool) = setup(options).await;
    let user = get_db_user(&pool, "adumbledore").await;

    assert_eq!(
        authorized_keys(&client, "username=adumbledore").await,
        ADUMBLEDORE_KEY
    );

    set_users_active(&client, &[user.id], false).await;
    assert_eq!(authorized_keys(&client, "username=adumbledore").await, "");

    set_users_active(&client, &[user.id], true).await;
    assert_eq!(
        authorized_keys(&client, "username=adumbledore").await,
        ADUMBLEDORE_KEY
    );
}

#[sqlx::test]
async fn test_authorized_keys_by_group_skip_disabled_members(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (client, pool) = setup(options).await;
    let user = get_db_user(&pool, "adumbledore").await;

    let keys = authorized_keys(&client, "group=ssh-users").await;
    let mut keys: Vec<&str> = keys.lines().collect();
    keys.sort_unstable();
    assert_eq!(keys, [HPOTTER_KEY, ADUMBLEDORE_KEY]);

    set_users_active(&client, &[user.id], false).await;
    assert_eq!(
        authorized_keys(&client, "group=ssh-users").await,
        HPOTTER_KEY
    );
}

#[sqlx::test]
async fn test_authorized_keys_by_group_and_username_skip_disabled_member(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (client, pool) = setup(options).await;
    let user = get_db_user(&pool, "adumbledore").await;

    assert_eq!(
        authorized_keys(&client, "group=ssh-users&username=adumbledore").await,
        ADUMBLEDORE_KEY
    );

    set_users_active(&client, &[user.id], false).await;
    assert_eq!(
        authorized_keys(&client, "group=ssh-users&username=adumbledore").await,
        ""
    );
}

#[sqlx::test]
async fn test_authorized_keys_by_group_and_username_skip_non_member(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (client, _pool) = setup(options).await;

    let group = EditGroupInfo::new("other", vec!["hpotter".into()], false);
    let response = client.post("/api/v1/group").json(&group).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    assert_eq!(
        authorized_keys(&client, "group=other&username=adumbledore").await,
        ""
    );
    assert_eq!(
        authorized_keys(&client, "group=other&username=hpotter").await,
        HPOTTER_KEY
    );
}

// The response for a disabled user must not differ from the one for an unknown user, so the
// endpoint does not reveal which accounts exist but are disabled.
#[sqlx::test]
async fn test_authorized_keys_disabled_user_matches_unknown_user(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (client, pool) = setup(options).await;
    let user = get_db_user(&pool, "adumbledore").await;
    set_users_active(&client, &[user.id], false).await;

    let disabled = client
        .get("/api/v1/ssh_authorized_keys?username=adumbledore")
        .send()
        .await;
    let unknown = client
        .get("/api/v1/ssh_authorized_keys?username=nobody")
        .send()
        .await;
    assert_eq!(disabled.status(), unknown.status());
    assert_eq!(disabled.text().await, unknown.text().await);
}

// sshd calls this endpoint as an `AuthorizedKeysCommand`, which has no Defguard session.
#[sqlx::test]
async fn test_authorized_keys_does_not_require_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (client, _pool) = setup(options).await;

    let response = client.post("/api/v1/auth/logout").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        authorized_keys(&client, "username=hpotter").await,
        HPOTTER_KEY
    );
}
