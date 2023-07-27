use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::OriWebError,
    handlers::{ApiResponse, ApiResult},
    mail::Mail,
    templates,
};

const TEST_MAIL_SUBJECT: &str = "Defguard email test";

#[derive(Clone, Deserialize)]
pub struct TestMail {
    pub to: String,
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
    let mail = Mail {
        to: data.to.clone(),
        subject: TEST_MAIL_SUBJECT.to_string(),
        content: templates::test_mail()?,
    };
    match appstate.mail_tx.send(mail.clone()) {
        Ok(_) => {
            info!(
                "User {} sent test mail to {}",
                session.user.username, data.to
            );
            Ok(ApiResponse {
                json: json!({}),
                status: Status::Ok,
            })
        }
        Err(err) => {
            error!("Error sending mail: {mail:?}: {err}");
            Err(OriWebError::Http(Status::InternalServerError))
        }
    }
}
