use defguard_common::db::{
    models::{
        Settings, User, mfa_flow::MfaFlow, settings::update_current_settings,
        vpn_client_session::VpnClientMfaMethod,
    },
    setup_pool,
};
use defguard_core::{
    enterprise::license::{get_cached_license, set_cached_license},
    events::ApiEventType,
};
use matches::assert_matches;
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    authenticate_admin, configure_smtp, make_network, make_test_client, set_enterprise_license,
    update_location_mfa_flows,
};

/// Single-step flow without OIDC - should succeed without any license.
#[sqlx::test]
async fn test_mfa_flow_single_step_no_license(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let saved = get_cached_license().clone();

    set_cached_license(None);
    let response = client
        .post("/api/v1/mfa-flow")
        .json(&json!({
            "title": "Test Flow",
            "steps": [{ "methods": ["totp"] }]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = response.json::<serde_json::Value>().await;
    let created_id = created["id"].as_i64().unwrap();

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1, "expected exactly 1 event after create");
    let (event_type, _user_id, _username) = &events[0];
    assert_matches!(
        event_type,
        ApiEventType::MfaFlowCreated { snapshot }
            if snapshot.flow.id == created_id
                && snapshot.flow.title == "Test Flow"
                && snapshot.steps.len() == 1
                && snapshot.steps[0].methods == vec![VpnClientMfaMethod::Totp]
    );

    set_cached_license(saved);
}

/// A free instance may create one flow, while subsequent flows require Business.
#[sqlx::test]
async fn test_additional_mfa_flow_requires_business(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let saved = get_cached_license().clone();
    let first_flow = json!({
        "title": "Free Flow",
        "steps": [{ "methods": ["totp"] }]
    });
    let second_flow = json!({
        "title": "Business Flow",
        "steps": [{ "methods": ["biometric"] }]
    });

    set_cached_license(None);
    let response = client
        .post("/api/v1/mfa-flow")
        .json(&first_flow)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    client.drain_all_events();

    let response = client
        .post("/api/v1/mfa-flow")
        .json(&second_flow)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["error"], "license_required");
    assert_eq!(body["fields"][0]["field"], "flow");
    assert_eq!(
        body["fields"][0]["code"],
        "additional_flow_business_license_required"
    );
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );

    set_cached_license(saved.clone());
    let response = client
        .post("/api/v1/mfa-flow")
        .json(&second_flow)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    set_cached_license(saved);
}

/// Multi-step flow (2+ steps) requires a business license.
#[sqlx::test]
async fn test_mfa_flow_multi_step_requires_business(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let saved = get_cached_license().clone();

    let body = json!({
        "title": "Multi-Step Flow",
        "steps": [
            { "methods": ["totp"] },
            { "methods": ["biometric"] }
        ]
    });

    // No license → 403, and a refused request must not emit an audit event.
    set_cached_license(None);
    let response = client.post("/api/v1/mfa-flow").json(&body).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );

    // Business license → 201
    set_cached_license(saved.clone());
    let response = client.post("/api/v1/mfa-flow").json(&body).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = response.json::<serde_json::Value>().await;
    let created_id = created["id"].as_i64().unwrap();

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1, "expected exactly 1 event after create");
    let (event_type, _user_id, _username) = &events[0];
    assert_matches!(
        event_type,
        ApiEventType::MfaFlowCreated { snapshot }
            if snapshot.flow.id == created_id && snapshot.steps.len() == 2
    );

    set_cached_license(saved);
}

/// OIDC method requires a business license + a configured OIDC provider.
#[sqlx::test]
async fn test_mfa_flow_oidc_requires_business_and_provider(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let saved = get_cached_license().clone();

    let body = json!({
        "title": "OIDC Flow",
        "steps": [{ "methods": ["oidc"] }]
    });

    // No license → 403
    set_cached_license(None);
    let response = client.post("/api/v1/mfa-flow").json(&body).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );

    // Business license but no OIDC provider → 400
    set_cached_license(saved.clone());
    let response = client.post("/api/v1/mfa-flow").json(&body).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["error"], "validation_failed");
    assert_eq!(body["fields"][0]["field"], "steps[0].methods");
    assert_eq!(body["fields"][0]["code"], "oidc_provider_missing");
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );

    set_cached_license(saved);
}

/// The Email method cannot be saved while SMTP is unconfigured, otherwise a flow would reference a
/// factor the instance is unable to deliver.
#[sqlx::test]
async fn test_mfa_flow_email_requires_smtp(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let response = client
        .post("/api/v1/mfa-flow")
        .json(&json!({
            "title": "Email Flow",
            "steps": [{ "methods": ["totp"] }, { "methods": ["email"] }]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = response.json().await;
    assert_eq!(body["error"], "validation_failed");
    // The field path must point at the offending step so the editor can highlight that row.
    assert_eq!(body["fields"][0]["field"], "steps[1].methods");
    assert_eq!(body["fields"][0]["code"], "smtp_not_configured");
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );
}

/// Group-scoped assignments require an enterprise license.
#[sqlx::test]
async fn test_mfa_flow_group_scoping_requires_enterprise(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let saved = get_cached_license().clone();

    // Create two single-step flows
    let flow1_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({
                "title": "Default Flow",
                "steps": [{ "methods": ["totp"] }]
            }))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };
    let flow2_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({
                "title": "Scoped Flow",
                "steps": [{ "methods": ["biometric"] }]
            }))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };

    // Get the admin group ID via group-info endpoint
    let groups_resp = client.get("/api/v1/group-info").send().await;
    assert_eq!(groups_resp.status(), StatusCode::OK);
    let groups = groups_resp.json::<serde_json::Value>().await;
    let admin_group_id = groups
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|g| g["id"].as_i64())
        .expect("admin group exists");

    // Create a location
    let network_resp = make_network(&client, "enterprise-test").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    // Default assignment (empty group_ids) + scoped assignment (non-empty)
    let assignment_body = json!([
        {
            "flow_id": flow1_id,
            "is_default": true,
            "group_ids": []
        },
        {
            "flow_id": flow2_id,
            "is_default": false,
            "group_ids": [admin_group_id]
        }
    ]);

    set_enterprise_license();
    let _ = client.drain_all_events();
    let response = update_location_mfa_flows(&client, location_id, assignment_body).await;
    assert_eq!(response.status(), StatusCode::OK);

    let events = client.drain_all_events();
    assert_eq!(
        events.len(),
        2,
        "location save must emit assignment and modification events"
    );
    let (event_type, _user_id, _username) = events
        .iter()
        .find(|event| matches!(event.0, ApiEventType::LocationMfaFlowsAssigned { .. }))
        .expect("missing MFA assignment event");
    assert_matches!(
        event_type,
        ApiEventType::LocationMfaFlowsAssigned {
            location_id: ev_location_id,
            location_name,
            assignments,
        } if *ev_location_id == location_id
            && location_name == "enterprise-test"
            && assignments.len() == 2
            && assignments[0].flow_id == flow1_id
            && assignments[0].position == 0
            && assignments[0].is_default
            && assignments[0].group_ids.is_empty()
            && assignments[1].flow_id == flow2_id
            && assignments[1].position == 1
            && !assignments[1].is_default
            && assignments[1].group_ids == vec![admin_group_id]
    );

    set_cached_license(saved);
}

/// Multi-step guard also applies to updates.
#[sqlx::test]
async fn test_mfa_flow_update_multi_step_requires_business(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let saved = get_cached_license().clone();

    // Create a single-step flow first (allowed)
    let create_resp = client
        .post("/api/v1/mfa-flow")
        .json(&json!({
            "title": "To Be Updated",
            "steps": [{ "methods": ["totp"] }]
        }))
        .send()
        .await;
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let flow_id = create_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    // Clear the create event before exercising the refusal path.
    let _ = client.drain_all_events();

    // Try updating to multi-step without license → 403, and no audit event on refusal.
    set_cached_license(None);
    let response = client
        .put(format!("/api/v1/mfa-flow/{flow_id}"))
        .json(&json!({
            "title": "Updated Flow",
            "steps": [
                { "methods": ["totp"] },
                { "methods": ["biometric"] }
            ]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );

    // Restore license → update succeeds
    set_cached_license(saved.clone());
    let response = client
        .put(format!("/api/v1/mfa-flow/{flow_id}"))
        .json(&json!({
            "title": "Updated Flow",
            "steps": [
                { "methods": ["totp"] },
                { "methods": ["biometric"] }
            ]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1, "expected exactly 1 event after update");
    let (event_type, _user_id, _username) = &events[0];
    assert_matches!(
        event_type,
        ApiEventType::MfaFlowUpdated { before, after }
            if before.flow.title == "To Be Updated"
                && before.steps.len() == 1
                && after.flow.title == "Updated Flow"
                && after.steps.len() == 2
    );

    set_cached_license(saved);
}

/// A step id belonging to another flow must be refused, not silently applied. Reconciliation
/// UPDATEs by step id, so an unscoped write would rewrite the other flow's step and report it as
/// this flow's.
#[sqlx::test]
async fn test_mfa_flow_update_rejects_foreign_step_id(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let resp = client
        .post("/api/v1/mfa-flow")
        .json(&json!({"title": "Flow A", "steps": [{ "methods": ["totp"] }]}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let flow_a: serde_json::Value = resp.json().await;

    let resp = client
        .post("/api/v1/mfa-flow")
        .json(&json!({"title": "Flow B", "steps": [{ "methods": ["totp"] }]}))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let flow_b: serde_json::Value = resp.json().await;

    let flow_a_id = flow_a["id"].as_i64().unwrap();
    let flow_b_id = flow_b["id"].as_i64().unwrap();
    let flow_b_step_id = flow_b["steps"][0]["id"].as_i64().unwrap();

    // Clear the two create events before exercising the refusal path.
    let _ = client.drain_all_events();

    // Update flow A, but hand it flow B's step id.
    let response = client
        .put(format!("/api/v1/mfa-flow/{flow_a_id}"))
        .json(&json!({
            "title": "Flow A",
            "steps": [{ "id": flow_b_step_id, "methods": ["biometric"] }]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["fields"][0]["code"], "unknown_step");
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );

    // Flow B must be untouched.
    let response = client
        .get(format!("/api/v1/mfa-flow/{flow_b_id}"))
        .send()
        .await;
    let flow_b_after: serde_json::Value = response.json().await;
    assert_eq!(
        flow_b_after["steps"], flow_b["steps"],
        "the other flow's steps must not have been rewritten"
    );
}

/// Assignment input that cannot be satisfied is a validation error, not a 500 from a constraint
/// violation, and an unknown location is a 404 rather than an empty list.
#[sqlx::test]
async fn test_location_mfa_flows_input_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let network_resp = make_network(&client, "assignment-validation").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();
    let flow_resp = client
        .post("/api/v1/mfa-flow")
        .json(&json!({"title": "Flow", "steps": [{ "methods": ["totp"] }]}))
        .send()
        .await;
    let flow_id = flow_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    // Clear the create event so the refusal assertions below are exact.
    let _ = client.drain_all_events();

    // Unknown location → 404, not an empty list.
    let response = client.get("/api/v1/location/999999/mfa-flows").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = update_location_mfa_flows(
        &client,
        999999,
        json!([{"flow_id": flow_id, "is_default": true, "group_ids": []}]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // The same flow twice would violate the (location_id, flow_id) primary key.
    let response = update_location_mfa_flows(
        &client,
        location_id,
        json!([
            {"flow_id": flow_id, "is_default": true, "group_ids": []},
            {"flow_id": flow_id, "is_default": false, "group_ids": []},
        ]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json::<serde_json::Value>().await["fields"][0]["code"],
        "duplicate"
    );

    // A nonexistent flow would violate the foreign key.
    let response = update_location_mfa_flows(
        &client,
        location_id,
        json!([
            {"flow_id": 999999, "is_default": true, "group_ids": []},
        ]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json::<serde_json::Value>().await["fields"][0]["code"],
        "unknown_flow"
    );

    // None of the refused requests above may have emitted an audit event.
    assert!(
        client.drain_all_events().is_empty(),
        "refused requests must not emit audit events"
    );
}

/// A non-default assignment with an empty group set can never match any user, so it must be
/// rejected with a field path pointing at the offending entry's `group_ids`.
#[sqlx::test]
async fn test_location_mfa_flows_non_default_without_groups(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let flow1_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({"title": "Scoped", "steps": [{"methods": ["totp"]}]}))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };
    let flow2_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({"title": "Default", "steps": [{"methods": ["biometric"]}]}))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };

    let network_resp = make_network(&client, "non-default-without-groups").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    // Clear the two create events before exercising the refusal path.
    let _ = client.drain_all_events();

    let response = update_location_mfa_flows(
        &client,
        location_id,
        json!([
            {"flow_id": flow1_id, "is_default": false, "group_ids": []},
            {"flow_id": flow2_id, "is_default": true, "group_ids": []},
        ]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["error"], "validation_failed");
    assert_eq!(body["fields"][0]["field"], "assignments[0].group_ids");
    assert_eq!(body["fields"][0]["code"], "non_default_must_have_groups");
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );
}

/// An MFA-disabled location's assignment list can be cleared to empty via the API.
#[sqlx::test]
async fn test_location_mfa_flows_clear_disabled_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let flow_resp = client
        .post("/api/v1/mfa-flow")
        .json(&json!({"title": "Flow", "steps": [{"methods": ["totp"]}]}))
        .send()
        .await;
    let flow_id = flow_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    let network_resp = make_network(&client, "clear-disabled").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    // Clear the create/location events so the assignment assertions below are exact.
    let _ = client.drain_all_events();

    // Assign a default, then clear it.
    let response = update_location_mfa_flows(
        &client,
        location_id,
        json!([
            {"flow_id": flow_id, "is_default": true, "group_ids": []},
        ]),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let events = client.drain_all_events();
    assert_eq!(
        events.len(),
        2,
        "location save must emit assignment and modification events"
    );
    let (event_type, _user_id, _username) = events
        .iter()
        .find(|event| matches!(event.0, ApiEventType::LocationMfaFlowsAssigned { .. }))
        .expect("missing MFA assignment event");
    assert_matches!(
        event_type,
        ApiEventType::LocationMfaFlowsAssigned {
            location_id: ev_location_id,
            assignments,
            ..
        } if *ev_location_id == location_id
            && assignments.len() == 1
            && assignments[0].flow_id == flow_id
            && assignments[0].position == 0
            && assignments[0].is_default
            && assignments[0].group_ids.is_empty()
    );

    let response = update_location_mfa_flows(&client, location_id, json!([])).await;
    assert_eq!(response.status(), StatusCode::OK);

    let events = client.drain_all_events();
    assert_eq!(
        events.len(),
        2,
        "location save must emit assignment and modification events"
    );
    let (event_type, _user_id, _username) = events
        .iter()
        .find(|event| matches!(event.0, ApiEventType::LocationMfaFlowsAssigned { .. }))
        .expect("missing MFA assignment event");
    assert_matches!(
        event_type,
        ApiEventType::LocationMfaFlowsAssigned {
            location_id: ev_location_id,
            assignments,
            ..
        } if *ev_location_id == location_id && assignments.is_empty()
    );

    let response = client
        .get(format!("/api/v1/location/{location_id}/mfa-flows"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .json::<serde_json::Value>()
            .await
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

/// Method availability returns all five methods with correct availability.
#[sqlx::test]
async fn test_method_availability_basic(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let saved = get_cached_license().clone();

    let response = client
        .get("/api/v1/mfa-flow/method-availability")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let items = response.json::<serde_json::Value>().await;
    let items = items.as_array().unwrap();
    assert_eq!(items.len(), 5);

    let find = |method: &str| -> &serde_json::Value {
        items
            .iter()
            .find(|m| m["method"].as_str() == Some(method))
            .unwrap()
    };

    assert_eq!(find("totp")["available"].as_bool(), Some(true));
    assert_eq!(find("email")["available"].as_bool(), Some(false));
    assert_eq!(
        find("email")["reason"].as_str(),
        Some("smtp_not_configured")
    );
    assert_eq!(find("oidc")["available"].as_bool(), Some(false));
    assert_eq!(
        find("oidc")["reason"].as_str(),
        Some("oidc_provider_missing")
    );
    assert_eq!(find("biometric")["available"].as_bool(), Some(true));
    assert_eq!(find("mobileapprove")["available"].as_bool(), Some(true));

    set_cached_license(None);
    let response = client
        .get("/api/v1/mfa-flow/method-availability")
        .send()
        .await;
    let items = response.json::<serde_json::Value>().await;
    let items = items.as_array().unwrap();
    let find = |method: &str| -> &serde_json::Value {
        items
            .iter()
            .find(|m| m["method"].as_str() == Some(method))
            .unwrap()
    };
    assert_eq!(find("oidc")["available"].as_bool(), Some(false));
    assert_eq!(find("oidc")["reason"].as_str(), Some("licensed"));

    set_cached_license(saved);
}

/// Updating a flow that already contains email (e.g. backfilled from a
/// migration) must succeed even when SMTP is not configured, as long as email
/// was already present in the flow. Adding email where it did not exist
/// before must still be rejected.
#[sqlx::test]
async fn test_mfa_flow_update_preserves_backfilled_email(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool.clone()).await;
    authenticate_admin(&mut client).await;

    // Enable SMTP so we can create a flow with email.
    let mut settings = Settings::get_current_settings();
    configure_smtp(&mut settings);
    update_current_settings(&pool, settings).await.unwrap();

    // Create a flow with email - this represents the backfilled "Default
    // Internal MFA" flow.
    let resp = client
        .post("/api/v1/mfa-flow")
        .json(&json!({
            "title": "Flow With Email",
            "steps": [{ "methods": ["totp", "email"] }]
        }))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created: serde_json::Value = resp.json().await;
    let flow_id = created["id"].as_i64().unwrap();

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1, "expected exactly 1 event after create");
    let (event_type, _user_id, _username) = &events[0];
    assert_matches!(
        event_type,
        ApiEventType::MfaFlowCreated { snapshot } if snapshot.flow.id == flow_id
    );

    // Remove SMTP.
    let mut settings = Settings::get_current_settings();
    settings.smtp.server = None;
    settings.smtp.port = None;
    settings.smtp.sender = None;
    update_current_settings(&pool, settings).await.unwrap();

    // Update the flow keeping email unchanged -> should succeed.
    let resp = client
        .put(format!("/api/v1/mfa-flow/{flow_id}"))
        .json(&json!({
            "title": "Flow With Email Updated",
            "steps": created["steps"]
        }))
        .send()
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "updating a flow with unchanged email must succeed even without SMTP"
    );

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1, "expected exactly 1 event after update");
    let (event_type, _user_id, _username) = &events[0];
    assert_matches!(
        event_type,
        ApiEventType::MfaFlowUpdated { before, after }
            if before.flow.title == "Flow With Email"
                && after.flow.title == "Flow With Email Updated"
    );

    // Update the flow adding email to a new step -> must be rejected.
    let resp = client
        .put(format!("/api/v1/mfa-flow/{flow_id}"))
        .json(&json!({
            "title": "Flow With Email Updated",
            "steps": [
                { "methods": ["totp"] },
                { "methods": ["email"] }
            ]
        }))
        .send()
        .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "adding email to a new step must still be rejected without SMTP"
    );
    let body: serde_json::Value = resp.json().await;
    assert_eq!(body["fields"][0]["field"], "steps[1].methods");
    assert_eq!(body["fields"][0]["code"], "smtp_not_configured");
    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );
}

/// The full `WireguardNetworkData` body used to toggle `mfa_enabled` on an existing location.
fn network_body(name: &str, mfa_enabled: bool, flow_id: i64) -> serde_json::Value {
    json!({
        "name": name,
        "address": "10.1.1.1/24",
        "port": 55555,
        "endpoint": "192.168.4.14",
        "allowed_ips": "10.1.1.0/24",
        "dns": "1.1.1.1",
        "mtu": 1420,
        "fwmark": 0,
        "allowed_groups": ["admin"],
        "allow_all_groups": false,
        "keepalive_interval": 25,
        "peer_disconnect_threshold": 300,
        "acl_enabled": false,
        "acl_default_allow": false,
        "allowed_ips_from_acl": false,
        "mfa_enabled": mfa_enabled,
        "service_location_mode": "disabled",
        "posture_checks": [],
        "mfa_flows": [{"flow_id": flow_id, "is_default": true, "group_ids": []}]
    })
}

/// Disabling MFA preserves the assignment list (and its default designation), and re-enabling
/// restores the same policy: the ADR's guarantee that the off toggle is non-destructive.
#[sqlx::test]
async fn test_mfa_enabled_disable_preserves_assignments(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool.clone()).await;
    authenticate_admin(&mut client).await;

    let flow_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({"title": "Lifecycle Flow", "steps": [{"methods": ["totp"]}]}))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };

    let network_resp = make_network(&client, "mfa-lifecycle").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    // Assign the flow as the location's default.
    let resp = update_location_mfa_flows(
        &client,
        location_id,
        json!([
            {"flow_id": flow_id, "is_default": true, "group_ids": []},
        ]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Enable MFA, then disable it: the assignment list must survive untouched.
    let resp = client
        .put(format!("/api/v1/network/{location_id}"))
        .json(&network_body("mfa-lifecycle", true, flow_id))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let license = get_cached_license().clone();
    set_cached_license(None);
    let body = network_body("mfa-lifecycle", false, flow_id);
    let resp = client
        .put(format!("/api/v1/network/{location_id}"))
        .json(&body)
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let resp = client
        .get(format!("/api/v1/location/{location_id}/mfa-flows"))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let assignments = resp.json::<serde_json::Value>().await;
    let assignments = assignments.as_array().unwrap();
    assert_eq!(
        assignments.len(),
        1,
        "disabling MFA must preserve the assignment list"
    );
    assert_eq!(assignments[0]["id"].as_i64(), Some(flow_id));
    assert_eq!(assignments[0]["is_default"].as_bool(), Some(true));

    // Re-enable: the same policy must be in force, resolving the same flow for a user.
    set_cached_license(license);
    let resp = client
        .put(format!("/api/v1/network/{location_id}"))
        .json(&network_body("mfa-lifecycle", true, flow_id))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let user = User::find_by_username(&pool, "hpotter")
        .await
        .unwrap()
        .unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let resolved = MfaFlow::resolve_for_user(&mut conn, location_id, user.id)
        .await
        .unwrap()
        .expect("a default assignment must resolve");
    assert_eq!(
        resolved.0.id, flow_id,
        "re-enabling must restore the same resolved flow"
    );
}

/// Deleting the only flow assigned to an MFA-enabled location is refused with
/// `location_requires_flow` (409), naming the location, and emits no audit event.
#[sqlx::test]
async fn test_mfa_flow_delete_location_requires_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let flow_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({"title": "Sole Flow", "steps": [{"methods": ["totp"]}]}))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };

    let network_resp = make_network(&client, "delete-orphan").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    let resp = update_location_mfa_flows(
        &client,
        location_id,
        json!([
            {"flow_id": flow_id, "is_default": true, "group_ids": []},
        ]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Enable MFA so the location requires this flow.
    let resp = client
        .put(format!("/api/v1/network/{location_id}"))
        .json(&network_body("delete-orphan", true, flow_id))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Drain the create/location/assign/enable events before the refusal.
    let _ = client.drain_all_events();

    let resp = client
        .delete(format!("/api/v1/mfa-flow/{flow_id}"))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: serde_json::Value = resp.json().await;
    assert_eq!(body["error"], "conflict");
    assert_eq!(body["fields"][0]["field"], "id");
    assert_eq!(body["fields"][0]["code"], "location_requires_flow");
    assert_eq!(body["fields"][0]["locations"], json!(["delete-orphan"]));

    assert!(
        client.drain_all_events().is_empty(),
        "refused delete must not emit an audit event"
    );
}

/// Deleting a flow that is a location's designated default is refused with `flow_is_default`
/// (409), distinct from `location_requires_flow`, naming the location.
#[sqlx::test]
async fn test_mfa_flow_delete_flow_is_default(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    let saved = get_cached_license().clone();

    let flow1_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({"title": "Default Flow", "steps": [{"methods": ["totp"]}]}))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };
    let flow2_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({"title": "Scoped Flow", "steps": [{"methods": ["biometric"]}]}))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };

    let groups_resp = client.get("/api/v1/group-info").send().await;
    let groups = groups_resp.json::<serde_json::Value>().await;
    let admin_group_id = groups
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|g| g["id"].as_i64())
        .expect("admin group exists");

    let network_resp = make_network(&client, "delete-default").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    // flow1 is the default, flow2 is group-scoped; group scoping needs Enterprise.
    set_enterprise_license();
    let resp = update_location_mfa_flows(
        &client,
        location_id,
        json!([
            {"flow_id": flow1_id, "is_default": true, "group_ids": []},
            {"flow_id": flow2_id, "is_default": false, "group_ids": [admin_group_id]},
        ]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let _ = client.drain_all_events();

    // Deleting the default is refused with the distinct `flow_is_default` code.
    let resp = client
        .delete(format!("/api/v1/mfa-flow/{flow1_id}"))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
    let body: serde_json::Value = resp.json().await;
    assert_eq!(body["error"], "conflict");
    assert_eq!(body["fields"][0]["code"], "flow_is_default");
    assert_eq!(body["fields"][0]["locations"], json!(["delete-default"]));
    assert!(
        client.drain_all_events().is_empty(),
        "refused delete must not emit an audit event"
    );

    // The non-default flow deletes cleanly and emits `MfaFlowDeleted`.
    let resp = client
        .delete(format!("/api/v1/mfa-flow/{flow2_id}"))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let events = client.drain_all_events();
    assert_eq!(events.len(), 1);
    assert_matches!(
        &events[0].0,
        ApiEventType::MfaFlowDeleted { snapshot } if snapshot.flow.id == flow2_id
    );

    set_cached_license(saved);
}

/// Plain CRUD over HTTP: create, list (with `step_count`), fetch, update, and delete an
/// unassigned flow, asserting the audit events at each step.
#[sqlx::test]
async fn test_mfa_flow_crud(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    // Create a two-step flow (business license is active by default).
    let resp = client
        .post("/api/v1/mfa-flow")
        .json(&json!({
            "title": "CRUD Flow",
            "steps": [
                { "methods": ["totp"] },
                { "methods": ["biometric"] }
            ]
        }))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created: serde_json::Value = resp.json().await;
    let flow_id = created["id"].as_i64().unwrap();
    assert_eq!(created["steps"].as_array().unwrap().len(), 2);

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1, "expected exactly 1 event after create");
    assert_matches!(
        &events[0].0,
        ApiEventType::MfaFlowCreated { snapshot }
            if snapshot.flow.id == flow_id && snapshot.steps.len() == 2
    );

    // List: the item must carry the server-computed step_count.
    let resp = client.get("/api/v1/mfa-flow").send().await;
    assert_eq!(resp.status(), StatusCode::OK);
    let items = resp.json::<serde_json::Value>().await;
    let item = items
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"].as_i64() == Some(flow_id))
        .expect("created flow must appear in list");
    assert_eq!(item["step_count"].as_i64(), Some(2));

    // Fetch single.
    let resp = client
        .get(format!("/api/v1/mfa-flow/{flow_id}"))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let detail = resp.json::<serde_json::Value>().await;
    assert_eq!(detail["title"], "CRUD Flow");
    assert_eq!(detail["steps"].as_array().unwrap().len(), 2);

    // Update: rename and collapse to one step.
    let resp = client
        .put(format!("/api/v1/mfa-flow/{flow_id}"))
        .json(&json!({
            "title": "CRUD Flow Updated",
            "steps": [
                { "id": detail["steps"][0]["id"], "methods": ["totp"] }
            ]
        }))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1, "expected exactly 1 event after update");
    assert_matches!(
        &events[0].0,
        ApiEventType::MfaFlowUpdated { before, after }
            if before.flow.title == "CRUD Flow"
                && before.steps.len() == 2
                && after.flow.title == "CRUD Flow Updated"
                && after.steps.len() == 1
    );

    // Delete (unassigned) succeeds.
    let resp = client
        .delete(format!("/api/v1/mfa-flow/{flow_id}"))
        .send()
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1, "expected exactly 1 event after delete");
    assert_matches!(
        &events[0].0,
        ApiEventType::MfaFlowDeleted { snapshot } if snapshot.flow.id == flow_id
    );
}

/// Saving an assignment set with no designated default is refused over HTTP with
/// `no_default_designated` (400), never silently normalised.
#[sqlx::test]
async fn test_location_mfa_flows_no_default_designated(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;

    let flow_id = {
        let resp = client
            .post("/api/v1/mfa-flow")
            .json(&json!({"title": "No Default", "steps": [{"methods": ["totp"]}]}))
            .send()
            .await;
        assert_eq!(resp.status(), StatusCode::CREATED);
        resp.json::<serde_json::Value>().await["id"]
            .as_i64()
            .unwrap()
    };

    let network_resp = make_network(&client, "no-default").await;
    let location_id = network_resp.json::<serde_json::Value>().await["id"]
        .as_i64()
        .unwrap();

    let _ = client.drain_all_events();

    let resp = update_location_mfa_flows(
        &client,
        location_id,
        json!([
            {"flow_id": flow_id, "is_default": false, "group_ids": []},
        ]),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = resp.json().await;
    assert_eq!(body["error"], "validation_failed");
    assert_eq!(body["fields"][0]["code"], "no_default_designated");

    assert!(
        client.drain_all_events().is_empty(),
        "refused request must not emit an audit event"
    );
}
