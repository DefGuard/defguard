use common::{
    client::TestClient, exceed_enterprise_limits, init_config, initialize_users, omit_id,
};
use defguard::{
    config::DefGuardConfig,
    db::{models::device::DeviceType, Device, Group, Id, NoId, User, WireguardNetwork},
    enterprise::{
        db::models::acl::{AclAlias, AclRule, RuleState},
        handlers::acl::{ApiAclAlias, ApiAclRule, EditAclRule},
        license::{get_cached_license, set_cached_license},
    },
    handlers::Auth,
};
use reqwest::StatusCode;
use serde_json::{from_value, json, Value};
use serial_test::serial;
use sqlx::PgPool;
use tokio::net::TcpListener;

use self::common::{make_base_client, make_test_client};

pub mod common;

async fn make_client_v2(pool: PgPool, config: DefGuardConfig) -> TestClient {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Could not bind ephemeral socket");
    initialize_users(&pool, &config).await;
    let (client, _) = make_base_client(pool, config, listener).await;
    client
}

async fn authenticate(client: &TestClient) {
    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

fn make_rule() -> EditAclRule {
    EditAclRule {
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

async fn set_rule_state(pool: &PgPool, id: Id, state: RuleState, parent_id: Option<Id>) {
    let mut rule = AclRule::find_by_id(pool, id).await.unwrap().unwrap();
    rule.state = state;
    rule.parent_id = parent_id;
    rule.save(pool).await.unwrap();
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

fn edit_data_into_api_response(
    data: &EditAclRule,
    id: Id,
    parent_id: Option<Id>,
    state: RuleState,
) -> ApiAclRule {
    ApiAclRule {
        id,
        parent_id,
        state,
        name: data.name.clone(),
        all_networks: data.all_networks,
        networks: data.networks.clone(),
        expires: data.expires,
        enabled: data.enabled,
        allow_all_users: data.allow_all_users,
        deny_all_users: data.deny_all_users,
        allowed_users: data.allowed_users.clone(),
        denied_users: data.denied_users.clone(),
        allowed_groups: data.allowed_groups.clone(),
        denied_groups: data.denied_groups.clone(),
        allowed_devices: data.allowed_devices.clone(),
        denied_devices: data.denied_devices.clone(),
        destination: data.destination.clone(),
        aliases: data.aliases.clone(),
        ports: data.ports.clone(),
        protocols: data.protocols.clone(),
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
    let response_rule: ApiAclRule = response.json().await;
    let expected_response = edit_data_into_api_response(
        &rule,
        response_rule.id,
        response_rule.parent_id,
        response_rule.state.clone(),
    );
    assert_eq!(response_rule, expected_response);

    // list
    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rules: Vec<ApiAclRule> = response.json().await;
    assert_eq!(response_rules.len(), 1);
    let response_rule = response_rules[0].clone();
    assert_eq!(response_rule, expected_response);

    // retrieve
    let response = client.get("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rule: ApiAclRule = response.json().await;
    assert_eq!(response_rule, expected_response);

    // update
    let mut rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    rule.name = "modified".to_string();
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rule: ApiAclRule = response.json().await;
    assert_eq!(response_rule, rule);
    let response_rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(response_rule, rule);

    // delete
    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rules: Vec<Value> = response.json().await;
    assert_eq!(response_rules.len(), 0);
}

#[tokio::test]
#[serial]
async fn test_rule_enterprise() {
    let (client, _) = make_test_client().await;
    authenticate(&client).await;

    exceed_enterprise_limits(&client).await;

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    // try to use ACL api
    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // restore valid license and try again
    set_cached_license(license);
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rules: Vec<Value> = response.json().await;
    assert_eq!(response_rules.len(), 1);
    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

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
    let response_aliases: Vec<Value> = response.json().await;
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
    let response_aliases: Vec<Value> = response.json().await;
    assert_eq!(response_aliases.len(), 0);
}

#[tokio::test]
#[serial]
async fn test_alias_enterprise() {
    let (client, _) = make_test_client().await;
    authenticate(&client).await;

    exceed_enterprise_limits(&client).await;

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    // try to use ACL api
    let alias = make_alias();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.get("/api/v1/acl/alias").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client.delete("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // restore valid license and try again
    set_cached_license(license);
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.get("/api/v1/acl/alias").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_aliases: Vec<Value> = response.json().await;
    assert_eq!(response_aliases.len(), 1);
    let response = client.get("/api/v1/acl/alias").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let alias: ApiAclAlias<Id> = client.get("/api/v1/acl/alias/1").send().await.json().await;
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.delete("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

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
    let response_rule: ApiAclRule = response.json().await;
    let expected_response = edit_data_into_api_response(
        &rule,
        response_rule.id,
        response_rule.parent_id,
        response_rule.state.clone(),
    );
    assert_eq!(response_rule, expected_response);

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

#[sqlx::test]
async fn test_related_objects(pool: PgPool) {
    let config = init_config(None);
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
    let response_rule: EditAclRule = response.json().await;
    assert_eq!(response_rule, rule);

    // retrieve
    let response = client.get("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rule: EditAclRule = omit_id(response.json().await);
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

#[sqlx::test]
async fn test_rule_create_modify_state(pool: PgPool) {
    let config = init_config(None);
    let client = make_client_v2(pool.clone(), config).await;
    authenticate(&client).await;

    let rule = make_rule();

    // assert created rule has correct state
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let dbrule = AclRule::find_by_id(&pool, 1).await.unwrap().unwrap();
    assert_eq!(dbrule.state, RuleState::New);
    assert_eq!(dbrule.parent_id, None);

    // test NEW rule modification
    let mut rule_modified: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(rule_modified.state, RuleState::New);
    rule_modified.enabled = !rule.enabled;
    let response = client
        .put("/api/v1/acl/rule/1")
        .json(&rule_modified)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let rule_from_api: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 1);
    assert_eq!(rule_from_api, rule_modified);

    // test APPLIED rule modification
    set_rule_state(&pool, 1, RuleState::Applied, None).await;
    let rule_before_mods: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    let mut rule_modified = rule_before_mods.clone();
    rule_modified.enabled = !rule_modified.enabled;
    let response = client
        .put("/api/v1/acl/rule/1")
        .json(&rule_modified)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);
    let rule_parent: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    let rule_child: ApiAclRule = client.get("/api/v1/acl/rule/2").send().await.json().await;
    assert_eq!(rule_parent, rule_before_mods);
    assert_eq!(rule_parent.state, RuleState::Applied);
    rule_modified.id = 2;
    rule_modified.state = RuleState::Modified;
    rule_modified.parent_id = Some(1);
    assert_eq!(rule_child, rule_modified);
    assert_eq!(rule_child.state, RuleState::Modified);
    assert_eq!(rule_child.parent_id, Some(rule_parent.id));
}

#[sqlx::test]
async fn test_rule_delete_state_new(pool: PgPool) {
    let config = init_config(None);
    let client = make_client_v2(pool.clone(), config).await;
    authenticate(&client).await;

    // test NEW rule deletion
    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 1);

    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 0);
}

#[sqlx::test]
async fn test_rule_delete_state_applied(pool: PgPool) {
    let config = init_config(None);
    let client = make_client_v2(pool.clone(), config).await;
    authenticate(&client).await;

    // test APPLIED rule deletion
    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 1);
    set_rule_state(&pool, 1, RuleState::Applied, None).await;

    let rule_before_mods: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);
    let rule_parent: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    let rule_child: Value = client.get("/api/v1/acl/rule/2").send().await.json().await;
    assert_eq!(rule_parent, rule_before_mods);
    let mut rule_after_mods = rule_before_mods.clone();
    rule_after_mods.id = 2;
    rule_after_mods.state = RuleState::Deleted;
    rule_after_mods.parent_id = Some(1);
    // don't care about related objects of deleted rule
    rule_after_mods.destination =
        from_value(rule_child.clone().get("destination").unwrap().clone()).unwrap();
    assert_eq!(json!(rule_after_mods), rule_child);
}

#[sqlx::test]
async fn test_rule_duplication(pool: PgPool) {
    // each modification / deletion of parent rule should remove the child and create a new one
    let config = init_config(None);
    let client = make_client_v2(pool.clone(), config).await;
    authenticate(&client).await;

    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    set_rule_state(&pool, 1, RuleState::Applied, None).await;

    // ensure we don't duplicate already modified / deleted rules
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 1);
    let rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);
    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);
    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);
}

#[sqlx::test]
async fn test_rule_application(pool: PgPool) {
    let config = init_config(None);
    let client = make_client_v2(pool.clone(), config).await;
    authenticate(&client).await;

    let rule = make_rule();

    // create new rule
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify initial status
    let rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(rule.state, RuleState::New);

    // apply rule
    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": vec![1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify rule was applied
    let mut rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(rule.state, RuleState::Applied);

    // cannot apply the same rule again
    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": vec![1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // modify rule
    rule.enabled = !rule.enabled;
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);

    // still cannot apply the same rule again
    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": vec![1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // apply modification
    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": vec![2] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify rule was applied
    let rule: ApiAclRule = client.get("/api/v1/acl/rule/2").send().await.json().await;
    assert_eq!(rule.state, RuleState::Applied);
    assert_eq!(rule.parent_id, None);

    // initial rule was deleted
    let response = client.get("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 1);
}

#[sqlx::test]
async fn test_multiple_rules_application(pool: PgPool) {
    let config = init_config(None);
    let client = make_client_v2(pool.clone(), config).await;
    authenticate(&client).await;

    let rule_1 = make_rule();
    let rule_2 = make_rule();
    let rule_3 = make_rule();

    // create new rules
    let response = client.post("/api/v1/acl/rule").json(&rule_1).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.post("/api/v1/acl/rule").json(&rule_2).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.post("/api/v1/acl/rule").json(&rule_3).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // apply multiple rules
    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": vec![1, 3] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 3);

    // verify rule state
    let rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(rule.state, RuleState::Applied);
    let rule: ApiAclRule = client.get("/api/v1/acl/rule/2").send().await.json().await;
    assert_eq!(rule.state, RuleState::New);
    let rule: ApiAclRule = client.get("/api/v1/acl/rule/3").send().await.json().await;
    assert_eq!(rule.state, RuleState::Applied);
}
