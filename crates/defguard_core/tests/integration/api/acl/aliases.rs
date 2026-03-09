use super::*;

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

#[sqlx::test]
async fn test_alias_requires_any_value(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // all fields empty
    let mut alias = make_alias();
    alias.addresses = String::new();
    alias.ports = String::new();
    alias.protocols = Vec::new();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // only addresses set
    let mut alias = make_alias();
    alias.ports = String::new();
    alias.protocols = Vec::new();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // only ports set
    let mut alias = make_alias();
    alias.addresses = String::new();
    alias.protocols = Vec::new();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    // only protocols set
    let mut alias = make_alias();
    alias.addresses = String::new();
    alias.ports = String::new();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[sqlx::test]
async fn test_alias_port_bounds(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let mut alias = make_alias();
    alias.name = "alias-max-port".to_string();
    alias.ports = "65535".to_string();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut alias = make_alias();
    alias.name = "alias-too-large-port".to_string();
    alias.ports = "65536".to_string();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut alias = make_alias();
    alias.name = "alias-max-range".to_string();
    alias.ports = "65534-65535".to_string();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut alias = make_alias();
    alias.name = "alias-too-large-range".to_string();
    alias.ports = "65535-65536".to_string();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_alias_rejects_invalid_port_ranges(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let alias = make_alias();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut alias = make_alias();
    alias.name = "alias-reversed-range".to_string();
    alias.ports = "200-100".to_string();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut alias = make_alias();
    alias.name = "alias-malformed-range".to_string();
    alias.ports = "1-2-3".to_string();
    let response = client.post("/api/v1/acl/alias").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let mut alias: ApiAclAlias = client.get("/api/v1/acl/alias/1").send().await.json().await;
    alias.ports = "200-100".to_string();
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    alias.ports = "1-2-3".to_string();
    let response = client.put("/api/v1/acl/alias/1").json(&alias).send().await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test]
async fn test_alias_apply_rejects_destination(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool, config).await;
    authenticate_admin(&mut client).await;

    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = client
        .put("/api/v1/acl/alias/apply")
        .json(&json!({ "aliases": [1] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
