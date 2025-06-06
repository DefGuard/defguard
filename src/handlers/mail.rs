use std::fmt::Display;

use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use chrono::{NaiveDateTime, Utc};
use lettre::message::header::ContentType;
use reqwest::Url;
use serde_json::json;
use tokio::{
    fs::read_to_string,
    sync::mpsc::{unbounded_channel, UnboundedSender},
};

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{models::enrollment::TokenError, Id, MFAMethod, Session, User},
    error::WebError,
    mail::{Attachment, Mail},
    server_config,
    support::dump_config,
    templates::{self, support_data_mail, TemplateError, TemplateLocation},
    PgPool,
};

static TEST_MAIL_SUBJECT: &str = "Defguard email test";
static SUPPORT_EMAIL_ADDRESS: &str = "support@defguard.net";
static SUPPORT_EMAIL_SUBJECT: &str = "Defguard support data";

static NEW_DEVICE_ADDED_EMAIL_SUBJECT: &str = "Defguard: new device added to your account";
static NEW_DEVICE_LOGIN_EMAIL_SUBJECT: &str = "Defguard: new device logged in to your account";

static EMAIL_MFA_ACTIVATION_EMAIL_SUBJECT: &str = "Your Multi-Factor Authentication Activation";
static EMAIL_MFA_CODE_EMAIL_SUBJECT: &str = "Your Multi-Factor Authentication Code for Login";

static GATEWAY_DISCONNECTED: &str = "Defguard: Gateway disconnected";
static GATEWAY_RECONNECTED: &str = "Defguard: Gateway reconnected";

pub static EMAIL_PASSOWRD_RESET_START_SUBJECT: &str = "Defguard: Password reset";
pub static EMAIL_PASSOWRD_RESET_SUCCESS_SUBJECT: &str = "Defguard: Password reset success";

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
        content: templates::test_mail(Some(&session.session))?,
        attachments: Vec::new(),
        result_tx: Some(tx),
    };
    let (to, subject) = (mail.to.clone(), mail.subject.clone());
    match appstate.mail_tx.send(mail) {
        Ok(()) => match rx.recv().await {
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

async fn read_logs() -> String {
    let Some(path) = &server_config().log_file else {
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
    let config = dump_config(&appstate.pool).await;
    let config =
        serde_json::to_string_pretty(&config).unwrap_or("Json formatting error".to_string());
    let config = Attachment {
        filename: format!("defguard-support-data-{}.json", Utc::now()),
        content: config.into(),
        content_type: ContentType::TEXT_PLAIN,
    };
    let logs = read_logs().await;
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
        Ok(()) => match rx.recv().await {
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

pub fn send_new_device_added_email(
    device_name: &str,
    public_key: &str,
    template_locations: &[TemplateLocation],
    user_email: &str,
    mail_tx: &UnboundedSender<Mail>,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TemplateError> {
    debug!("User {user_email} new device added mail to {SUPPORT_EMAIL_ADDRESS}");

    let mail = Mail {
        to: user_email.to_string(),
        subject: NEW_DEVICE_ADDED_EMAIL_SUBJECT.to_string(),
        content: templates::new_device_added_mail(
            device_name,
            public_key,
            template_locations,
            ip_address,
            device_info,
        )?,
        attachments: Vec::new(),
        result_tx: None,
    };

    let to = mail.to.clone();

    match mail_tx.send(mail) {
        Ok(()) => {
            info!("Sent new device notification to {to}");
            Ok(())
        }
        Err(err) => {
            error!("Sending new device notification to {to} failed with erorr:\n{err}");
            Ok(())
        }
    }
}

pub async fn send_gateway_disconnected_email(
    gateway_name: Option<String>,
    network_name: String,
    gateway_adress: &str,
    mail_tx: &UnboundedSender<Mail>,
    pool: &PgPool,
) -> Result<(), WebError> {
    debug!("Sending gateway disconnected mail to all admin users");
    let admin_users = User::find_admins(pool).await?;
    let gateway_name = gateway_name.unwrap_or_default();
    for user in admin_users {
        let mail = Mail {
            to: user.email,
            subject: GATEWAY_DISCONNECTED.to_string(),
            content: templates::gateway_disconnected_mail(
                &gateway_name,
                gateway_adress,
                &network_name,
            )?,
            attachments: Vec::new(),
            result_tx: None,
        };
        let to = mail.to.clone();

        match mail_tx.send(mail) {
            Ok(()) => {
                info!("Sent gateway disconnected notification to {to}");
            }
            Err(err) => {
                error!(
                    "Sending gateway disconnected notification to {to} failed with error:\n{err}"
                );
            }
        }
    }
    Ok(())
}

pub async fn send_gateway_reconnected_email(
    gateway_name: Option<String>,
    network_name: String,
    gateway_adress: &str,
    mail_tx: &UnboundedSender<Mail>,
    pool: &PgPool,
) -> Result<(), WebError> {
    debug!("Sending gateway reconnect mail to all admin users");
    let admin_users = User::find_admins(pool).await?;
    let gateway_name = gateway_name.unwrap_or_default();
    for user in admin_users {
        let mail = Mail {
            to: user.email,
            subject: GATEWAY_RECONNECTED.to_string(),
            content: templates::gateway_reconnected_mail(
                &gateway_name,
                gateway_adress,
                &network_name,
            )?,
            attachments: Vec::new(),
            result_tx: None,
        };
        let to = mail.to.clone();

        match mail_tx.send(mail) {
            Ok(()) => {
                info!("Sent gateway reconnected notification to {to}");
            }
            Err(err) => {
                error!(
                    "Sending gateway reconnected notification to {to} failed with error:\n{err}"
                );
            }
        }
    }
    Ok(())
}

pub async fn send_new_device_login_email(
    user_email: &str,
    mail_tx: &UnboundedSender<Mail>,
    session: &Session,
    created: NaiveDateTime,
) -> Result<(), TemplateError> {
    debug!("User {user_email} new device login mail to {SUPPORT_EMAIL_ADDRESS}");

    let mail = Mail {
        to: user_email.to_string(),
        subject: NEW_DEVICE_LOGIN_EMAIL_SUBJECT.to_string(),
        content: templates::new_device_login_mail(session, created)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    let to = mail.to.clone();

    match mail_tx.send(mail) {
        Ok(()) => {
            info!("Sent new device login notification to {to}");
        }
        Err(err) => {
            error!("Sending new device login notification to {to} failed with erorr:\n{err}");
        }
    }

    Ok(())
}

pub async fn send_new_device_ocid_login_email(
    user_email: &str,
    oauth2client_name: String,
    mail_tx: &UnboundedSender<Mail>,
    session: &Session,
) -> Result<(), TemplateError> {
    debug!("User {user_email} new device OCID login mail to {SUPPORT_EMAIL_ADDRESS}");

    let subject = format!("New login to {oauth2client_name} application with defguard");

    let mail = Mail {
        to: user_email.to_string(),
        subject,
        content: templates::new_device_ocid_login_mail(session, &oauth2client_name)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    let to = mail.to.clone();

    match mail_tx.send(mail) {
        Ok(()) => {
            info!("Sent new device OCID login notification to {to}");
        }
        Err(err) => {
            error!("Sending new device OCID login notification to {to} failed with erorr:\n{err}");
        }
    }

    Ok(())
}

pub fn send_mfa_configured_email(
    session: Option<&Session>,
    user: &User<Id>,
    mfa_method: &MFAMethod,
    mail_tx: &UnboundedSender<Mail>,
) -> Result<(), TemplateError> {
    debug!("Sending MFA configured mail to {}", user.email);

    let subject = format!("MFA method {mfa_method} has been activated on your account");

    let mail = Mail {
        to: user.email.clone(),
        subject,
        content: templates::mfa_configured_mail(session, mfa_method)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    let to = mail.to.clone();

    match mail_tx.send(mail) {
        Ok(()) => {
            info!("MFA configured mail sent to {to}");
            Ok(())
        }
        Err(err) => {
            error!("Failed to send mfa configured mail to {to} with error:\n{err}");
            Ok(())
        }
    }
}

pub fn send_email_mfa_activation_email(
    user: &User<Id>,
    mail_tx: &UnboundedSender<Mail>,
    session: &Session,
) -> Result<(), TemplateError> {
    debug!("Sending email MFA activation mail to {}", user.email);

    // generate a verification code
    let code = user.generate_email_mfa_code().map_err(|err| {
        error!("Failed to generate email MFA code: {err}");
        TemplateError::MfaError
    })?;

    let mail = Mail {
        to: user.email.clone(),
        subject: EMAIL_MFA_ACTIVATION_EMAIL_SUBJECT.into(),
        content: templates::email_mfa_activation_mail(user, &code, session)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    let to = mail.to.clone();

    match mail_tx.send(mail) {
        Ok(()) => {
            info!("Email MFA activation mail sent to {to}");
            Ok(())
        }
        Err(err) => {
            error!("Failed to send email MFA activation mail to {to} with error:\n{err}");
            Ok(())
        }
    }
}

pub fn send_email_mfa_code_email(
    user: &User<Id>,
    mail_tx: &UnboundedSender<Mail>,
    session: Option<&Session>,
) -> Result<(), TemplateError> {
    debug!("Sending email MFA code mail to {}", user.email);

    // generate a verification code
    let code = user.generate_email_mfa_code().map_err(|err| {
        error!("Failed to generate email MFA code: {err}");
        TemplateError::MfaError
    })?;

    let mail = Mail {
        to: user.email.clone(),
        subject: EMAIL_MFA_CODE_EMAIL_SUBJECT.into(),
        content: templates::email_mfa_code_mail(user, &code, session)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    let to = mail.to.clone();

    match mail_tx.send(mail) {
        Ok(()) => {
            info!("Email MFA code mail sent to {to}");
            Ok(())
        }
        Err(err) => {
            error!("Failed to send email MFA code mail to {to} with error:\n{err}");
            Ok(())
        }
    }
}

pub fn send_password_reset_email(
    user: &User<Id>,
    mail_tx: &UnboundedSender<Mail>,
    service_url: Url,
    token: &str,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TokenError> {
    debug!("Sending password reset email to {}", user.email);

    let mail = Mail {
        to: user.email.clone(),
        subject: EMAIL_PASSOWRD_RESET_START_SUBJECT.into(),
        content: templates::email_password_reset_mail(service_url, token, ip_address, device_info)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    let to = mail.to.clone();

    match mail_tx.send(mail) {
        Ok(()) => {
            info!("Password reset email sent to {to}");
            Ok(())
        }
        Err(err) => {
            error!("Failed to send password reset email to {to} with error:\n{err}");
            Err(TokenError::NotificationError(err.to_string()))
        }
    }
}

pub fn send_password_reset_success_email(
    user: &User<Id>,
    mail_tx: &UnboundedSender<Mail>,
    ip_address: Option<&str>,
    device_info: Option<&str>,
) -> Result<(), TokenError> {
    debug!("Sending password reset success email to {}", user.email);

    let mail = Mail {
        to: user.email.clone(),
        subject: EMAIL_PASSOWRD_RESET_SUCCESS_SUBJECT.into(),
        content: templates::email_password_reset_success_mail(ip_address, device_info)?,
        attachments: Vec::new(),
        result_tx: None,
    };

    let to = mail.to.clone();

    match mail_tx.send(mail) {
        Ok(()) => {
            info!("Password reset email success sent to {to}");
        }
        Err(err) => {
            error!("Failed to send password reset success email to {to} with error:\n{err}");
        }
    }
    Ok(())
}
