use common::{client::TestClient, init_config, init_test_db, omit_id};
use defguard::{
    config::DefGuardConfig,
    db::{models::device::DeviceType, Device, Group, Id, NoId, User, WireguardNetwork},
    enterprise::{
        db::models::acl::AclAlias,
        handlers::acl::{ApiAclAlias, ApiAclRule},
    },
    handlers::Auth,
};
use reqwest::StatusCode;
use sqlx::PgPool;
use tokio::net::TcpListener;

use self::common::{make_base_client, make_test_client};

pub mod common;

async fn make_client_v2(pool: PgPool, config: DefGuardConfig) -> TestClient {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Could not bind ephemeral socket");
    let (client, _) = make_base_client(pool, config, listener).await;
    client
}

async fn authenticate(client: &TestClient) {
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

fn make_rule() -> ApiAclRule {
    ApiAclRule {
        id: NoId,
        parent_id: Default::default(),
        state: Default::default(),
        name: "rule".to_string(),
        all_networks: false,
        networks: vec![],
        expires: None,
        allow_all_users: false,
        deny_all_users: false,
        allowed_users: vec![],
        denied_users: vec![],
        allowed_groups: vec![],
        denied_groups: vec![],
        allowed_devices: vec![],
        denied_devices: vec![],
        destination: "10.2.2.2, 10.0.0.1/24, 10.0.10.1-10.0.20.1".to_string(),
        aliases: vec![],
        enabled: true,
        protocols: vec![6, 17],
        ports: "1, 2, 3, 10-20, 30-40".to_string(),
    }
}

fn make_alias() -> ApiAclAlias {
    ApiAclAlias {
        id: NoId,
        name: "alias".to_string(),
        destination: "10.2.2.2, 10.0.0.1/24, 10.0.10.1-10.0.20.1".to_string(),
        protocols: vec![6, 17],
        ports: "1, 2, 3, 10-20, 30-40".to_string(),
    }
}

#[tokio::test]
async fn test_rule_crud() {
    let (client, _) = make_test_client().await;
    authenticate(&client).await;

    let rule = make_rule();

    // create
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_rule: ApiAclRule<NoId> = omit_id(response.json().await);
    assert_eq!(response_rule, rule);

    // list
    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rules: Vec<serde_json::Value> = response.json().await;
    assert_eq!(response_rules.len(), 1);
    let response_rule: ApiAclRule<NoId> = omit_id(response_rules[0].clone());
    assert_eq!(response_rule, rule);

    // retrieve
    let response = client.get("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rule: ApiAclRule<NoId> = omit_id(response.json().await);
    assert_eq!(response_rule, rule);

    // update
    let mut rule: ApiAclRule<Id> = client.get("/api/v1/acl/rule/1").send().await.json().await;
    rule.name = "modified".to_string();
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rule: ApiAclRule<Id> = response.json().await;
    assert_eq!(response_rule, rule);
    let response_rule: ApiAclRule<Id> = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(response_rule, rule);

    // delete
    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rules: Vec<serde_json::Value> = response.json().await;
    assert_eq!(response_rules.len(), 0);
}

// FIXME: race conditions on global statics in integration tests
// #[tokio::test]
// async fn test_rule_enterprise() {
//     let (client, _) = make_test_client().await;
//     authenticate(&client).await;

//     exceed_enterprise_limits(&client).await;

//     // unset the license
//     let license = get_cached_license().clone();
//     set_cached_license(None);

//     // try to use ACL api
//     let rule = make_rule();
//     let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
//     assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
//     assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     let response = client.get("/api/v1/acl/rule").send().await;
//     assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     let response = client.delete("/api/v1/acl/rule/1").send().await;
//     assert_eq!(response.status(), StatusCode::FORBIDDEN);

//     // restore valid license and try again
//     set_cached_license(license);
//     let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
//     assert_eq!(response.status(), StatusCode::CREATED);
//     let response = client.get("/api/v1/acl/rule").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     let response_rules: Vec<serde_json::Value> = response.json().await;
//     assert_eq!(response_rules.len(), 1);
//     let response = client.get("/api/v1/acl/rule").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     let rule: ApiAclRule<Id> = client.get("/api/v1/acl/rule/1").send().await.json().await;
//     let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     let response = client.delete("/api/v1/acl/rule/1").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
// }

#[tokio::test]
async fn test_alias_crud() {
    let (client, _) = make_test_client().await;
    authenticate(&client).await;

    let alias = make_alias();

    // create
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_alias: ApiAclAlias<NoId> = omit_id(response.json().await);
    assert_eq!(response_alias, alias);

    // list
    let response = client.get("/api/v1/acl/alias").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_aliases: Vec<serde_json::Value> = response.json().await;
    assert_eq!(response_aliases.len(), 1);
    let response_alias: ApiAclAlias<NoId> = omit_id(response_aliases[0].clone());
    assert_eq!(response_alias, alias);

    // retrieve
    let response = client.get("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_alias: ApiAclAlias<NoId> = omit_id(response.json().await);
    assert_eq!(response_alias, alias);

    // update
    let mut alias: ApiAclAlias<Id> = client.get("/api/v1/acl/alias/1").send().await.json().await;
    alias.name = "modified".to_string();
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_alias: ApiAclAlias<Id> = response.json().await;
    assert_eq!(response_alias, alias);
    let response_alias: ApiAclAlias<Id> =
        client.get("/api/v1/acl/alias/1").send().await.json().await;
    assert_eq!(response_alias, alias);

    // delete
    let response = client.delete("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = client.get("/api/v1/acl/alias").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_aliases: Vec<serde_json::Value> = response.json().await;
    assert_eq!(response_aliases.len(), 0);
}

// FIXME: race conditions on global statics in integration tests
// #[tokio::test]
// async fn test_alias_enterprise() {
//     let (client, _) = make_test_client().await;
//     authenticate(&client).await;

//     exceed_enterprise_limits(&client).await;

//     // unset the license
//     let license = get_cached_license().clone();
//     set_cached_license(None);

//     // try to use ACL api
//     let alias = make_alias();
//     let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
//     assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
//     assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     let response = client.get("/api/v1/acl/alias").send().await;
//     assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     let response = client.delete("/api/v1/acl/alias/1").send().await;
//     assert_eq!(response.status(), StatusCode::FORBIDDEN);

//     // restore valid license and try again
//     set_cached_license(license);
//     let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
//     assert_eq!(response.status(), StatusCode::CREATED);
//     let response = client.get("/api/v1/acl/alias").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     let response_aliases: Vec<serde_json::Value> = response.json().await;
//     assert_eq!(response_aliases.len(), 1);
//     let response = client.get("/api/v1/acl/alias").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     let alias: ApiAclAlias<Id> = client.get("/api/v1/acl/alias/1").send().await.json().await;
//     let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
//     assert_eq!(response.status(), StatusCode::OK);
//     let response = client.delete("/api/v1/acl/alias/1").send().await;
//     assert_eq!(response.status(), StatusCode::OK);
// }

#[tokio::test]
async fn test_empty_strings() {
    let (client, _) = make_test_client().await;
    authenticate(&client).await;

    // rule
    let mut rule = make_rule();
    rule.destination = String::new();
    rule.ports = String::new();

    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_rule: ApiAclRule<NoId> = omit_id(response.json().await);
    assert_eq!(response_rule, rule);

    // alias
    let mut alias = make_alias();
    alias.destination = String::new();
    alias.ports = String::new();

    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_alias: ApiAclAlias<NoId> = omit_id(response.json().await);
    assert_eq!(response_alias, alias);
}

#[tokio::test]
async fn test_nonadmin() {
    let (client, _) = make_test_client().await;

    let auth = Auth::new("hpotter", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // rule
    let rule = make_rule();

    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.get("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.get("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.delete("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // alias
    let alias = make_alias();

    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.get("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.get("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client
        .delete("/api/v1/acl/alias/1")
        .json(&alias)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_related_objects() {
    let config = init_config(None);
    let pool = init_test_db(&config).await;
    let client = make_client_v2(pool.clone(), config).await;
    authenticate(&client).await;

    // create related objects
    // networks
    for net in ["net 1", "net 2"] {
        WireguardNetwork::new(
            net.to_string(),
            Vec::new(),
            1000,
            "endpoint1".to_string(),
            None,
            Vec::new(),
            false,
            100,
            100,
            false,
            false,
        )
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    }

    // users
    User::new("user1", None, "", "", "u1@mail.com", None)
        .save(&pool)
        .await
        .unwrap();
    User::new("user2", None, "", "", "u2@mail.com", None)
        .save(&pool)
        .await
        .unwrap();

    // grups
    Group::new("group1").save(&pool).await.unwrap();
    Group::new("group2").save(&pool).await.unwrap();

    // devices
    Device::new(
        "device1".to_string(),
        String::new(),
        1,
        DeviceType::Network,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();
    Device::new(
        "device2".to_string(),
        String::new(),
        1,
        DeviceType::Network,
        None,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    // aliases
    AclAlias::new("alias1", Vec::new(), Vec::new(), Vec::new())
        .save(&pool)
        .await
        .unwrap();
    AclAlias::new("alias2", Vec::new(), Vec::new(), Vec::new())
        .save(&pool)
        .await
        .unwrap();

    // create an acl rule with related objects
    let mut rule = make_rule();
    rule.networks = vec![1, 2];
    rule.allowed_users = vec![1, 2];
    rule.allowed_groups = vec![1, 2];
    rule.allowed_devices = vec![1, 2];
    rule.aliases = vec![1, 2];

    // create
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_rule: ApiAclRule<NoId> = omit_id(response.json().await);
    assert_eq!(response_rule, rule);

    // retrieve
    let response = client.get("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rule: ApiAclRule<NoId> = omit_id(response.json().await);
    assert_eq!(response_rule, rule);
}

#[tokio::test]
async fn test_invalid_related_objects() {
    let (client, _) = make_test_client().await;
    authenticate(&client).await;

    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // networks
    let mut rule = make_rule();
    rule.networks = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // allowed_users
    let mut rule = make_rule();
    rule.allowed_users = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // denied_users
    let mut rule = make_rule();
    rule.denied_users = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // denied_users
    let mut rule = make_rule();
    rule.denied_users = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // allowed_groups
    let mut rule = make_rule();
    rule.allowed_groups = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // denied_groups
    let mut rule = make_rule();
    rule.denied_groups = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // allowed_devices
    let mut rule = make_rule();
    rule.allowed_devices = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // denied_devices
    let mut rule = make_rule();
    rule.denied_devices = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // aliases
    let mut rule = make_rule();
    rule.aliases = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
