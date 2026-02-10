use std::{borrow::Borrow, sync::LazyLock};

use axum::http::{HeaderName, HeaderValue};
use defguard_common::db::{
    Id,
    models::{DeviceLoginEvent, User},
};
use defguard_mail::templates::{SessionContext, TemplateError};
use sqlx::PgPool;
use uaparser::{Client, Parser, UserAgentParser};

use crate::handlers::mail::send_new_device_login_email;

pub(crate) const CONTENT_SECURITY_POLICY_HEADER_NAME: HeaderName =
    HeaderName::from_static("content-security-policy");
pub(crate) const CONTENT_SECURITY_POLICY_HEADER_VALUE: HeaderValue =
    HeaderValue::from_static("frame-ancestors 'none';");

pub static USER_AGENT_PARSER: LazyLock<UserAgentParser> = LazyLock::new(|| {
    let regexes = include_bytes!("../user_agent_header_regexes.yaml");
    UserAgentParser::from_bytes(regexes).expect("Parser creation failed")
});

#[must_use]
pub fn get_device_info(user_agent: &str) -> String {
    let escaped = tera::escape_html(user_agent);
    let client = USER_AGENT_PARSER.parse(&escaped);
    get_user_agent_device(&client)
}

#[must_use]
pub(crate) fn get_user_agent_device(user_agent_client: &Client) -> String {
    let device_type = user_agent_client
        .device
        .model
        .as_ref()
        .map_or("unknown model", Borrow::borrow);

    let mut device_version = String::new();
    if let Some(major) = &user_agent_client.os.major {
        device_version.push_str(major);

        if let Some(minor) = &user_agent_client.os.minor {
            device_version.push('.');
            device_version.push_str(minor);

            if let Some(patch) = &user_agent_client.os.patch {
                device_version.push('.');
                device_version.push_str(patch);
            }
        }
    }

    let mut device_os = user_agent_client.os.family.to_string();
    device_os.push(' ');
    device_os.push_str(&device_version);
    device_os.push_str(", ");
    device_os.push_str(&user_agent_client.user_agent.family);

    format!("{device_type}, OS: {device_os}")
}

fn get_user_agent_device_login_data(
    user_id: Id,
    ip_address: String,
    event_type: String,
    user_agent_client: &Client,
) -> DeviceLoginEvent {
    let model = user_agent_client
        .device
        .model
        .as_ref()
        .map(ToString::to_string);

    let brand = user_agent_client
        .device
        .brand
        .as_ref()
        .map(ToString::to_string);

    let family = user_agent_client.device.family.to_string();
    let os_family = user_agent_client.os.family.to_string();
    let browser = user_agent_client.user_agent.family.to_string();

    DeviceLoginEvent::new(
        user_id, ip_address, model, family, brand, os_family, browser, event_type,
    )
}

pub(crate) async fn check_new_device_login(
    pool: &PgPool,
    session: &SessionContext,
    user: &User<Id>,
    ip_address: String,
    event_type: String,
    agent: Client<'_>,
) -> Result<(), TemplateError> {
    let device_login_event =
        get_user_agent_device_login_data(user.id, ip_address, event_type, &agent);

    if let Ok(Some(created_device_login_event)) = device_login_event
        .check_if_device_already_logged_in(pool)
        .await
    {
        send_new_device_login_email(&user.email, session, created_device_login_event.created)?;
    }

    Ok(())
}
