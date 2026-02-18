use defguard_common::{
    config::DefGuardConfig,
    db::{
        Id,
        models::{
            Device, DeviceType, User, WireguardNetwork,
            group::Group,
            settings::initialize_current_settings,
            wireguard::{DEFAULT_WIREGUARD_MTU, LocationMfaMode, ServiceLocationMode},
        },
    },
};
use defguard_core::{
    enterprise::{
        db::models::acl::{AclAlias, AclRule, AliasKind, AliasState, RuleState},
        handlers::acl::{
            ApiAclRule, EditAclRule,
            alias::{ApiAclAlias, EditAclAlias},
        },
        license::{get_cached_license, set_cached_license},
    },
    handlers::Auth,
};
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::net::TcpListener;

use super::common::{
    authenticate_admin, client::TestClient, exceed_enterprise_limits, make_base_client,
    make_test_client, setup_pool,
};
use crate::common::{init_config, initialize_users};

async fn make_client_v2(pool: PgPool, config: DefGuardConfig) -> TestClient {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Could not bind ephemeral socket");
    initialize_users(&pool, &config).await;
    initialize_current_settings(&pool)
        .await
        .expect("Could not initialize settings");
    let (client, _) = make_base_client(pool, config, listener).await;
    client
}

fn make_rule() -> EditAclRule {
    EditAclRule {
        name: "rule".to_string(),
        all_locations: false,
        locations: Vec::new(),
        expires: None,
        allow_all_users: false,
        deny_all_users: false,
        allow_all_groups: false,
        deny_all_groups: false,
        allow_all_network_devices: false,
        deny_all_network_devices: false,
        allowed_users: vec![1],
        denied_users: Vec::new(),
        allowed_groups: Vec::new(),
        denied_groups: Vec::new(),
        allowed_network_devices: Vec::new(),
        denied_network_devices: Vec::new(),
        addresses: "10.2.2.2, 10.0.0.1/24, 10.0.10.1-10.0.20.1".to_string(),
        aliases: Vec::new(),
        destinations: Vec::new(),
        enabled: true,
        protocols: vec![6, 17],
        ports: "1, 2, 3, 10-20, 30-40".to_string(),
        any_address: false,
        any_port: false,
        any_protocol: false,
        use_manual_destination_settings: true,
    }
}

async fn set_rule_state(pool: &PgPool, id: Id, state: RuleState, parent_id: Option<Id>) {
    let mut rule = AclRule::find_by_id(pool, id).await.unwrap().unwrap();
    rule.state = state;
    rule.parent_id = parent_id;
    rule.save(pool).await.unwrap();
}

fn make_alias() -> EditAclAlias {
    EditAclAlias {
        name: "alias".to_string(),
        addresses: "10.2.2.2, 10.0.0.1/24, 10.0.10.1-10.0.20.1".to_string(),
        protocols: vec![6, 17],
        ports: "1, 2, 3, 10-20, 30-40".to_string(),
    }
}

fn edit_rule_data_into_api_response(
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
        all_locations: data.all_locations,
        locations: data.locations.clone(),
        expires: data.expires,
        enabled: data.enabled,
        allow_all_users: data.allow_all_users,
        deny_all_users: data.deny_all_users,
        allow_all_groups: data.allow_all_groups,
        deny_all_groups: data.deny_all_groups,
        allow_all_network_devices: data.allow_all_network_devices,
        deny_all_network_devices: data.deny_all_network_devices,
        allowed_users: data.allowed_users.clone(),
        denied_users: data.denied_users.clone(),
        allowed_groups: data.allowed_groups.clone(),
        denied_groups: data.denied_groups.clone(),
        allowed_network_devices: data.allowed_network_devices.clone(),
        denied_network_devices: data.denied_network_devices.clone(),
        addresses: data.addresses.clone(),
        aliases: data.aliases.clone(),
        destinations: data.destinations.clone(),
        ports: data.ports.clone(),
        protocols: data.protocols.clone(),
        any_address: data.any_address,
        any_port: data.any_port,
        any_protocol: data.any_protocol,
        use_manual_destination_settings: data.use_manual_destination_settings,
    }
}

fn edit_alias_data_into_api_response(
    data: EditAclAlias,
    id: Id,
    parent_id: Option<Id>,
    state: AliasState,
    kind: AliasKind,
    rules: Vec<Id>,
) -> ApiAclAlias {
    ApiAclAlias {
        id,
        parent_id,
        state,
        name: data.name,
        kind,
        addresses: data.addresses,
        ports: data.ports,
        protocols: data.protocols,
        rules,
    }
}

#[sqlx::test]
async fn test_rule_crud(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let rule = make_rule();

    // create
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_rule: ApiAclRule = response.json().await;
    let expected_response =
        edit_rule_data_into_api_response(&rule, response_rule.id, None, RuleState::New);
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

#[sqlx::test]
async fn test_rule_enterprise(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

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
    // GET should be allowed
    let response = client.get("/api/v1/acl/rule").send().await;
    assert_eq!(response.status(), StatusCode::OK);
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

#[sqlx::test]
async fn test_alias_crud(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let alias = make_alias();

    // create
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_alias: ApiAclAlias = response.json().await;
    let expected_response = edit_alias_data_into_api_response(
        alias,
        response_alias.id,
        None,
        AliasState::Applied,
        AliasKind::Component,
        Vec::new(),
    );
    assert_eq!(response_alias, expected_response);

    // list
    let response = client.get("/api/v1/acl/alias").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_aliases: Vec<ApiAclAlias> = response.json().await;
    assert_eq!(response_aliases.len(), 1);
    let response_alias = response_aliases[0].clone();
    assert_eq!(response_alias, expected_response);

    // retrieve
    let response = client.get("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_alias: ApiAclAlias = response.json().await;
    assert_eq!(response_alias, expected_response);

    // update
    let mut alias: ApiAclAlias = client.get("/api/v1/acl/alias/1").send().await.json().await;
    alias.name = "modified".to_string();
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_alias: ApiAclAlias = response.json().await;
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/2").send().await.json().await;
    assert_eq!(response_alias, alias);

    // delete
    let response = client.delete("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/acl/alias").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_aliases: Vec<ApiAclAlias> = response.json().await;
    assert_eq!(response_aliases.len(), 0);
}

#[sqlx::test]
async fn test_alias_enterprise(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

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
    // GET should be allowed
    let response = client.get("/api/v1/acl/alias").send().await;
    assert_eq!(response.status(), StatusCode::OK);
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
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/1").send().await.json().await;
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.delete("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_empty_strings(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // rule
    let mut rule = make_rule();
    rule.addresses = String::new();
    rule.ports = String::new();

    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_rule: ApiAclRule = response.json().await;
    let expected_response = edit_rule_data_into_api_response(
        &rule,
        response_rule.id,
        response_rule.parent_id,
        response_rule.state.clone(),
    );
    assert_eq!(response_rule, expected_response);

    // alias
    let mut alias = make_alias();
    alias.addresses = String::new();
    alias.ports = String::new();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_alias: ApiAclAlias = response.json().await;
    let expected_response = edit_alias_data_into_api_response(
        alias,
        response_alias.id,
        None,
        AliasState::Applied,
        AliasKind::Component,
        Vec::new(),
    );
    assert_eq!(response_alias, expected_response);
}

#[sqlx::test]
async fn test_nonadmin(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _) = make_test_client(pool).await;

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
async fn test_related_objects(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    // create related objects
    // networks
    for net in ["net 1", "net 2"] {
        WireguardNetwork::new(
            net.to_string(),
            Vec::new(),
            1000,
            "endpoint1".to_string(),
            None,
            DEFAULT_WIREGUARD_MTU,
            0,
            Vec::new(),
            100,
            100,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
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
    AclAlias::new(
        "alias1",
        AliasState::Applied,
        AliasKind::Component,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        true,
        true,
        true,
    )
    .save(&pool)
    .await
    .unwrap();
    AclAlias::new(
        "alias2",
        AliasState::Applied,
        AliasKind::Component,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        true,
        true,
        true,
    )
    .save(&pool)
    .await
    .unwrap();

    // create an acl rule with related objects
    let mut rule = make_rule();
    rule.locations = vec![1, 2];
    rule.allowed_users = vec![1, 2];
    rule.allowed_groups = vec![1, 2];
    rule.allowed_network_devices = vec![1, 2];
    rule.aliases = vec![1, 2];

    // create
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_rule = response.json::<EditAclRule>().await;
    assert_eq!(response_rule, rule);

    // retrieve
    let response = client.get("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_rule = response.json::<EditAclRule>().await;
    assert_eq!(response_rule, rule);

    // related rules in alias details
    let response = client.get("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_alias = response.json::<ApiAclAlias>().await;
    assert_eq!(response_alias.rules, [1]);

    // add another rule
    let mut rule = make_rule();
    rule.aliases = vec![1];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client.get("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_alias = response.json::<ApiAclAlias>().await;
    assert_eq!(response_alias.rules, [1, 2]);
    let response = client.get("/api/v1/acl/alias/2").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_alias = response.json::<ApiAclAlias>().await;
    assert_eq!(response_alias.rules, [1]);
}

#[sqlx::test]
async fn test_invalid_related_objects(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // networks
    let mut rule = make_rule();
    rule.locations = vec![100];
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
    rule.allowed_network_devices = vec![100];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // denied_devices
    let mut rule = make_rule();
    rule.denied_network_devices = vec![100];
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

    // alias with invalid state
    AclAlias::new(
        "alias1",
        AliasState::Modified,
        AliasKind::Destination,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        true,
        true,
        true,
    )
    .save(&state.pool)
    .await
    .unwrap();
    let mut rule = make_rule();
    rule.aliases = vec![1];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_invalid_data(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // invalid port
    let mut rule = make_rule();
    rule.ports = "65536".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.ports = "-1".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.ports = "65535".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // invalid ip range
    let mut rule = make_rule();
    rule.addresses = "10.10.10.20-10.10.10.10".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.10.10.10-10.10.10.20".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn test_rule_create_modify_state(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

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
async fn test_rule_delete_state_new(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

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
async fn test_rule_delete_state_applied(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    // create a location
    WireguardNetwork::new(
        "test location".to_string(),
        Vec::new(),
        1000,
        "endpoint1".to_string(),
        None,
        DEFAULT_WIREGUARD_MTU,
        0,
        Vec::new(),
        100,
        100,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .save(&pool)
    .await
    .unwrap();

    // test APPLIED rule deletion
    let mut rule = make_rule();
    rule.locations = vec![1];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 1);
    set_rule_state(&pool, 1, RuleState::Applied, None).await;

    let rule_before_mods: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    let response = client.delete("/api/v1/acl/rule/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);
    let rule_parent: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    let rule_child: ApiAclRule = client.get("/api/v1/acl/rule/2").send().await.json().await;
    assert_eq!(rule_parent, rule_before_mods);
    let mut rule_after_mods = rule_before_mods.clone();
    rule_after_mods.id = 2;
    rule_after_mods.state = RuleState::Deleted;
    rule_after_mods.parent_id = Some(1);

    assert_eq!(rule_after_mods, rule_child);

    // related networks are returned correctly
    assert_eq!(rule_child.locations, vec![1]);

    // cannot modify a DELETED rule
    let response = client
        .put("/api/v1/acl/rule/2")
        .json(&rule_after_mods)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn test_rule_duplication(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // each modification / deletion of parent rule should remove the child and create a new one
    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

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
async fn test_rule_application(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

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
        .json(&json!({ "rules": [1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify rule was applied
    let mut rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(rule.state, RuleState::Applied);

    // cannot apply the same rule again
    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": [1] }))
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
        .json(&json!({ "rules": [1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // apply modification
    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": [2] }))
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

    // delete rule
    let response = client.delete("/api/v1/acl/rule/2").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify rule was marked for deletion
    let rule: ApiAclRule = client.get("/api/v1/acl/rule/3").send().await.json().await;
    assert_eq!(rule.state, RuleState::Deleted);
    assert_eq!(rule.parent_id, Some(2));
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);

    // apply modification
    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": [3] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify rules were removed
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 0);
}

#[sqlx::test]
async fn test_multiple_rules_application(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

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
        .json(&json!({ "rules": [1, 3] }))
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

#[sqlx::test]
async fn test_alias_create_modify_state(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    let alias = make_alias();

    // assert created alias has correct state
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let dbalias = AclAlias::find_by_id(&pool, 1).await.unwrap().unwrap();
    assert_eq!(dbalias.state, AliasState::Applied);
    assert_eq!(dbalias.parent_id, None);

    // test APPLIED alias modification
    let alias_before_mods: ApiAclAlias =
        client.get("/api/v1/acl/alias/1").send().await.json().await;
    let mut alias_modified = alias_before_mods.clone();
    let response = client
        .put("/api/v1/acl/alias/1")
        .json(&alias_modified)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 2);
    let alias_parent: ApiAclAlias = client.get("/api/v1/acl/alias/1").send().await.json().await;
    let alias_child: ApiAclAlias = client.get("/api/v1/acl/alias/2").send().await.json().await;
    assert_eq!(alias_parent, alias_before_mods);
    assert_eq!(alias_parent.state, AliasState::Applied);
    alias_modified.id = 2;
    alias_modified.state = AliasState::Modified;
    alias_modified.parent_id = Some(1);
    assert_eq!(alias_child, alias_modified);
    assert_eq!(alias_child.state, AliasState::Modified);
    assert_eq!(alias_child.parent_id, Some(alias_parent.id));
}

#[sqlx::test]
async fn test_alias_delete(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    // create alias
    let alias = make_alias();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 1);

    // use alias in a new rule
    let mut rule = make_rule();
    rule.aliases = vec![1];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // cannot delete alias if used by a rule
    let response = client.delete("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 1);

    // remove alias from rule
    rule.aliases = Vec::new();
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 1);

    // delete alias
    let response = client.delete("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 0);

    // create another alias
    let mut alias = make_alias();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 1);

    // modify alias
    alias.name = "modified".to_string();
    let response = client.put("/api/v1/acl/alias/2").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 2);

    // delete pending modification
    let response = client.delete("/api/v1/acl/alias/3").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 1);

    // modify alias again
    alias.name = "modified again".to_string();
    let response = client.put("/api/v1/acl/alias/2").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 2);

    // delete original alias
    let response = client.delete("/api/v1/acl/alias/2").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 0);
}

#[sqlx::test]
async fn test_alias_duplication(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // each modification of parent alias should remove the child and create a new one
    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    let alias = make_alias();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // ensure we don't duplicate already modified / deleted aliass
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 1);
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/1").send().await.json().await;
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 2);
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 2);
    let response = client.delete("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 0);
}

#[sqlx::test]
async fn test_alias_application(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    // create new alias
    let alias = make_alias();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify initial status
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/1").send().await.json().await;
    assert_eq!(alias.state, AliasState::Applied);

    // use alias in a new rule
    let mut rule = make_rule();
    rule.aliases = vec![1];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify rule assignment
    let mut alias: ApiAclAlias = client.get("/api/v1/acl/alias/1").send().await.json().await;
    assert_eq!(alias.rules, vec![1]);

    // modify alias
    alias.name = "modified".to_string();
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 2);

    // cannot apply already applied alias
    let response = client
        .put("/api/v1/acl/alias/apply")
        .json(&json!({ "aliases": [1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // apply modification
    let response = client
        .put("/api/v1/acl/alias/apply")
        .json(&json!({ "aliases": [2] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify alias was applied
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/2").send().await.json().await;
    assert_eq!(alias.state, AliasState::Applied);
    assert_eq!(alias.parent_id, None);
    assert_eq!(alias.rules, vec![1]);

    // initial alias was deleted
    let response = client.get("/api/v1/acl/alias/1").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 1);
}

#[sqlx::test]
async fn test_multiple_aliases_application(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    let alias_1 = make_alias();
    let alias_2 = make_alias();
    let alias_3 = make_alias();

    // create new aliass
    let response = client.post("/api/v1/acl/alias").json(&alias_1).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.post("/api/v1/acl/alias").json(&alias_2).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.post("/api/v1/acl/alias").json(&alias_3).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // modify aliases
    let mut alias_1: ApiAclAlias = client.get("/api/v1/acl/alias/1").send().await.json().await;
    alias_1.name = "modified 1".to_string();
    let response = client
        .put("/api/v1/acl/alias/1")
        .json(&alias_1)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut alias_2: ApiAclAlias = client.get("/api/v1/acl/alias/2").send().await.json().await;
    alias_2.name = "modified 2".to_string();
    let response = client
        .put("/api/v1/acl/alias/2")
        .json(&alias_2)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut alias_3: ApiAclAlias = client.get("/api/v1/acl/alias/3").send().await.json().await;
    alias_3.name = "modified 3".to_string();
    let response = client
        .put("/api/v1/acl/alias/3")
        .json(&alias_3)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 6);

    // apply multiple aliases
    let response = client
        .put("/api/v1/acl/alias/apply")
        .json(&json!({ "aliases": [4, 6] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclAlias::all(&pool).await.unwrap().len(), 4);

    // verify alias state
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/2").send().await.json().await;
    assert_eq!(alias.state, AliasState::Applied);
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/4").send().await.json().await;
    assert_eq!(alias.state, AliasState::Applied);
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/5").send().await.json().await;
    assert_eq!(alias.state, AliasState::Modified);
    let alias: ApiAclAlias = client.get("/api/v1/acl/alias/6").send().await.json().await;
    assert_eq!(alias.state, AliasState::Applied);
}
