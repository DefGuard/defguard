use std::fmt::Display;

use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chrono::Utc;
use lettre::message::header::ContentType;
use serde_json::json;
use tokio::{
    fs::read_to_string,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    config::DefGuardConfig,
    db::{models::enrollment::EnrollmentError, Device, User},
    mail::{Attachment, Mail},
    support::dump_config,
    templates::{self, support_data_mail},
};

static TEST_MAIL_SUBJECT: &str = "Defguard email test";
static SUPPORT_EMAIL_ADDRESS: &str = "support@defguard.net";
static SUPPORT_EMAIL_SUBJECT: &str = "Defguard support data";

static NEW_DEVICE_ADDED_EMAIL_SUBJECT: &str = "Defguard: new device added to your account";

#[derive(Clone, Deserialize)]
pub struct TestMail {
    pub to: String,
}

/// Handles logging the error and returns ApiResponse that contains it
fn internal_error(to: &str, subject: &str, error: &impl Display) -> ApiResponse {
    error!("Error sending mail to {to}, subject: {subject}, error: {error}");
    ApiResponse {
        json: json!({
            "error": error.to_string(),
        }),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn test_mail(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<TestMail>,
) -> ApiResult {
    debug!(
        "User {} sending test mail to {}",
        session.user.username, data.to
    );

    let (tx, mut rx) = unbounded_channel();
    let mail = Mail {
        to: data.to.clone(),
        subject: TEST_MAIL_SUBJECT.to_string(),
        content: templates::test_mail()?,
        attachments: Vec::new(),
        result_tx: Some(tx),
    };
    let (to, subject) = (mail.to.clone(), mail.subject.clone());
    match appstate.mail_tx.send(mail) {
        Ok(_) => match rx.recv().await {
            Some(Ok(_)) => {
                info!(
                    "User {} sent test mail to {}",
                    session.user.username, data.to
                );
                Ok(ApiResponse {
                    json: json!({}),
                    status: StatusCode::OK,
                })
            }
            Some(Err(err)) => Ok(internal_error(&to, &subject, &err)),
            None => Ok(internal_error(
                &to,
                &subject,
                &String::from("None received"),
            )),
        },
        Err(err) => Ok(internal_error(&to, &subject, &err)),
    }
}

async fn read_logs(config: &DefGuardConfig) -> String {
    let Some(path) = &config.log_file else {
        return "Log file not configured".to_string();
    };

    match read_to_string(path).await {
        Ok(logs) => logs,
        Err(err) => {
            error!("Error dumping app logs: {err}");
            format!("Error dumping app logs: {err}")
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
    let config = dump_config(&appstate.pool, &appstate.config).await;
    let config =
        serde_json::to_string_pretty(&config).unwrap_or("Json formatting error".to_string());
    let config = Attachment {
        filename: format!("defguard-support-data-{}.json", Utc::now()),
        content: config.into(),
        content_type: ContentType::TEXT_PLAIN,
    };
    let logs = read_logs(&appstate.config).await;
    let logs = Attachment {
        filename: format!("defguard-logs-{}.txt", Utc::now()),
        content: logs.into(),
        content_type: ContentType::TEXT_PLAIN,
    };
    let (tx, mut rx) = unbounded_channel();
    let mail = Mail {
        to: SUPPORT_EMAIL_ADDRESS.to_string(),
        subject: SUPPORT_EMAIL_SUBJECT.to_string(),
        content: support_data_mail()?,
        attachments: vec![config, logs],
        result_tx: Some(tx),
    };
    let (to, subject) = (mail.to.clone(), mail.subject.clone());
    match appstate.mail_tx.send(mail) {
        Ok(_) => match rx.recv().await {
            Some(Ok(_)) => {
                info!(
                    "User {} sent support mail to {SUPPORT_EMAIL_ADDRESS}",
                    session.user.username
                );
                Ok(ApiResponse {
                    json: json!({}),
                    status: StatusCode::OK,
                })
            }
            Some(Err(err)) => Ok(internal_error(&to, &subject, &err)),
            None => Ok(internal_error(
                &to,
                &subject,
                &String::from("None received"),
            )),
        },
        Err(err) => Ok(internal_error(&to, &subject, &err)),
    }
}

pub async fn send_new_device_added_email(
    device: Device,
    user: User,
    device_network_ips: Vec<String>,
    mail_tx: &UnboundedSender<Mail>,
) -> Result<(), EnrollmentError> {
    debug!(
        "User {} new device added mail to {SUPPORT_EMAIL_ADDRESS}",
        user.email
    );

    let mail = Mail {
        to: user.email,
        subject: NEW_DEVICE_ADDED_EMAIL_SUBJECT.to_string(),
        content: templates::new_device_added_mail(device, device_network_ips)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    match mail_tx.send(mail) {
        Ok(_) => {
            info!("Sent new device added mail for user",);
            Ok(())
        }
        Err(err) => {
            error!("Error sending mail: {err}");
            Err(EnrollmentError::NotificationError(err.to_string()))
        }
    }
}

pub async fn send_mfa_configured_email(
    user: User,
    mfa_type: String,
    mail_tx: &UnboundedSender<Mail>,
) -> Result<(), EnrollmentError> {
    debug!("Sending MFA configured mail to {}", user.email);

    let subject = format!("MFA method {} was activated on your account", &mfa_type);

    let mail = Mail {
        to: user.email,
        subject,
        content: templates::mfa_configured_mail(mfa_type)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    match mail_tx.send(mail) {
        Ok(_) => {
            info!("Sent mfa configured mail for user",);
            Ok(())
        }
        Err(err) => {
            error!("Error sending mail: {err}");
            Err(EnrollmentError::NotificationError(err.to_string()))
        }
    }
}
