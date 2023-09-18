use std::fmt::Display;

use chrono::Utc;
use lettre::message::header::ContentType;
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};
use tokio::sync::mpsc::unbounded_channel;

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    config::DefGuardConfig,
    mail::{Attachment, Mail},
    support::dump_config,
    templates::{self, support_data_mail},
};

const TEST_MAIL_SUBJECT: &str = "Defguard email test";
const SUPPORT_EMAIL_ADDRESS: &str = "support@defguard.net";
const SUPPORT_EMAIL_SUBJECT: &str = "Defguard support data";

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
        status: Status::InternalServerError,
    }
}

#[post("/test", format = "json", data = "<data>")]
pub async fn test_mail(
    _admin: AdminRole,
    session: SessionInfo,
    appstate: &State<AppState>,
    data: Json<TestMail>,
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
                    status: Status::Ok,
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

    match tokio::fs::read_to_string(path).await {
        Ok(logs) => logs,
        Err(err) => {
            error!("Error dumping app logs: {err}");
            format!("Error dumping app logs: {err}")
        }
    }
}

#[post("/support", format = "json")]
pub async fn send_support_data(
    _admin: AdminRole,
    session: SessionInfo,
    appstate: &State<AppState>,
) -> ApiResult {
    debug!(
        "User {} sending support mail to {}",
        session.user.username, SUPPORT_EMAIL_ADDRESS
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
                    "User {} sent support mail to {}",
                    session.user.username, SUPPORT_EMAIL_ADDRESS
                );
                Ok(ApiResponse {
                    json: json!({}),
                    status: Status::Ok,
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
