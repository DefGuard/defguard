use super::*;

fn assert_related_object_rules_rewritten(
    related_object_name: &str,
    rules: &[Id],
    old_rule_id: Id,
    new_rule_id: Id,
) {
    assert!(
        !rules.contains(&old_rule_id),
        "{related_object_name} should no longer reference the old applied rule {old_rule_id}; got {rules:?}",
    );
    assert!(
        rules.contains(&new_rule_id),
        "{related_object_name} should reference the new applied rule {new_rule_id}; got {rules:?}",
    );
    assert_eq!(
        rules,
        [new_rule_id],
        "{related_object_name} should reference only the new applied rule",
    );
}

fn assert_pending_related_object_rules_not_exposed(
    related_object_name: &str,
    rules: &[Id],
    old_rule_id: Id,
    new_rule_id: Id,
) {
    assert!(
        !rules.contains(&old_rule_id),
        "{related_object_name} should not expose the stale applied rule {old_rule_id}; got {rules:?}",
    );
    assert!(
        !rules.contains(&new_rule_id),
        "{related_object_name} should not expose the rewritten applied rule {new_rule_id} before the pending row itself is applied; got {rules:?}",
    );
    assert!(
        rules.is_empty(),
        "{related_object_name} currently exposes an empty rules array for pending rows; got {rules:?}",
    );
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
async fn test_rule_requires_destination(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // manual destination enabled but empty
    let mut rule = make_rule();
    rule.use_manual_destination_settings = true;
    rule.addresses = String::new();
    rule.ports = String::new();
    rule.protocols = Vec::new();
    rule.any_address = false;
    rule.any_port = false;
    rule.any_protocol = false;
    rule.destinations = Vec::new();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // manual destination disabled and no destination aliases
    let mut rule = make_rule();
    rule.use_manual_destination_settings = false;
    rule.addresses = String::new();
    rule.ports = String::new();
    rule.protocols = Vec::new();
    rule.any_address = false;
    rule.any_port = false;
    rule.any_protocol = false;
    rule.destinations = Vec::new();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // manual destination configured
    let mut rule = make_rule();
    rule.use_manual_destination_settings = true;
    rule.addresses = "10.0.0.1".to_string();
    rule.ports = "80".to_string();
    rule.protocols = vec![6];
    rule.any_address = false;
    rule.any_port = false;
    rule.any_protocol = false;
    rule.destinations = Vec::new();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_rule: ApiAclRule = response.json().await;

    // destination alias configured
    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let destination: Value = response.json().await;
    let destination_id = destination["id"].as_i64().unwrap();

    let mut rule = make_rule();
    rule.use_manual_destination_settings = false;
    rule.addresses = String::new();
    rule.ports = String::new();
    rule.protocols = Vec::new();
    rule.any_address = false;
    rule.any_port = false;
    rule.any_protocol = false;
    rule.destinations = vec![destination_id];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // update to invalid manual destination
    let mut invalid_update = created_rule.clone();
    invalid_update.use_manual_destination_settings = true;
    invalid_update.addresses = String::new();
    invalid_update.ports = String::new();
    invalid_update.protocols = Vec::new();
    invalid_update.any_address = false;
    invalid_update.any_port = false;
    invalid_update.any_protocol = false;
    invalid_update.destinations = Vec::new();
    let response = client
        .put(format!("/api/v1/acl/rule/{}", created_rule.id))
        .json(&invalid_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // update to invalid alias-only destination
    let mut invalid_update = created_rule.clone();
    invalid_update.use_manual_destination_settings = false;
    invalid_update.addresses = String::new();
    invalid_update.ports = String::new();
    invalid_update.protocols = Vec::new();
    invalid_update.any_address = false;
    invalid_update.any_port = false;
    invalid_update.any_protocol = false;
    invalid_update.destinations = Vec::new();
    let response = client
        .put(format!("/api/v1/acl/rule/{}", created_rule.id))
        .json(&invalid_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
    let mut created_rule: ApiAclRule = response.json().await;

    let mut rule = make_rule();
    rule.ports = "65534-65535".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut rule = make_rule();
    rule.ports = "65535-65536".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    created_rule.ports = "200-100".into();
    let response = client
        .put(format!("/api/v1/acl/rule/{}", created_rule.id))
        .json(&created_rule)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    created_rule.ports = "1-2-3".into();
    let response = client
        .put(format!("/api/v1/acl/rule/{}", created_rule.id))
        .json(&created_rule)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut rule = make_rule();
    rule.ports = "200-100".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.ports = "1-2-3".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // invalid ip range
    let mut rule = make_rule();
    rule.addresses = "10.10.10.20-10.10.10.10".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.10.10.10-10.10.10.10".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.10.10.10-2001:db8::1".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.10.10.10-10.10.10.20-10.10.10.30".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.10.10.0/24-10.10.10.20".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.0.0.1/24/25".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "2001:db8::1/64/65".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.0.0.1/1e1".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.0.0.1/0x18".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "2001:db8::1/64foo".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    rule.addresses = "10.10.10.10-10.10.10.20".into();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    rule.addresses = "2001:db8::1-2001:db8::2".into();
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
async fn test_rule_modify_pending_child_updates_in_place(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": [1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let applied_parent_before_update: ApiAclRule =
        client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(applied_parent_before_update.state, RuleState::Applied);

    let mut first_update = applied_parent_before_update.clone();
    first_update.name = "rule pending child".to_string();
    let response = client
        .put("/api/v1/acl/rule/1")
        .json(&first_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(AclRule::all(&pool).await.unwrap().len(), 2);

    let pending_child_before_update: ApiAclRule =
        client.get("/api/v1/acl/rule/2").send().await.json().await;
    assert_eq!(pending_child_before_update.state, RuleState::Modified);
    assert_eq!(pending_child_before_update.parent_id, Some(1));

    let mut pending_child_update = pending_child_before_update.clone();
    pending_child_update.name = "rule pending child updated".to_string();
    let response = client
        .put("/api/v1/acl/rule/2")
        .json(&pending_child_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_pending_child: ApiAclRule = response.json().await;

    let rules = AclRule::all(&pool).await.unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(
        rules
            .iter()
            .filter(|rule| rule.state == RuleState::Applied)
            .count(),
        1
    );
    assert_eq!(
        rules
            .iter()
            .filter(|rule| rule.state == RuleState::Modified)
            .count(),
        1
    );

    let response = client.get("/api/v1/acl/rule/3").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let applied_parent_after_update: ApiAclRule =
        client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(applied_parent_after_update, applied_parent_before_update);
    assert_eq!(applied_parent_after_update.state, RuleState::Applied);
    assert_eq!(applied_parent_after_update.parent_id, None);

    let mut expected_pending_child = pending_child_before_update.clone();
    expected_pending_child.name = "rule pending child updated".to_string();
    assert_eq!(updated_pending_child, expected_pending_child);

    let pending_child_after_update: ApiAclRule =
        client.get("/api/v1/acl/rule/2").send().await.json().await;
    assert_eq!(pending_child_after_update, expected_pending_child);
    assert_eq!(
        pending_child_after_update.id,
        pending_child_before_update.id
    );
    assert_eq!(pending_child_after_update.state, RuleState::Modified);
    assert_eq!(pending_child_after_update.parent_id, Some(1));
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
async fn test_rule_apply_rewrites_related_alias_and_destination_rule_ids(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    let mut alias = make_alias();
    alias.name = "applied alias".to_string();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let applied_alias: ApiAclAlias = response.json().await;

    let mut destination = make_destination();
    destination.name = "applied destination".to_string();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let applied_destination: ApiAclDestination = response.json().await;

    let mut rule = make_rule();
    rule.name = "rule parent".to_string();
    rule.use_manual_destination_settings = false;
    rule.addresses = String::new();
    rule.ports = String::new();
    rule.protocols = Vec::new();
    rule.any_address = false;
    rule.any_port = false;
    rule.any_protocol = false;
    rule.aliases = vec![applied_alias.id];
    rule.destinations = vec![applied_destination.id];

    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_rule: ApiAclRule = response.json().await;

    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": [created_rule.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let applied_parent_rule: ApiAclRule = client
        .get(format!("/api/v1/acl/rule/{}", created_rule.id))
        .send()
        .await
        .json()
        .await;
    assert_eq!(applied_parent_rule.state, RuleState::Applied);
    assert_eq!(applied_parent_rule.parent_id, None);
    assert_eq!(applied_parent_rule.aliases, vec![applied_alias.id]);
    assert_eq!(
        applied_parent_rule.destinations,
        vec![applied_destination.id]
    );

    let deployed_alias_before_rule_reapply: ApiAclAlias = client
        .get(format!("/api/v1/acl/alias/{}", applied_alias.id))
        .send()
        .await
        .json()
        .await;
    assert_eq!(
        deployed_alias_before_rule_reapply.rules,
        vec![applied_parent_rule.id],
        "deployed alias should initially reference the applied parent rule",
    );

    let deployed_destination_before_rule_reapply: ApiAclDestination = client
        .get(format!(
            "/api/v1/acl/destination/{}",
            applied_destination.id
        ))
        .send()
        .await
        .json()
        .await;
    assert_eq!(
        deployed_destination_before_rule_reapply.rules,
        vec![applied_parent_rule.id],
        "deployed destination should initially reference the applied parent rule",
    );

    let mut alias_update = deployed_alias_before_rule_reapply.clone();
    alias_update.name = "pending alias child".to_string();
    let response = client
        .put(format!("/api/v1/acl/alias/{}", applied_alias.id))
        .json(&alias_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let pending_alias_child: ApiAclAlias = response.json().await;
    assert_eq!(pending_alias_child.state, AliasState::Modified);
    assert_eq!(pending_alias_child.parent_id, Some(applied_alias.id));

    let mut destination_update = deployed_destination_before_rule_reapply.clone();
    destination_update.name = "pending destination child".to_string();
    let response = client
        .put(format!(
            "/api/v1/acl/destination/{}",
            applied_destination.id
        ))
        .json(&destination_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let pending_destination_child: ApiAclDestination = response.json().await;
    assert_eq!(pending_destination_child.state, AliasState::Modified);
    assert_eq!(
        pending_destination_child.parent_id,
        Some(applied_destination.id)
    );

    let mut modified_rule = applied_parent_rule.clone();
    modified_rule.name = "rule child".to_string();
    let response = client
        .put(format!("/api/v1/acl/rule/{}", applied_parent_rule.id))
        .json(&modified_rule)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let pending_rule_child: ApiAclRule = response.json().await;
    assert_eq!(pending_rule_child.state, RuleState::Modified);
    assert_eq!(pending_rule_child.parent_id, Some(applied_parent_rule.id));
    assert_eq!(pending_rule_child.aliases, vec![applied_alias.id]);
    assert_eq!(
        pending_rule_child.destinations,
        vec![applied_destination.id]
    );

    let response = client
        .put("/api/v1/acl/rule/apply")
        .json(&json!({ "rules": [pending_rule_child.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(format!("/api/v1/acl/rule/{}", applied_parent_rule.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let applied_rule_child: ApiAclRule = client
        .get(format!("/api/v1/acl/rule/{}", pending_rule_child.id))
        .send()
        .await
        .json()
        .await;
    assert_eq!(applied_rule_child.state, RuleState::Applied);
    assert_eq!(applied_rule_child.parent_id, None);
    assert_eq!(applied_rule_child.aliases, vec![applied_alias.id]);
    assert_eq!(
        applied_rule_child.destinations,
        vec![applied_destination.id]
    );

    let deployed_alias_after_rule_apply: ApiAclAlias = client
        .get(format!("/api/v1/acl/alias/{}", applied_alias.id))
        .send()
        .await
        .json()
        .await;
    assert_related_object_rules_rewritten(
        "deployed alias details",
        &deployed_alias_after_rule_apply.rules,
        applied_parent_rule.id,
        applied_rule_child.id,
    );

    let deployed_destination_after_rule_apply: ApiAclDestination = client
        .get(format!(
            "/api/v1/acl/destination/{}",
            applied_destination.id
        ))
        .send()
        .await
        .json()
        .await;
    assert_related_object_rules_rewritten(
        "deployed destination details",
        &deployed_destination_after_rule_apply.rules,
        applied_parent_rule.id,
        applied_rule_child.id,
    );

    let pending_alias_after_rule_apply: ApiAclAlias = client
        .get(format!("/api/v1/acl/alias/{}", pending_alias_child.id))
        .send()
        .await
        .json()
        .await;
    assert_eq!(pending_alias_after_rule_apply.state, AliasState::Modified);
    assert_eq!(
        pending_alias_after_rule_apply.parent_id,
        Some(applied_alias.id)
    );
    // Pending alias rows expose a `rules` field, but the backend keeps rule associations on the
    // deployed alias row until the alias child itself is applied.
    assert_pending_related_object_rules_not_exposed(
        "pending alias details",
        &pending_alias_after_rule_apply.rules,
        applied_parent_rule.id,
        applied_rule_child.id,
    );

    let pending_destination_after_rule_apply: ApiAclDestination = client
        .get(format!(
            "/api/v1/acl/destination/{}",
            pending_destination_child.id
        ))
        .send()
        .await
        .json()
        .await;
    assert_eq!(
        pending_destination_after_rule_apply.state,
        AliasState::Modified
    );
    assert_eq!(
        pending_destination_after_rule_apply.parent_id,
        Some(applied_destination.id)
    );
    // Pending destination rows expose a `rules` field, but the backend keeps rule associations on
    // the deployed destination row until the destination child itself is applied.
    assert_pending_related_object_rules_not_exposed(
        "pending destination details",
        &pending_destination_after_rule_apply.rules,
        applied_parent_rule.id,
        applied_rule_child.id,
    );
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
async fn test_acl_count_endpoints(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool.clone()).await;
    authenticate_admin(&mut client).await;

    // rules: 1 applied, 1 pending (new)
    let rule = make_rule();
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    set_rule_state(&pool, 1, RuleState::Applied, None).await;

    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // aliases: 2 applied, 1 pending (modified)
    let alias = make_alias();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let mut alias_to_update: ApiAclAlias =
        client.get("/api/v1/acl/alias/2").send().await.json().await;
    alias_to_update.name = "updated alias".to_string();
    let response = client
        .put("/api/v1/acl/alias/2")
        .json(&alias_to_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // destinations: 2 applied, 1 pending (modified)
    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let destination_1: Value = response.json().await;
    let destination_1_id = destination_1["id"].as_i64().unwrap();

    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut destination_to_update = destination.clone();
    destination_to_update.name = "updated destination".to_string();
    let response = client
        .put(format!("/api/v1/acl/destination/{destination_1_id}"))
        .json(&destination_to_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let counts: Value = client
        .get("/api/v1/acl/rule/count")
        .send()
        .await
        .json()
        .await;
    assert_eq!(counts["applied"], json!(1));
    assert_eq!(counts["pending"], json!(1));

    let counts: Value = client
        .get("/api/v1/acl/alias/count")
        .send()
        .await
        .json()
        .await;
    assert_eq!(counts["applied"], json!(2));
    assert_eq!(counts["pending"], json!(1));

    let counts: Value = client
        .get("/api/v1/acl/destination/count")
        .send()
        .await
        .json()
        .await;
    assert_eq!(counts["applied"], json!(2));
    assert_eq!(counts["pending"], json!(1));
}
