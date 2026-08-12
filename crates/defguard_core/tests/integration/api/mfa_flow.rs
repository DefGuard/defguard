use defguard_common::db::{
    models::{Settings, settings::update_current_settings},
    setup_pool,
};
use defguard_core::enterprise::license::{get_cached_license, set_cached_license};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{
    authenticate_admin, configure_smtp, make_network, make_test_client, set_enterprise_license,
};

/// Single-step flow without OIDC — should succeed without any license.
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

    // No license → 403
    set_cached_license(None);
    let response = client.post("/api/v1/mfa-flow").json(&body).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Business license → 201
    set_cached_license(saved.clone());
    let response = client.post("/api/v1/mfa-flow").json(&body).send().await;
    assert_eq!(response.status(), StatusCode::CREATED);

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

    // Business license but no OIDC provider → 400
    set_cached_license(saved.clone());
    let response = client.post("/api/v1/mfa-flow").json(&body).send().await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = response.json().await;
    assert_eq!(body["error"], "validation_failed");
    assert_eq!(body["fields"][0]["field"], "steps[0].methods");
    assert_eq!(body["fields"][0]["code"], "oidc_provider_missing");

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
    let assignment_body = json!({
        "assignments": [
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
        ]
    });

    // Business license → 403 (group scoping needs enterprise)
    let response = client
        .put(format!("/api/v1/location/{location_id}/mfa-flows"))
        .json(&assignment_body)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Enterprise license → 200
    set_enterprise_license();
    let response = client
        .put(format!("/api/v1/location/{location_id}/mfa-flows"))
        .json(&assignment_body)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

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

    // Try updating to multi-step without license → 403
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

    // Unknown location → 404, not an empty list.
    let response = client.get("/api/v1/location/999999/mfa-flows").send().await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = client
        .put("/api/v1/location/999999/mfa-flows")
        .json(&json!({"assignments": [{"flow_id": flow_id, "is_default": true, "group_ids": []}]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // The same flow twice would violate the (location_id, flow_id) primary key.
    let response = client
        .put(format!("/api/v1/location/{location_id}/mfa-flows"))
        .json(&json!({"assignments": [
            {"flow_id": flow_id, "is_default": true, "group_ids": []},
            {"flow_id": flow_id, "is_default": false, "group_ids": []},
        ]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json::<serde_json::Value>().await["fields"][0]["code"],
        "duplicate"
    );

    // A nonexistent flow would violate the foreign key.
    let response = client
        .put(format!("/api/v1/location/{location_id}/mfa-flows"))
        .json(&json!({"assignments": [
            {"flow_id": 999999, "is_default": true, "group_ids": []},
        ]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json::<serde_json::Value>().await["fields"][0]["code"],
        "unknown_flow"
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
/// was already present in the flow.  Adding email where it did not exist
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
}
