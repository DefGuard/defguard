use chrono::{NaiveDateTime, TimeDelta};
use defguard_common::{
    db::{
        Id,
        models::{
            Device, DeviceState, DeviceType, User,
            device::DeviceVerificationStatus,
            device_verification::{
                DEVICE_VERIFICATION_CODE_PERIOD, DeviceVerification, DeviceVerificationOutcome,
            },
        },
    },
    types::proxy::ProxyControlMessage,
};
use defguard_core::events::ApiEventType;
use reqwest::StatusCode;
use serde_json::{Value, json};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
    query,
};

use tokio::sync::mpsc;

use super::common::{client::TestClient, make_test_client, set_enterprise_license, setup_pool};

fn expect_resolved_broadcast(
    proxy_control_rx: &mut mpsc::Receiver<ProxyControlMessage>,
    session_id: Id,
    outcome: DeviceVerificationOutcome,
) {
    match proxy_control_rx.try_recv() {
        Ok(ProxyControlMessage::BroadcastDeviceVerificationResolved(verification)) => {
            assert_eq!(verification.id, session_id);
            assert_eq!(verification.outcome, Some(outcome));
        }
        Ok(_) => panic!("unexpected proxy control message"),
        Err(err) => panic!("no device verification broadcast: {err}"),
    }
}

async fn make_device(
    pool: &PgPool,
    name: &str,
    user_id: Id,
    device_type: DeviceType,
) -> Device<Id> {
    Device::new(
        name.to_owned(),
        format!("{name}-pubkey"),
        user_id,
        device_type,
        None,
        true,
    )
    .save(pool)
    .await
    .unwrap()
}

async fn expire(pool: &PgPool, device_id: Id) {
    query!(
        "UPDATE device_verification SET expires_at = (now() AT TIME ZONE 'UTC') - interval '1 second' \
        WHERE device_id = $1",
        device_id
    )
    .execute(pool)
    .await
    .unwrap();
}

fn verification_url(device_id: Id) -> String {
    format!("/api/v1/device/{device_id}/verification")
}

fn timestamp(body: &Value, field: &str) -> NaiveDateTime {
    serde_json::from_value(body[field].clone()).unwrap()
}

async fn assert_status_for_both(client: &TestClient, device_id: Id, status: StatusCode) {
    let url = verification_url(device_id);
    assert_eq!(client.post(&url).send().await.status(), status);
    assert_eq!(client.delete(&url).send().await.status(), status);
}

#[sqlx::test]
async fn test_device_verification_access(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    let pool = &client_state.pool;
    let user_id = client_state.test_user.id;
    let device = make_device(pool, "laptop", user_id, DeviceType::User).await;
    let network_device = make_device(pool, "router", user_id, DeviceType::Network).await;

    assert_status_for_both(&client, device.id, StatusCode::UNAUTHORIZED).await;

    client.login_user("admin", "pass123").await;
    assert_status_for_both(&client, device.id, StatusCode::FORBIDDEN).await;

    set_enterprise_license();
    client.login_user("hpotter", "pass123").await;
    assert_status_for_both(&client, device.id, StatusCode::FORBIDDEN).await;

    client.login_user("admin", "pass123").await;
    assert_status_for_both(&client, 9999, StatusCode::NOT_FOUND).await;
    assert_status_for_both(&client, network_device.id, StatusCode::BAD_REQUEST).await;

    assert!(
        DeviceVerification::find_active_for_device(pool, device.id)
            .await
            .unwrap()
            .is_none()
    );
    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_start_device_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    set_enterprise_license();
    let pool = &client_state.pool;
    let owner = &client_state.test_user;
    let device = make_device(pool, "laptop", owner.id, DeviceType::User).await;
    let admin = User::find_by_username(pool, "admin")
        .await
        .unwrap()
        .unwrap();
    client.login_user("admin", "pass123").await;
    let url = verification_url(device.id);

    let response = client.post(&url).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    assert_eq!(body["device_id"], device.id);
    let code = body["code"].as_str().unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));
    let code_expires_in = body["code_expires_in"].as_u64().unwrap();
    assert!((1..=DEVICE_VERIFICATION_CODE_PERIOD).contains(&code_expires_in));
    assert_eq!(body["code_period"], DEVICE_VERIFICATION_CODE_PERIOD);
    let created_at = timestamp(&body, "created_at");
    let expires_at = timestamp(&body, "expires_at");
    assert_eq!(expires_at - created_at, TimeDelta::hours(1));

    let verification = DeviceVerification::find_active_for_device(pool, device.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(verification.started_by, Some(admin.id));
    let events = client.drain_all_events();
    assert_eq!(events.len(), 1);
    match &events[0] {
        (
            ApiEventType::UserDeviceVerificationStarted {
                owner: o,
                device: d,
            },
            user_id,
            _,
        ) => {
            assert_eq!(o.id, owner.id);
            assert_eq!(d.id, device.id);
            assert_eq!(*user_id, Some(admin.id));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let response = client.post(&url).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    assert_eq!(timestamp(&body, "created_at"), created_at);
    assert_eq!(timestamp(&body, "expires_at"), expires_at);
    client.assert_event_queue_is_empty();

    expire(pool, device.id).await;
    let response = client.post(&url).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    assert!(timestamp(&body, "created_at") > created_at);
    let events = client.drain_all_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].0,
        ApiEventType::UserDeviceVerificationStarted { .. }
    ));
}

#[sqlx::test]
async fn test_cancel_device_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, mut client_state) = make_test_client(pool).await;
    set_enterprise_license();
    let pool = &client_state.pool;
    let owner = &client_state.test_user;
    let device = make_device(pool, "laptop", owner.id, DeviceType::User).await;
    client.login_user("admin", "pass123").await;
    let url = verification_url(device.id);

    assert_eq!(
        client.delete(&url).send().await.status(),
        StatusCode::NOT_FOUND
    );
    client.assert_event_queue_is_empty();

    assert_eq!(client.post(&url).send().await.status(), StatusCode::OK);
    client.drain_all_events();
    let session = DeviceVerification::find_active_for_device(pool, device.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(client.delete(&url).send().await.status(), StatusCode::OK);
    expect_resolved_broadcast(
        &mut client_state.proxy_control_rx,
        session.id,
        DeviceVerificationOutcome::Cancelled,
    );
    assert!(
        DeviceVerification::find_active_for_device(pool, device.id)
            .await
            .unwrap()
            .is_none()
    );
    let resolved = DeviceVerification::find_for_device(pool, session.id, device.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.outcome, Some(DeviceVerificationOutcome::Cancelled));
    let events = client.drain_all_events();
    assert_eq!(events.len(), 1);
    match &events[0].0 {
        ApiEventType::UserDeviceVerificationCancelled {
            owner: o,
            device: d,
        } => {
            assert_eq!(o.id, owner.id);
            assert_eq!(d.id, device.id);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    assert_eq!(
        client.delete(&url).send().await.status(),
        StatusCode::NOT_FOUND
    );
    client.assert_event_queue_is_empty();

    assert_eq!(client.post(&url).send().await.status(), StatusCode::OK);
    let events = client.drain_all_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].0,
        ApiEventType::UserDeviceVerificationStarted { .. }
    ));

    expire(pool, device.id).await;
    assert_eq!(
        client.delete(&url).send().await.status(),
        StatusCode::NOT_FOUND
    );
    client.assert_event_queue_is_empty();
    assert!(
        client_state.proxy_control_rx.try_recv().is_err(),
        "a cancel that resolves nothing must not broadcast"
    );
}

#[sqlx::test]
async fn test_bulk_actions_resolve_device_verification(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, mut client_state) = make_test_client(pool).await;
    set_enterprise_license();
    let pool = &client_state.pool;
    let user_id = client_state.test_user.id;
    client.login_user("admin", "pass123").await;

    for (action, outcome) in [
        ("verify", DeviceVerificationOutcome::Verified),
        ("block", DeviceVerificationOutcome::Blocked),
    ] {
        let device = make_device(pool, action, user_id, DeviceType::User).await;
        assert_eq!(
            client
                .post(verification_url(device.id))
                .send()
                .await
                .status(),
            StatusCode::OK
        );
        client.drain_all_events();
        let session = DeviceVerification::find_active_for_device(pool, device.id)
            .await
            .unwrap()
            .unwrap();

        let response = client
            .post(format!("/api/v1/device/bulk/{action}"))
            .json(&json!({"devices": [device.id]}))
            .send()
            .await;
        assert_eq!(response.status(), StatusCode::OK, "action: {action}");

        let row = DeviceVerification::find_active_for_device(pool, device.id)
            .await
            .unwrap();
        assert!(row.is_none(), "action: {action}");
        expect_resolved_broadcast(&mut client_state.proxy_control_rx, session.id, outcome);
        let resolved = DeviceVerification::find_for_device(pool, session.id, device.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resolved.outcome, Some(outcome), "action: {action}");
        let events = client.drain_all_events();
        assert_eq!(events.len(), 1, "action: {action}");
        assert!(
            !matches!(
                events[0].0,
                ApiEventType::UserDeviceVerificationCancelled { .. }
            ),
            "action: {action}"
        );
    }
}

#[sqlx::test]
async fn test_verify_resolves_session_of_verified_device(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    set_enterprise_license();
    let pool = &client_state.pool;
    let user_id = client_state.test_user.id;
    client.login_user("admin", "pass123").await;
    let device = make_device(pool, "laptop", user_id, DeviceType::User).await;
    let response = client
        .post("/api/v1/device/bulk/verify")
        .json(&json!({"devices": [device.id]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        client
            .post(verification_url(device.id))
            .send()
            .await
            .status(),
        StatusCode::OK
    );
    let session = DeviceVerification::find_active_for_device(pool, device.id)
        .await
        .unwrap()
        .unwrap();

    let response = client
        .post("/api/v1/device/bulk/verify")
        .json(&json!({"devices": [device.id]}))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let resolved = DeviceVerification::find_for_device(pool, session.id, device.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.outcome, Some(DeviceVerificationOutcome::Verified));
}

#[sqlx::test]
async fn test_get_device_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    let pool = &client_state.pool;
    let user_id = client_state.test_user.id;
    let device = make_device(pool, "laptop", user_id, DeviceType::User).await;
    let other_device = make_device(pool, "phone", user_id, DeviceType::User).await;
    let session_url =
        |device_id: Id, session_id: Id| format!("{}/{session_id}", verification_url(device_id));

    client.login_user("admin", "pass123").await;
    assert_eq!(
        client.get(session_url(device.id, 1)).send().await.status(),
        StatusCode::FORBIDDEN
    );
    set_enterprise_license();

    let response = client.post(verification_url(device.id)).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let started: Value = response.json().await;
    let session_id = started["session_id"].as_i64().unwrap();
    client.drain_all_events();

    let response = client.get(session_url(device.id, session_id)).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    assert_eq!(body["session_id"], session_id);
    assert_eq!(body["device_id"], device.id);
    assert_eq!(body["expires_at"], started["expires_at"]);
    assert_eq!(body["code"].as_str().unwrap().len(), 6);
    assert_eq!(
        client
            .get(session_url(other_device.id, session_id))
            .send()
            .await
            .status(),
        StatusCode::NOT_FOUND,
        "a session of another device"
    );

    assert_eq!(
        client
            .delete(verification_url(device.id))
            .send()
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .get(session_url(device.id, session_id))
            .send()
            .await
            .status(),
        StatusCode::NOT_FOUND,
        "a resolved session"
    );

    let response = client.post(verification_url(device.id)).send().await;
    let session_id = response.json::<Value>().await["session_id"]
        .as_i64()
        .unwrap();
    expire(pool, device.id).await;
    assert_eq!(
        client
            .get(session_url(device.id, session_id))
            .send()
            .await
            .status(),
        StatusCode::NOT_FOUND,
        "an expired session"
    );
    assert!(
        DeviceVerification::find_for_device(pool, session_id, device.id)
            .await
            .unwrap()
            .is_some(),
        "reading must not replace the session"
    );
}

#[sqlx::test]
async fn test_start_device_verification_of_blocked_device(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    set_enterprise_license();
    let pool = &client_state.pool;
    let user_id = client_state.test_user.id;
    let mut device = make_device(pool, "laptop", user_id, DeviceType::User).await;
    device.state = DeviceState::Blocked;
    device.save(pool).await.unwrap();
    client.login_user("admin", "pass123").await;

    assert_eq!(
        client
            .post(verification_url(device.id))
            .send()
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert!(
        DeviceVerification::find_active_for_device(pool, device.id)
            .await
            .unwrap()
            .is_none()
    );
    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_overview_reports_pending_verification(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, client_state) = make_test_client(pool).await;
    set_enterprise_license();
    let pool = &client_state.pool;
    let user_id = client_state.test_user.id;
    let pending = make_device(pool, "pending", user_id, DeviceType::User).await;
    let unverified = make_device(pool, "unverified", user_id, DeviceType::User).await;
    let mut verified = make_device(pool, "verified", user_id, DeviceType::User).await;
    verified.verification = DeviceVerificationStatus::Verified;
    verified.save(pool).await.unwrap();
    let expired = make_device(pool, "expired", user_id, DeviceType::User).await;
    client.login_user("admin", "pass123").await;
    for device in [&pending, &verified, &expired] {
        assert_eq!(
            client
                .post(verification_url(device.id))
                .send()
                .await
                .status(),
            StatusCode::OK
        );
    }
    expire(pool, expired.id).await;

    let response = client.get("/api/v1/device/overview").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await;
    let status_of = |id: Id| {
        body["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == id)
            .unwrap()["verification"]
            .clone()
    };
    assert_eq!(status_of(pending.id), "pending");
    assert_eq!(status_of(unverified.id), "unverified");
    assert_eq!(status_of(verified.id), "verified");
    assert_eq!(status_of(expired.id), "unverified");

    for (filter, expected) in [
        ("pending", vec![pending.id]),
        ("unverified", vec![expired.id, unverified.id]),
    ] {
        let response = client
            .get(format!("/api/v1/device/overview?verification={filter}"))
            .send()
            .await;
        let body: Value = response.json().await;
        let ids: Vec<Id> = body["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_i64().unwrap())
            .collect();
        assert_eq!(ids, expected, "filter: {filter}");
        assert_eq!(
            body["pagination"]["total_items"],
            expected.len(),
            "filter: {filter}"
        );
    }
}
