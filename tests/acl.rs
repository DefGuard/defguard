use common::{init_config, init_test_db, omit_id};
use defguard::{
    db::{
        models::{oauth2client::OAuth2Client, NewOpenIDClient},
        AddDevice, Id, NoId, UserInfo, WireguardNetwork,
    },
    enterprise::{db::models::acl::PortRange, handlers::acl::ApiAclRule},
    handlers::{AddUserData, Auth, PasswordChange, PasswordChangeSelf, Username},
};
use reqwest::{header::USER_AGENT, StatusCode};
use tokio_stream::{self as stream, StreamExt};

use self::common::{client::TestClient, fetch_user_details, make_network, make_test_client};

async fn make_client() -> TestClient {
    let (client, _) = make_test_client().await;
    client
}

pub mod common;

#[tokio::test]
async fn test_rule_crud() {
    let client = make_client().await;
    let config = init_config(None);
    let pool = init_test_db(&config).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let rule = ApiAclRule {
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
    };

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
    // // prepare related objects
    // let network1 = WireguardNetwork::new(
    //     "net1".to_string(),
    //     vec!["192.168.1.10".parse().unwrap()],
    //     5555,
    //     "test.com".to_string(),
    //     None,
    //     Vec::new(),
    //     false,
    //     100,
    //     200,
    //     false,
    //     false,
    // )
    // .unwrap()
    // .save(&pool)
    // .await
    // .unwrap();
    // let network2 = WireguardNetwork::new(
    //     "net2".to_string(),
    //     vec!["192.168.2.10".parse().unwrap()],
    //     5555,
    //     "test.com".to_string(),
    //     None,
    //     Vec::new(),
    //     false,
    //     100,
    //     200,
    //     false,
    //     false,
    // )
    // .unwrap()
    // .save(&pool)
    // .await
    // .unwrap();
}
