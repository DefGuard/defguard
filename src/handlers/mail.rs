use std::fmt::Display;

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
    mail::Mail,
    templates,
};

const TEST_MAIL_SUBJECT: &str = "Defguard email test";

#[derive(Clone, Deserialize)]
pub struct TestMail {
    pub to: String,
}

/// Handles logging the error and returns ApiResponse that contains it
fn internal_error(from: &str, to: &str, subject: &str, error: &impl Display) -> ApiResponse {
    error!("Error sending mail from: {from}, to {to}, subject: {subject}, error: {error}");
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
        result_tx: Some(tx),
    };
    let (from, to, subject) = (data.to.clone(), mail.to.clone(), mail.subject.clone());
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
            Ok(Err(err)) => Ok(internal_error(&from, &to, &subject, &err)),
            Err(err) => Ok(internal_error(&from, &to, &subject, &err)),
        },
        Err(err) => Ok(internal_error(&from, &to, &subject, &err)),
    }
}
