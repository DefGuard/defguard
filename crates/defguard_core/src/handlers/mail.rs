use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chrono::Utc;
use defguard_common::db::models::{User, gateway::Gateway, proxy::Proxy};
use serde_json::json;
use sqlx::query_scalar;
use tera::Context;
use thiserror::Error;
use tokio::fs::read_to_string;
use utoipa::ToSchema;

use super::{ApiErrorResponse, ApiResponse, ApiResult};
use crate::{
    PgPool,
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    mail::{
        Attachment,
        templates::{self, SUPPORT_EMAIL_ADDRESS},
    },
    server_config,
    support::dump_config,
};

#[derive(Clone, Deserialize, ToSchema)]
pub struct TestMail {
    pub to: String,
}

/// Send a test email to verify the SMTP configuration
#[utoipa::path(
    post,
    path = "/api/v1/mail/test",
    tag = "support",
    request_body = TestMail,
    responses(
        (status = 200, description = "Test email sent."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to send test email.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
        (status = 503, description = "SMTP is not configured.", body = ApiErrorResponse, example = json!({"msg": "SMTP is not configured", "code": "smtp_not_configured"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
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
    let result = templates::test_mail(&data.to, &mut conn, Some(&session.session.into())).await;

    Ok(match result {
        Ok(()) => {
            info!(
                "User {} sent test mail to {}",
                session.user.username, data.to
            );
            ApiResponse::with_status(StatusCode::OK)
        }
        Err(err) => {
            error!(
                "User {} failed to send test mail to {}: {err}",
                session.user.username, data.to
            );
            ApiResponse::with_status(StatusCode::SERVICE_UNAVAILABLE)
        }
    })
}

async fn read_logs() -> String {
    let Some(path) = &server_config().log_file else {
        return "Log file not configured".to_owned();
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

/// Send the support data bundle to the defguard support address
#[utoipa::path(
    post,
    path = "/api/v1/mail/support",
    tag = "support",
    responses(
        (status = 200, description = "Support data sent."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to send support data.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
        (status = 503, description = "SMTP is not configured.", body = ApiErrorResponse, example = json!({"msg": "SMTP is not configured", "code": "smtp_not_configured"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn send_support_data(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} sending support mail", session.user.username);

    let mut conn = appstate.pool.begin().await?;
    let proxies = Proxy::all(&mut *conn).await?;
    let gateways = Gateway::all(&mut *conn).await?;

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
            "certificate": g.certificate_serial,
            "name": g.name,
            "connected_at": g.connected_at,
        })).collect::<Vec<_>>(),
    });
    let now = Utc::now();
    let components_json =
        serde_json::to_vec(&components_info).unwrap_or(b"JSON formatting error".into());
    let components = Attachment::new(format!("defguard-components-{now}.json"), components_json);
    let config = dump_config(&mut conn)
        .await
        .unwrap_or(json!({"err": "Failed to dump configuration"}));
    let config = serde_json::to_vec_pretty(&config).unwrap_or(b"JSON formatting error".into());
    let config = Attachment::new(format!("defguard-support-data-{now}.json"), config);
    let logs = read_logs().await;
    let logs = Attachment::new(format!("defguard-logs-{now}.txt"), logs.into());

    let result = templates::support_data_mail(
        SUPPORT_EMAIL_ADDRESS,
        &mut conn,
        vec![components, config, logs],
    )
    .await;
    Ok(match result {
        Ok(()) => {
            info!("User {} sent support mail", session.user.username);
            ApiResponse::with_status(StatusCode::OK)
        }
        Err(err) => {
            error!("Error sending support mail: {err}");
            ApiResponse::new(
                json!({"error": err.to_string()}),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    })
}

/// Errors arising from automated mail operations.
#[derive(Debug, Error)]
pub enum MailError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Template error: {0}")]
    Template(#[from] crate::mail::templates::TemplateError),
}

pub async fn send_gateway_disconnected_email(
    gateway_name: String,
    network_name: String,
    gateway_adress: &str,
    pool: &PgPool,
) -> Result<(), MailError> {
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
) -> Result<(), MailError> {
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

pub async fn send_user_import_blocked_email(pool: &PgPool) -> Result<(), MailError> {
    debug!("Sending blocked user import mail to all admin users");
    let admin_emails = get_admins_emails(pool).await?;
    let mut conn = pool.acquire().await?;

    for email in admin_emails {
        templates::user_import_blocked_mail(&email, &mut conn, Context::new()).await?;
        debug!("Scheduled blocked user import mail to admin {}", email);
    }

    Ok(())
}
