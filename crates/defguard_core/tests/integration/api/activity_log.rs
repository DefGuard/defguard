use chrono::{TimeDelta, Utc};
use defguard_common::db::{NoId, setup_pool};
use defguard_core::db::models::activity_log::{ActivityLogEvent, ActivityLogModule, EventType};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{get_db_user, make_client_with_db};

#[derive(Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct ApiActivityLogEvent {
    id: i64,
    ip: Option<String>,
    description: Option<String>,
}

#[sqlx::test]
async fn test_activity_log_persists_and_returns_null_ip(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, db) = make_client_with_db(pool.clone()).await;
    let admin = get_db_user(&db, "admin").await;

    client.login_user("admin", "pass123").await;

    let marker = format!(
        "nullable-ip-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let saved_event = ActivityLogEvent {
        id: NoId,
        timestamp: Utc::now().naive_utc() + TimeDelta::seconds(5),
        user_id: admin.id,
        username: admin.username,
        location: None,
        ip: None,
        event: EventType::UserLogout,
        module: ActivityLogModule::Defguard,
        device: "integration-test".to_string(),
        description: Some(marker.clone()),
        metadata: None,
    }
    .save(&db)
    .await
    .expect("activity log event with null ip should persist");

    let fetched_event = ActivityLogEvent::find_by_id(&db, saved_event.id)
        .await
        .expect("activity log event query should succeed")
        .expect("saved activity log event should exist");
    assert_eq!(fetched_event.ip, None);

    let response = client.get("/api/v1/activity_log").send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let payload: PaginatedResponse<ApiActivityLogEvent> = response.json().await;
    let api_event = payload
        .data
        .into_iter()
        .find(|event| event.id == saved_event.id)
        .expect("activity log endpoint should return saved event");

    assert_eq!(api_event.description.as_deref(), Some(marker.as_str()));
    assert_eq!(api_event.ip, None);
}
