use std::collections::HashSet;

use chrono::{NaiveDateTime, TimeDelta, Timelike, Utc};
use defguard_common::db::{Id, NoId, models::User, setup_pool};
use defguard_core::db::models::activity_log::{ActivityLogEvent, ActivityLogModule, EventType};
use reqwest::StatusCode;
use serde::Deserialize;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{client::TestClient, get_db_user, make_client_with_db};

#[derive(Deserialize)]
struct PaginatedResponse<T> {
    data: Vec<T>,
    pagination: PaginationMeta,
}

#[derive(Deserialize)]
struct PaginationMeta {
    next_page: Option<u32>,
    total_items: u32,
}

#[derive(Clone, Deserialize)]
struct ApiActivityLogEvent {
    id: i64,
    timestamp: NaiveDateTime,
    username: String,
    ip: Option<String>,
    description: Option<String>,
}

fn unique_marker(prefix: &str) -> String {
    format!(
        "{prefix}-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

fn activity_log_url(marker: &str, extra_query: &str) -> String {
    if extra_query.is_empty() {
        return format!("/api/v1/activity_log?search={marker}");
    }

    format!("/api/v1/activity_log?search={marker}&{extra_query}")
}

async fn fetch_activity_log(
    client: &TestClient,
    marker: &str,
    extra_query: &str,
) -> PaginatedResponse<ApiActivityLogEvent> {
    let response = client
        .get(activity_log_url(marker, extra_query))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await
}

async fn save_activity_log_event(
    db: &sqlx::PgPool,
    user: &User<Id>,
    marker: &str,
    description_suffix: &str,
    timestamp: NaiveDateTime,
) -> ActivityLogEvent<Id> {
    let persisted_timestamp = truncate_timestamp_to_microseconds(timestamp);

    ActivityLogEvent {
        id: NoId,
        timestamp: persisted_timestamp,
        user_id: user.id,
        username: user.username.clone(),
        location: None,
        ip: None,
        event: EventType::UserLogout,
        module: ActivityLogModule::Defguard,
        device: "integration-test".to_string(),
        description: Some(format!("{marker}-{description_suffix}")),
        metadata: None,
    }
    .save(db)
    .await
    .expect("activity log event should persist")
}

fn truncate_timestamp_to_microseconds(timestamp: NaiveDateTime) -> NaiveDateTime {
    timestamp
        .with_nanosecond((timestamp.nanosecond() / 1_000) * 1_000)
        .expect("microsecond timestamp truncation should produce a valid NaiveDateTime")
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

    let marker = unique_marker("nullable-ip");
    let saved_event = save_activity_log_event(
        &db,
        &admin,
        &marker,
        "saved",
        Utc::now().naive_utc() + TimeDelta::seconds(5),
    )
    .await;

    let fetched_event = ActivityLogEvent::find_by_id(&db, saved_event.id)
        .await
        .expect("activity log event query should succeed")
        .expect("saved activity log event should exist");
    assert_eq!(fetched_event.ip, None);

    let payload = fetch_activity_log(&client, &marker, "").await;
    let expected_description = format!("{marker}-saved");
    let api_event = payload
        .data
        .into_iter()
        .find(|event| event.id == saved_event.id)
        .expect("activity log endpoint should return saved event");

    assert_eq!(
        api_event.description.as_deref(),
        Some(expected_description.as_str())
    );
    assert_eq!(api_event.ip, None);
}

#[sqlx::test]
async fn test_activity_log_timestamp_desc_uses_id_desc_for_equal_timestamps(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, db) = make_client_with_db(pool.clone()).await;
    let admin = get_db_user(&db, "admin").await;

    client.login_user("admin", "pass123").await;

    let marker = unique_marker("equal-timestamp-desc");
    let shared_timestamp = Utc::now().naive_utc() + TimeDelta::seconds(5);
    let first_event =
        save_activity_log_event(&db, &admin, &marker, "first", shared_timestamp).await;
    let second_event =
        save_activity_log_event(&db, &admin, &marker, "second", shared_timestamp).await;
    let third_event =
        save_activity_log_event(&db, &admin, &marker, "third", shared_timestamp).await;

    let payload = fetch_activity_log(&client, &marker, "sort_by=timestamp&sort_order=desc").await;
    let ids: Vec<i64> = payload.data.into_iter().map(|event| event.id).collect();

    assert_eq!(
        ids,
        vec![third_event.id, second_event.id, first_event.id],
        "equal timestamps should fall back to descending ids",
    );
}

#[sqlx::test]
async fn test_activity_log_timestamp_desc_orders_by_timestamp_then_id(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, db) = make_client_with_db(pool.clone()).await;
    let admin = get_db_user(&db, "admin").await;

    client.login_user("admin", "pass123").await;

    let marker = unique_marker("near-simultaneous-desc");
    let base_timestamp = Utc::now().naive_utc() + TimeDelta::seconds(5);
    let earlier_event =
        save_activity_log_event(&db, &admin, &marker, "earlier", base_timestamp).await;
    let later_same_time_first = save_activity_log_event(
        &db,
        &admin,
        &marker,
        "later-first",
        base_timestamp + TimeDelta::milliseconds(1),
    )
    .await;
    let later_same_time_second = save_activity_log_event(
        &db,
        &admin,
        &marker,
        "later-second",
        base_timestamp + TimeDelta::milliseconds(1),
    )
    .await;

    let payload = fetch_activity_log(&client, &marker, "sort_by=timestamp&sort_order=desc").await;
    let ordered_events: Vec<(i64, NaiveDateTime)> = payload
        .data
        .into_iter()
        .map(|event| (event.id, event.timestamp))
        .collect();

    assert_eq!(
        ordered_events,
        vec![
            (later_same_time_second.id, later_same_time_second.timestamp),
            (later_same_time_first.id, later_same_time_first.timestamp),
            (earlier_event.id, earlier_event.timestamp),
        ],
        "descending timestamp sort should use timestamp first and id as the stable fallback",
    );
}

#[sqlx::test]
async fn test_activity_log_timestamp_asc_uses_id_asc_for_equal_timestamps(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, db) = make_client_with_db(pool.clone()).await;
    let admin = get_db_user(&db, "admin").await;

    client.login_user("admin", "pass123").await;

    let marker = unique_marker("timestamp-asc");
    let base_timestamp = Utc::now().naive_utc() + TimeDelta::seconds(5);
    let first_event = save_activity_log_event(&db, &admin, &marker, "first", base_timestamp).await;
    let second_event =
        save_activity_log_event(&db, &admin, &marker, "second", base_timestamp).await;
    let later_event = save_activity_log_event(
        &db,
        &admin,
        &marker,
        "later",
        base_timestamp + TimeDelta::milliseconds(1),
    )
    .await;

    let payload = fetch_activity_log(&client, &marker, "sort_by=timestamp&sort_order=asc").await;
    let ids: Vec<i64> = payload.data.into_iter().map(|event| event.id).collect();

    assert_eq!(
        ids,
        vec![first_event.id, second_event.id, later_event.id],
        "ascending timestamp sort should use ascending ids for equal timestamps",
    );
}

#[sqlx::test]
async fn test_activity_log_non_timestamp_sort_uses_id_as_stable_tiebreaker(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, db) = make_client_with_db(pool.clone()).await;
    let admin = get_db_user(&db, "admin").await;
    let hpotter = get_db_user(&db, "hpotter").await;

    client.login_user("admin", "pass123").await;

    let marker = unique_marker("username-sort");
    let shared_timestamp = Utc::now().naive_utc() + TimeDelta::seconds(5);
    let admin_first =
        save_activity_log_event(&db, &admin, &marker, "admin-first", shared_timestamp).await;
    let hpotter_event =
        save_activity_log_event(&db, &hpotter, &marker, "hpotter", shared_timestamp).await;
    let admin_second =
        save_activity_log_event(&db, &admin, &marker, "admin-second", shared_timestamp).await;

    let payload = fetch_activity_log(&client, &marker, "sort_by=username&sort_order=asc").await;
    let ordered_events: Vec<(String, i64)> = payload
        .data
        .into_iter()
        .map(|event| (event.username, event.id))
        .collect();

    assert_eq!(
        ordered_events,
        vec![
            (admin.username.clone(), admin_first.id),
            (admin.username.clone(), admin_second.id),
            (hpotter.username.clone(), hpotter_event.id),
        ],
        "non-timestamp sorts should use the same direction for the id tiebreaker",
    );
}

#[sqlx::test]
async fn test_activity_log_pagination_is_stable_across_pages_for_equal_timestamps(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, db) = make_client_with_db(pool.clone()).await;
    let admin = get_db_user(&db, "admin").await;

    client.login_user("admin", "pass123").await;

    let marker = unique_marker("pagination-boundary");
    let shared_timestamp = Utc::now().naive_utc() + TimeDelta::seconds(5);
    let mut inserted_ids = Vec::new();

    for index in 0..55 {
        let event = save_activity_log_event(
            &db,
            &admin,
            &marker,
            &format!("event-{index}"),
            shared_timestamp,
        )
        .await;
        inserted_ids.push(event.id);
    }

    let page_one =
        fetch_activity_log(&client, &marker, "sort_by=timestamp&sort_order=desc&page=1").await;
    let page_two =
        fetch_activity_log(&client, &marker, "sort_by=timestamp&sort_order=desc&page=2").await;

    assert_eq!(
        page_one.data.len(),
        50,
        "first page should stop at the page size boundary"
    );
    assert_eq!(
        page_two.data.len(),
        5,
        "second page should contain the remaining events"
    );
    assert_eq!(page_one.pagination.total_items, 55);
    assert_eq!(page_one.pagination.next_page, Some(2));
    assert_eq!(page_two.pagination.next_page, None);

    let combined_ids: Vec<i64> = page_one
        .data
        .iter()
        .chain(page_two.data.iter())
        .map(|event| event.id)
        .collect();
    let unique_ids: HashSet<i64> = combined_ids.iter().copied().collect();
    let expected_ids: Vec<i64> = inserted_ids.into_iter().rev().collect();

    assert_eq!(
        combined_ids, expected_ids,
        "pagination should preserve the stable global order"
    );
    assert_eq!(
        unique_ids.len(),
        combined_ids.len(),
        "pagination should not duplicate events across pages"
    );
}
