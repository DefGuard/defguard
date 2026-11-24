use chrono::{NaiveDateTime, TimeDelta};
use defguard_common::db::{
    Id,
    models::{
        Device, DeviceType, User,
        device_verification::{DEVICE_VERIFICATION_CODE_PERIOD, DeviceVerification},
    },
};
use defguard_core::events::ApiEventType;
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
    query,
};

use super::common::{client::TestClient, make_test_client, set_enterprise_license, setup_pool};

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
    let (mut client, client_state) = make_test_client(pool).await;
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

    assert_eq!(client.delete(&url).send().await.status(), StatusCode::OK);
    assert!(
        DeviceVerification::find_active_for_device(pool, device.id)
            .await
            .unwrap()
            .is_none()
    );
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
}
