use super::*;

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
