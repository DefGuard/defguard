use std::fmt::Display;

use chrono::Utc;
use lettre::message::header::ContentType;
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};
use tokio::sync::oneshot::channel;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    handlers::{ApiResponse, ApiResult},
    mail::{Attachment, Mail},
    support::dump_config,
    templates::{self, support_data_mail},
};

const TEST_MAIL_SUBJECT: &str = "Defguard email test";
// TODO
const SUPPORT_EMAIL_ADDRESS: &str = "jchmielewski@teonite.com";
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

    let (tx, rx) = channel();
    let mail = Mail {
        to: data.to.clone(),
        subject: TEST_MAIL_SUBJECT.to_string(),
        content: templates::test_mail()?,
        attachments: Vec::new(),
        result_tx: Some(tx),
    };
    let (to, subject) = (mail.to.clone(), mail.subject.clone());
    match appstate.mail_tx.send(mail) {
        Ok(_) => match rx.await {
            Ok(Ok(_)) => {
                info!(
                    "User {} sent test mail to {}",
                    session.user.username, data.to
                );
                Ok(ApiResponse {
                    json: json!({}),
                    status: Status::Ok,
                })
            }
            Ok(Err(err)) => Ok(internal_error(&to, &subject, &err)),
            Err(err) => Ok(internal_error(&to, &subject, &err)),
        },
        Err(err) => Ok(internal_error(&to, &subject, &err)),
    }
}

#[post("/support", format = "json")]
pub async fn support(
    _admin: AdminRole,
    session: SessionInfo,
    appstate: &State<AppState>,
) -> ApiResult {
    debug!(
        "User {} sending support mail to {}",
        session.user.username, SUPPORT_EMAIL_ADDRESS
    );
    let config = dump_config(&appstate.pool, &appstate.config)
        .await
        .to_string();
    let config = Attachment {
        filename: format!("defguard-support-data-{}", Utc::now().to_string()),
        content: config.into(),
        content_type: ContentType::TEXT_PLAIN,
    };
    let (tx, rx) = channel();
    let mail = Mail {
        to: SUPPORT_EMAIL_ADDRESS.to_string(),
        subject: SUPPORT_EMAIL_SUBJECT.to_string(),
        content: support_data_mail()?,
        attachments: vec![config],
        result_tx: Some(tx),
    };
    let (to, subject) = (mail.to.clone(), mail.subject.clone());
    match appstate.mail_tx.send(mail) {
        Ok(_) => match rx.await {
            Ok(Ok(_)) => {
                info!(
                    "User {} sent support mail to {}",
                    session.user.username, SUPPORT_EMAIL_ADDRESS
                );
                Ok(ApiResponse {
                    json: json!({}),
                    status: Status::Ok,
                })
            }
            Ok(Err(err)) => Ok(internal_error(&to, &subject, &err)),
            Err(err) => Ok(internal_error(&to, &subject, &err)),
        },
        Err(err) => Ok(internal_error(&to, &subject, &err)),
    }
}
