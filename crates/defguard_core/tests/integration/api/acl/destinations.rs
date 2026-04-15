use tokio::time::sleep;

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
async fn test_destination_modify_pending_child_updates_in_place(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

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
    assert_eq!(count_destinations(&pool).await, 1);

    let applied_parent_before_update: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;

    let mut first_update = applied_parent_before_update.clone();
    first_update.name = "destination pending child".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&first_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 2);

    let pending_child_before_update: ApiAclDestination = client
        .get("/api/v1/acl/destination/2")
        .send()
        .await
        .json()
        .await;
    assert_eq!(pending_child_before_update.state, AliasState::Modified);
    assert_eq!(pending_child_before_update.parent_id, Some(1));

    let mut pending_child_update = pending_child_before_update.clone();
    pending_child_update.name = "destination pending child updated".to_string();
    let response = client
        .put("/api/v1/acl/destination/2")
        .json(&pending_child_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_pending_child: ApiAclDestination = response.json().await;

    let destinations = AclAlias::all_of_kind(&pool, AliasKind::Destination)
        .await
        .unwrap();
    assert_eq!(destinations.len(), 2);
    assert_eq!(
        destinations
            .iter()
            .filter(|destination| destination.state == AliasState::Applied)
            .count(),
        1
    );
    assert_eq!(
        destinations
            .iter()
            .filter(|destination| destination.state == AliasState::Modified)
            .count(),
        1
    );

    let response = client.get("/api/v1/acl/destination/3").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let applied_parent_after_update: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    assert_eq!(applied_parent_after_update, applied_parent_before_update);
    assert_eq!(applied_parent_after_update.state, AliasState::Applied);
    assert_eq!(applied_parent_after_update.parent_id, None);

    let mut expected_pending_child = pending_child_before_update.clone();
    expected_pending_child.name = "destination pending child updated".to_string();
    assert_eq!(updated_pending_child, expected_pending_child);

    let pending_child_after_update: ApiAclDestination = client
        .get("/api/v1/acl/destination/2")
        .send()
        .await
        .json()
        .await;
    assert_eq!(pending_child_after_update, expected_pending_child);
    assert_eq!(
        pending_child_after_update.id,
        pending_child_before_update.id
    );
    assert_eq!(pending_child_after_update.state, AliasState::Modified);
    assert_eq!(pending_child_after_update.parent_id, Some(1));
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
async fn test_destination_audit_fields_track_acting_user_across_mutations(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let config = init_config(None, &pool).await;
    let mut client = make_client_v2(pool.clone(), config).await;
    authenticate_promoted_admin(&mut client, &pool, "hpotter").await;

    let destination = make_destination();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created_destination: ApiAclDestination = response.json().await;

    let created_destination_row =
        AclAlias::find_by_id_and_kind(&pool, created_destination.id, AliasKind::Destination)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(created_destination_row.modified_by, "hpotter");
    assert_ne!(created_destination_row.modified_by, "admin");
    let created_modified_at = created_destination_row.modified_at;

    sleep(std::time::Duration::from_millis(2)).await;

    let mut destination_update = created_destination.clone();
    destination_update.name = "destination updated by hpotter".to_string();
    let response = client
        .put(format!(
            "/api/v1/acl/destination/{}",
            created_destination.id
        ))
        .json(&destination_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated_destination: ApiAclDestination = response.json().await;

    let updated_destination_row =
        AclAlias::find_by_id_and_kind(&pool, updated_destination.id, AliasKind::Destination)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(updated_destination_row.state, AliasState::Modified);
    assert_eq!(updated_destination_row.modified_by, "hpotter");
    assert_eq!(
        updated_destination_row.name,
        "destination updated by hpotter"
    );
    assert!(updated_destination_row.modified_at > created_modified_at);
    let updated_modified_at = updated_destination_row.modified_at;

    sleep(std::time::Duration::from_millis(2)).await;

    let response = client
        .put("/api/v1/acl/destination/apply")
        .json(&json!({ "destinations": [updated_destination.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let applied_destination_row =
        AclAlias::find_by_id_and_kind(&pool, updated_destination.id, AliasKind::Destination)
            .await
            .unwrap()
            .unwrap();
    assert_eq!(applied_destination_row.state, AliasState::Applied);
    assert_eq!(applied_destination_row.modified_by, "hpotter");
    assert_ne!(applied_destination_row.modified_by, "admin");
    assert!(applied_destination_row.modified_at > updated_modified_at);
}

#[sqlx::test]
async fn test_destination_apply_after_delete_recreate_preserves_rule_association(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

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

    let mut rule = make_rule();
    rule.use_manual_destination_settings = false;
    rule.destinations = vec![1];
    let response = client.post("/api/v1/acl/rule").json(&rule).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let applied_parent_before_update: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    assert_eq!(applied_parent_before_update.rules, vec![1]);

    let mut first_update = applied_parent_before_update.clone();
    first_update.name = "destination pending child".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&first_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let pending_child_before_delete: ApiAclDestination = client
        .get("/api/v1/acl/destination/2")
        .send()
        .await
        .json()
        .await;
    assert_eq!(pending_child_before_delete.state, AliasState::Modified);
    assert_eq!(pending_child_before_delete.parent_id, Some(1));

    let response = client.delete("/api/v1/acl/destination/2").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(count_destinations(&pool).await, 1);

    let mut recreated_child_update: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    recreated_child_update.name = "destination pending child recreated".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&recreated_child_update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let recreated_child: ApiAclDestination = response.json().await;
    assert_eq!(recreated_child.state, AliasState::Modified);
    assert_eq!(recreated_child.parent_id, Some(1));

    let response = client
        .put("/api/v1/acl/destination/apply")
        .json(&json!({ "destinations": [recreated_child.id] }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let applied_recreated_child: ApiAclDestination = client
        .get(format!("/api/v1/acl/destination/{}", recreated_child.id))
        .send()
        .await
        .json()
        .await;
    assert_eq!(applied_recreated_child.state, AliasState::Applied);
    assert_eq!(applied_recreated_child.parent_id, None);
    assert_eq!(applied_recreated_child.rules, vec![1]);

    let response = client.get("/api/v1/acl/destination/1").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(count_destinations(&pool).await, 1);

    let applied_rule: ApiAclRule = client.get("/api/v1/acl/rule/1").send().await.json().await;
    assert_eq!(applied_rule.destinations, vec![recreated_child.id]);
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
async fn test_destination_rejects_invalid_address_ranges(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
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

    for (name, addresses) in [
        ("destination-reversed-address-range", "10.0.0.2-10.0.0.1"),
        ("destination-equal-address-range", "10.0.0.1-10.0.0.1"),
        ("destination-mixed-address-range", "10.0.0.1-2001:db8::1"),
        (
            "destination-multi-dash-address-range",
            "10.0.0.1-10.0.0.2-10.0.0.3",
        ),
        ("destination-cidr-endpoint-range", "10.0.0.0/24-10.0.0.2"),
        ("destination-multi-slash-ipv4-cidr", "10.0.0.1/24/25"),
        ("destination-multi-slash-ipv6-cidr", "2001:db8::1/64/65"),
        ("destination-scientific-notation-prefix", "10.0.0.1/1e1"),
        ("destination-hex-prefix", "10.0.0.1/0x18"),
        ("destination-trailing-text-ipv6-prefix", "2001:db8::1/64foo"),
    ] {
        let mut invalid_destination = make_destination();
        invalid_destination.name = name.to_string();
        invalid_destination.addresses = addresses.to_string();
        let response = client
            .post("/api/v1/acl/destination")
            .json(&invalid_destination)
            .send()
            .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{name}"
        );
    }

    let mut valid_destination = make_destination();
    valid_destination.name = "destination-valid-address-range".to_string();
    valid_destination.addresses = "10.0.0.1-10.0.0.2".to_string();
    let response = client
        .post("/api/v1/acl/destination")
        .json(&valid_destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let mut destination: ApiAclDestination = client
        .get("/api/v1/acl/destination/1")
        .send()
        .await
        .json()
        .await;
    for addresses in [
        "10.0.0.2-10.0.0.1",
        "10.0.0.1-10.0.0.1",
        "10.0.0.1-2001:db8::1",
        "10.0.0.1-10.0.0.2-10.0.0.3",
        "10.0.0.0/24-10.0.0.2",
        "10.0.0.1/24/25",
        "2001:db8::1/64/65",
        "10.0.0.1/1e1",
        "10.0.0.1/0x18",
        "2001:db8::1/64foo",
    ] {
        destination.addresses = addresses.to_string();
        let response = client
            .put("/api/v1/acl/destination/1")
            .json(&destination)
            .send()
            .await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{addresses}"
        );
    }

    destination.addresses = "2001:db8::1-2001:db8::2".to_string();
    let response = client
        .put("/api/v1/acl/destination/1")
        .json(&destination)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
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
