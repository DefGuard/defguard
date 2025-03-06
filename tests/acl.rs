use common::{exceed_enterprise_limits, omit_id};
use defguard::{
    db::{Id, NoId},
    enterprise::{
        handlers::acl::ApiAclRule,
        license::{get_cached_license, set_cached_license},
    },
    handlers::Auth,

};
use reqwest::StatusCode;

use self::common::make_test_client;

pub mod common;

#[allow(dead_code)]
fn make_rule() -> ApiAclRule {
    ApiAclRule {
        id: NoId,
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
        destination: "10.0.0.1/24".to_string(),
        aliases: vec![],
        enabled: true,
        protocols: vec![],
        ports: "10-20, 30-40".to_string(),
    }
}

#[tokio::test]
async fn test_rule_crud() {
    let (client, _) = make_test_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

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

#[tokio::test]
async fn test_rule_enterprise() {
    let (client, _) = make_test_client().await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    exceed_enterprise_limits(&client).await;

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // restore valid license and try again
    set_cached_license(license);
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rules: Vec<serde_json::Value> = response.json().await;
    assert_eq!(response_rules.len(), 1);
}
