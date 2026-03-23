use std::fmt::Display;

use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chrono::Utc;
use defguard_common::db::models::{User, gateway::Gateway, proxy::Proxy};
use defguard_mail::{Attachment, Mail, templates};
use serde_json::json;
use sqlx::query_scalar;
use tera::Context;
use tokio::fs::read_to_string;

use super::{ApiResponse, ApiResult};
use crate::{
    PgPool,
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    server_config,
    support::dump_config,
};

static SUPPORT_EMAIL_ADDRESS: &str = "support@defguard.net";
static SUPPORT_EMAIL_SUBJECT: &str = "Defguard: Support data";

#[derive(Clone, Deserialize)]
pub struct TestMail {
    pub to: String,
}

/// Handles logging the error and returns ApiResponse that contains it
fn internal_error(to: &str, subject: &str, error: impl Display) -> ApiResponse {
    error!("Error sending mail to {to}, subject: {subject}, error: {error}");
    ApiResponse::new(
        json!({"error": error.to_string()}),
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

pub(crate) async fn test_mail(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<TestMail>,
) -> ApiResult {
    debug!(
        "User {} sending test mail to {}",
        session.user.username, data.to
    );

    let mut conn = appstate.pool.begin().await?;
    templates::test_mail(&data.to, &mut conn, Some(&session.session.into())).await?;

    info!(
        "User {} sent test mail to {}",
        session.user.username, data.to
    );

    Ok(ApiResponse::with_status(StatusCode::OK))
}

async fn read_logs() -> String {
    let Some(path) = &server_config().log_file else {
        return "Log file not configured".to_string();
    };

    match read_to_string(path).await {
        Ok(logs) => logs,
        Err(err) => {
            let msg = format!("Error dumping app logs: {err}");
            error!(msg);
            msg
        }
    }
}

pub async fn send_support_data(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} sending support mail to {SUPPORT_EMAIL_ADDRESS}",
        session.user.username
    );

    let proxies = Proxy::all(&appstate.pool).await?;
    let gateways = Gateway::all(&appstate.pool).await?;

    let components_info = json!({
        "proxies": proxies.iter().map(|p| json!({
            "id": p.id,
            "name": p.name,
            "version": p.version.as_deref().unwrap_or("unknown"),
            "address": p.address,
            "connected_at": p.connected_at
        })).collect::<Vec<_>>(),
        "gateways": gateways.iter().map(|g| json!({
            "id": g.id,
            "network_id": g.location_id,
            "version": g.version.as_deref().unwrap_or("unknown"),
            "address": g.address,
            "port": g.port,
            "certificate": g.certificate,
            "name": g.name,
            "connected_at": g.connected_at,
        })).collect::<Vec<_>>(),
    });

    let components_json =
        serde_json::to_vec(&components_info).unwrap_or(b"JSON formatting error".into());

    let components = Attachment::new(
        format!("defguard-components-{}.json", Utc::now()),
        components_json,
    );

    let config = dump_config(&appstate.pool).await;
    let config = serde_json::to_vec_pretty(&config).unwrap_or(b"JSON formatting error".into());
    let config = Attachment::new(format!("defguard-support-data-{}.json", Utc::now()), config);
    let logs = read_logs().await;
    let logs = Attachment::new(format!("defguard-logs-{}.txt", Utc::now()), logs.into());
    let result = Mail::new(
        SUPPORT_EMAIL_ADDRESS,
        SUPPORT_EMAIL_SUBJECT,
        templates::support_data_mail()?,
    )
    .set_attachments(vec![components, config, logs])
    .send()
    .await;

    let (to, subject) = (SUPPORT_EMAIL_ADDRESS, SUPPORT_EMAIL_SUBJECT);
    match result {
        Ok(()) => {
            info!(
                "User {} sent support mail to {SUPPORT_EMAIL_ADDRESS}",
                session.user.username
            );
            Ok(ApiResponse::with_status(StatusCode::OK))
        }
        Err(err) => Ok(internal_error(to, subject, &err)),
    }
}

pub async fn send_gateway_disconnected_email(
    gateway_name: String,
    network_name: String,
    gateway_adress: &str,
    pool: &PgPool,
) -> Result<(), WebError> {
    debug!("Sending Gateway disconnected mail to all admin users");
    let mut conn = pool.begin().await?;
    let admin_users = User::find_admins(&mut *conn).await?;
    for user in admin_users {
        templates::gateway_disconnected_mail(
            &user.email,
            &mut conn,
            &gateway_name,
            gateway_adress,
            &network_name,
        )
        .await?;
    }

    Ok(())
}

pub async fn send_gateway_reconnected_email(
    gateway_name: String,
    network_name: String,
    gateway_adress: &str,
    pool: &PgPool,
) -> Result<(), WebError> {
    debug!("Sending Gateway reconnect mail to all admin users");
    let mut conn = pool.begin().await?;
    let admin_users = User::find_admins(&mut *conn).await?;
    for user in admin_users {
        templates::gateway_reconnected_mail(
            &user.email,
            &mut conn,
            &gateway_name,
            gateway_adress,
            &network_name,
        )
        .await?;
    }

    Ok(())
}

pub async fn get_admins_emails(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    debug!("Getting emails of active admins");
    query_scalar::<_, String>(
        "SELECT u.email \
            FROM \"user\" u \
            JOIN group_user gu oN gu.user_id = u.id JOIN \"group\" g ON gu.group_id = g.id \
            WHERE g.is_admin AND u.is_active",
    )
    .fetch_all(pool)
    .await
}

pub async fn send_user_import_blocked_email(pool: &PgPool) -> Result<(), WebError> {
    debug!("Sending blocked user import mail to all admin users");
    let admin_emails = get_admins_emails(pool).await?;
    let mut conn = pool.acquire().await?;

    for email in admin_emails {
        templates::user_import_blocked_mail(&email, &mut conn, Context::new()).await?;
        debug!("Scheduled blocked user import mail to admin {}", email);
    }

    Ok(())
}
