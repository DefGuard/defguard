use super::*;

#[sqlx::test]
async fn test_destination_crud(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let destination = make_destination();

    // create
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response_destination: ApiAclDestination = response.json().await;
    let expected_response = edit_destination_data_into_api_response(
        destination,
        response_destination.id,
        None,
        AliasState::Applied,
        Vec::new(),
    );
    assert_eq!(response_destination, expected_response);

    // list
    let response = client.get("/api/v1/acl/destination").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_destinations: Vec<ApiAclDestination> = response.json().await;
    assert_eq!(response_destinations.len(), 1);
    let response_destination = response_destinations[0].clone();
    assert_eq!(response_destination, expected_response);

    // retrieve
    let response = client.get("/api/v1/acl/destination/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_destination: ApiAclDestination = response.json().await;
    assert_eq!(response_destination, expected_response);

    // update
    let mut destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    destination.name = "modified".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_destination: ApiAclDestination = response.json().await;
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/2")
        .send()
        .await
        .json()
        .await;
    assert_eq!(response_destination, destination);

    // delete
    let response = client.delete("/api/v1/acl/destination/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/acl/destination").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_destinations: Vec<ApiAclDestination> = response.json().await;
    assert_eq!(response_destinations.len(), 0);
}

#[sqlx::test]
async fn test_destination_enterprise(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    exceed_enterprise_limits(&client).await;

    // unset the license
    let license = get_cached_license().clone();
    set_cached_license(None);

    // try to use ACL api
    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    // GET should be allowed
    let response = client.get("/api/v1/acl/destination").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.delete("/api/v1/acl/destination/1").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // restore valid license and try again
    set_cached_license(license);
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client.get("/api/v1/acl/destination").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response_destinations: Vec<Value> = response.json().await;
    assert_eq!(response_destinations.len(), 1);
    let response = client.get("/api/v1/acl/destination").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.delete("/api/v1/acl/destination/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_destination_create_modify_state(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    let destination = make_destination();

    // assert created destination has correct state
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let dbdestination = AclAlias::find_by_id_and_kind(&pool, 1, AliasKind::Destination)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(dbdestination.state, AliasState::Applied);
    assert_eq!(dbdestination.parent_id, None);

    // test APPLIED destination modification
    let destination_before_mods: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    let mut destination_modified = destination_before_mods.clone();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination_modified)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 2);
    let destination_parent: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    let destination_child: ApiAclDestination = client
        .get("/api/v1/acl/destination/2")
        .send()
        .await
        .json()
        .await;
    assert_eq!(destination_parent, destination_before_mods);
    assert_eq!(destination_parent.state, AliasState::Applied);
    destination_modified.id = 2;
    destination_modified.state = AliasState::Modified;
    destination_modified.parent_id = Some(1);
    assert_eq!(destination_child, destination_modified);
    assert_eq!(destination_child.state, AliasState::Modified);
    assert_eq!(destination_child.parent_id, Some(destination_parent.id));
}

#[sqlx::test]
async fn test_destination_delete(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    // create destination
    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let destination: Value = response.json().await;
    let destination_id = destination["id"].as_i64().unwrap();
    assert_eq!(count_destinations(&pool).await, 1);

    // use destination in a new rule
    let mut rule = make_rule();
    rule.use_manual_destination_settings = false;
    rule.destinations = vec![destination_id];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // cannot delete destination if used by a rule
    let response = client
        .delete(format!("/api/v1/acl/destination/{destination_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(count_destinations(&pool).await, 1);

    // remove destination from rule
    rule.use_manual_destination_settings = true;
    rule.destinations = Vec::new();
    let response = client.put("/api/v1/acl/rule/1").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // delete alias
    let response = client
        .delete(format!("/api/v1/acl/destination/{destination_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 0);

    // create another destination
    let mut destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(count_destinations(&pool).await, 1);

    // modify destination
    destination.name = "modified".to_string();
    let response = client
        .put("/api/v1/acl/destination/2")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 2);

    // delete pending modification
    let response = client.delete("/api/v1/acl/destination/3").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 1);

    // modify destination again
    destination.name = "modified again".to_string();
    let response = client
        .put("/api/v1/acl/destination/2")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 2);

    // delete original destination
    let response = client.delete("/api/v1/acl/destination/2").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 0);
}

#[sqlx::test]
async fn test_destination_duplication(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // each modification of parent destination should remove the child and create a new one
    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // ensure we don't duplicate already modified / deleted destinations
    assert_eq!(count_destinations(&pool).await, 1);
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 2);
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 2);
    let response = client.delete("/api/v1/acl/destination/1").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 0);
}

#[sqlx::test]
async fn test_destination_application(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    // create new destination
    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify initial status
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    assert_eq!(destination.state, AliasState::Applied);

    // use destination in a new rule
    let mut rule = make_rule();
    rule.use_manual_destination_settings = false;
    rule.destinations = vec![1];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // verify rule assignment
    let mut destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    assert_eq!(destination.rules, vec![1]);

    // modify destination
    destination.name = "modified".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 2);

    // cannot apply already applied destination
    let response = client
        .put("/api/v1/acl/destination/apply")
        .json(&json!({ "destinations": [1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // apply modification
    let response = client
        .put("/api/v1/acl/destination/apply")
        .json(&json!({ "destinations": [2] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify destination was applied
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/2")
        .send()
        .await
        .json()
        .await;
    assert_eq!(destination.state, AliasState::Applied);
    assert_eq!(destination.parent_id, None);
    assert_eq!(destination.rules, vec![1]);

    // initial destination was deleted
    let response = client.get("/api/v1/acl/destination/1").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(count_destinations(&pool).await, 1);
}

#[sqlx::test]
async fn test_multiple_destinations_application(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_admin(&mut client).await;

    let destination_1 = make_destination();
    let destination_2 = make_destination();
    let destination_3 = make_destination();

    // create new destinations
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination_1)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination_2)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination_3)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // modify destinations
    let mut destination_1: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    destination_1.name = "modified 1".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination_1)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut destination_2: ApiAclDestination = client
        .get("/api/v1/acl/destination/2")
        .send()
        .await
        .json()
        .await;
    destination_2.name = "modified 2".to_string();
    let response = client
        .put("/api/v1/acl/destination/2")
        .json(&destination_2)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut destination_3: ApiAclDestination = client
        .get("/api/v1/acl/destination/3")
        .send()
        .await
        .json()
        .await;
    destination_3.name = "modified 3".to_string();
    let response = client
        .put("/api/v1/acl/destination/3")
        .json(&destination_3)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 6);

    // apply multiple destinations
    let response = client
        .put("/api/v1/acl/destination/apply")
        .json(&json!({ "destinations": [4, 6] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 4);

    // verify destination state
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/2")
        .send()
        .await
        .json()
        .await;
    assert_eq!(destination.state, AliasState::Applied);
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/4")
        .send()
        .await
        .json()
        .await;
    assert_eq!(destination.state, AliasState::Applied);
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/5")
        .send()
        .await
        .json()
        .await;
    assert_eq!(destination.state, AliasState::Modified);
    let destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/6")
        .send()
        .await
        .json()
        .await;
    assert_eq!(destination.state, AliasState::Applied);
}

#[sqlx::test]
async fn test_destination_requires_any_or_values(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // create destination with empty fields and no any flags
    let invalid_destination = json!({
        "name": "invalid destination",
        "addresses": "",
        "ports": "",
        "protocols": [],
        "any_address": false,
        "any_port": false,
        "any_protocol": false
    });
    let response = client
        .post("/api/v1/acl/destination")
        .json(&invalid_destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // try to create destinations with only some destination fields set
    let invalid_destination = json!({
        "name": "invalid destination",
        "addresses": "",
        "ports": "22, 443",
        "protocols": [],
        "any_address": false,
        "any_port": false,
        "any_protocol": true
    });
    let response = client
        .post("/api/v1/acl/destination")
        .json(&invalid_destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // create valid destination
    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let destination: Value = response.json().await;
    let destination_id = destination["id"].as_i64().unwrap();

    // update destination with empty fields and no any flags
    let invalid_update = json!({
        "name": "invalid update",
        "addresses": "",
        "ports": "",
        "protocols": [],
        "any_address": false,
        "any_port": false,
        "any_protocol": false
    });
    let response = client
        .put(format!("/api/v1/acl/destination/{destination_id}"))
        .json(&invalid_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // update destination with some destination fields set
    let invalid_update = json!({
        "name": "invalid update",
        "addresses": "",
        "ports": "5432",
        "protocols": [],
        "any_address": true,
        "any_port": false,
        "any_protocol": false
    });
    let response = client
        .put(format!("/api/v1/acl/destination/{destination_id}"))
        .json(&invalid_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // create valid destination with only "any" flags enabled
    let destination = json!({
        "name": "valid destination",
        "addresses": "",
        "ports": "",
        "protocols": [],
        "any_address": true,
        "any_port": true,
        "any_protocol": true
    });
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn test_destination_port_bounds(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let mut destination = make_destination();
    destination.name = "destination-max-port".to_string();
    destination.ports = "65535".to_string();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut destination = make_destination();
    destination.name = "destination-too-large-port".to_string();
    destination.ports = "65536".to_string();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut destination = make_destination();
    destination.name = "destination-max-range".to_string();
    destination.ports = "65534-65535".to_string();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut destination = make_destination();
    destination.name = "destination-too-large-range".to_string();
    destination.ports = "65535-65536".to_string();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_destination_rejects_invalid_port_ranges(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut destination = make_destination();
    destination.name = "destination-reversed-range".to_string();
    destination.ports = "200-100".to_string();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut destination = make_destination();
    destination.name = "destination-malformed-range".to_string();
    destination.ports = "1-2-3".to_string();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    destination.ports = "200-100".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    destination.ports = "1-2-3".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_destination_apply_rejects_alias(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool, config).await;
    authenticate_admin(&mut client).await;

    let alias = make_alias();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client
        .put("/api/v1/acl/destination/apply")
        .json(&json!({ "destinations": [1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
