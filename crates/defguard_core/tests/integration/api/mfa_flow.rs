use defguard_common::db::setup_pool;
use defguard_core::enterprise::license::{get_cached_license, set_cached_license};
use reqwest::StatusCode;
use serde_json::json;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{authenticate_admin, make_network, make_test_client, set_enterprise_license};

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
            { "methods": ["email"] }
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

    set_cached_license(saved);
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
                "steps": [{ "methods": ["email"] }]
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
                { "methods": ["email"] }
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
                { "methods": ["email"] }
            ]
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    set_cached_license(saved);
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
