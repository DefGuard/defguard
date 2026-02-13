use std::fmt::Display;

use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chrono::{NaiveDateTime, Utc};
use defguard_common::db::{
    Id,
    models::{MFAMethod, User, gateway::Gateway, proxy::Proxy},
};
use defguard_mail::{
    Attachment, Mail,
    templates::{self, SessionContext, TemplateError, support_data_mail},
};
use reqwest::Url;
use serde_json::json;
use tokio::fs::read_to_string;

use super::{ApiResponse, ApiResult};
use crate::{
    PgPool,
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::models::enrollment::TokenError,
    error::WebError,
    server_config,
    support::dump_config,
};

static TEST_MAIL_SUBJECT: &str = "Defguard email test";
static SUPPORT_EMAIL_ADDRESS: &str = "support@defguard.net";

static SUPPORT_EMAIL_SUBJECT: &str = "Defguard: Support data";

static NEW_DEVICE_LOGIN_EMAIL_SUBJECT: &str = "Defguard: new device logged in to your account";

static EMAIL_MFA_ACTIVATION_EMAIL_SUBJECT: &str =
    "Defguard: Multi-Factor Authentication activation";

static GATEWAY_DISCONNECTED_SUBJECT: &str = "Defguard: Gateway disconnected";
static GATEWAY_RECONNECTED_SUBJECT: &str = "Defguard: Gateway reconnected";

pub(crate) static EMAIL_PASSWORD_RESET_START_SUBJECT: &str = "Defguard: Password reset";
pub(crate) static EMAIL_PASSWORD_RESET_SUCCESS_SUBJECT: &str = "Defguard: Password reset success";

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

pub async fn test_mail(
    _admin: AdminRole,
    session: SessionInfo,
    Json(data): Json<TestMail>,
) -> ApiResult {
    debug!(
        "User {} sending test mail to {}",
        session.user.username, data.to
    );

    let result = Mail::new(
        &data.to,
        TEST_MAIL_SUBJECT,
        templates::test_mail(Some(&session.session.into()))?,
    )
    .send()
    .await;

    let (to, subject) = (&data.to, TEST_MAIL_SUBJECT);
    match result {
        Ok(()) => {
            info!("User {} sent test mail to {to}", session.user.username);
            Ok(ApiResponse::with_status(StatusCode::OK))
        }
        Err(err) => Ok(internal_error(to, subject, &err)),
    }
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
            "network_id": g.network_id,
            "version": g.version.as_deref().unwrap_or("unknown"),
            "url": g.url,
            "has_certificate": g.has_certificate,
            "hostname": g.hostname,
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
        support_data_mail()?,
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
    gateway_name: Option<String>,
    network_name: String,
    gateway_adress: &str,
    pool: &PgPool,
) -> Result<(), WebError> {
    debug!("Sending gateway disconnected mail to all admin users");
    let admin_users = User::find_admins(pool).await?;
    let gateway_name = gateway_name.unwrap_or_default();
    for user in admin_users {
        Mail::new(
            &user.email,
            GATEWAY_DISCONNECTED_SUBJECT,
            templates::gateway_disconnected_mail(&gateway_name, gateway_adress, &network_name)?,
        )
        .send_and_forget();
    }

    Ok(())
}

pub async fn send_gateway_reconnected_email(
    gateway_name: Option<String>,
    network_name: String,
    gateway_adress: &str,
    pool: &PgPool,
) -> Result<(), WebError> {
    debug!("Sending gateway reconnect mail to all admin users");
    let admin_users = User::find_admins(pool).await?;
    let gateway_name = gateway_name.unwrap_or_default();
    for user in admin_users {
        Mail::new(
            &user.email,
            GATEWAY_RECONNECTED_SUBJECT,
            templates::gateway_reconnected_mail(&gateway_name, gateway_adress, &network_name)?,
        )
        .send_and_forget();
    }

    Ok(())
}

pub fn send_new_device_login_email(
    user_email: &str,
    session: &SessionContext,
    created: NaiveDateTime,
) -> Result<(), TemplateError> {
    debug!("User {user_email} new device login mail to {SUPPORT_EMAIL_ADDRESS}");

    Mail::new(
        user_email,
        NEW_DEVICE_LOGIN_EMAIL_SUBJECT,
        templates::new_device_login_mail(session, created)?,
    )
    .send_and_forget();

    Ok(())
}

pub fn send_new_device_ocid_login_email(
    user_email: &str,
    oauth2client_name: &str,
    session: &SessionContext,
) -> Result<(), TemplateError> {
    debug!("User {user_email} new device OCID login mail to {SUPPORT_EMAIL_ADDRESS}");

    Mail::new(
        user_email,
        format!("New login to {oauth2client_name} application with Defguard"),
        templates::new_device_ocid_login_mail(session, oauth2client_name)?,
    )
    .send_and_forget();

    Ok(())
}

pub fn send_mfa_configured_email(
    session: Option<&SessionContext>,
    user: &User<Id>,
    mfa_method: &MFAMethod,
) -> Result<(), TemplateError> {
    debug!("Sending MFA configured mail to {}", user.email);

    Mail::new(
        &user.email,
        format!("MFA method {mfa_method} has been activated on your account"),
        templates::mfa_configured_mail(session, mfa_method)?,
    )
    .send_and_forget();

    Ok(())
}

pub fn send_email_mfa_activation_email(
    user: &User<Id>,
    session: Option<&SessionContext>,
) -> Result<(), TemplateError> {
    debug!("Sending email MFA activation mail to {}", user.email);

    // generate a verification code
    let code = user.generate_email_mfa_code().map_err(|err| {
        error!("Failed to generate email MFA code: {err}");
        TemplateError::MfaError
    })?;

    Mail::new(
        &user.email,
        EMAIL_MFA_ACTIVATION_EMAIL_SUBJECT,
        templates::email_mfa_activation_mail(&user.into(), &code, session)?,
    )
    .send_and_forget();

    Ok(())
}

pub fn send_password_reset_email(
    user: &User<Id>,
    service_url: Url,
    token: &str,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TokenError> {
    debug!("Sending password reset email to {}", user.email);

    Mail::new(
        &user.email,
        EMAIL_PASSWORD_RESET_START_SUBJECT,
        templates::email_password_reset_mail(service_url, token, ip_address, device_info)?,
    )
    .send_and_forget();

    Ok(())
}

pub fn send_password_reset_success_email(
    user: &User<Id>,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TokenError> {
    debug!("Sending password reset success email to {}", user.email);

    Mail::new(
        &user.email,
        EMAIL_PASSWORD_RESET_SUCCESS_SUBJECT,
        templates::email_password_reset_success_mail(ip_address, device_info)?,
    )
    .send_and_forget();

    Ok(())
}
