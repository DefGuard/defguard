use std::{borrow::Borrow, sync::Arc};

use tokio::sync::mpsc::UnboundedSender;
use uaparser::{Client, Parser, UserAgentParser};

use crate::{
    appstate::AppState,
    db::{models::device_login::DeviceLoginEvent, DbPool, User},
    handlers::mail::send_new_device_login_email,
    mail::Mail,
    templates::TemplateError,
};

#[must_use]
pub fn create_user_agent_parser() -> Arc<UserAgentParser> {
    Arc::new(
        UserAgentParser::builder()
            .build_from_yaml("user_agent_header_regexes.yaml")
            .expect("Parser creation failed"),
    )
}

#[must_use]
pub fn parse_user_agent<'a>(appstate: &'a AppState, user_agent: &'a str) -> Option<Client<'a>> {
    if user_agent.is_empty() {
        None
    } else {
        Some(appstate.user_agent_parser.parse(user_agent))
    }
}

#[must_use]
pub fn get_device_type(user_agent_client: Option<Client>) -> String {
    if let Some(client) = user_agent_client {
        get_user_agent_device(&client)
    } else {
        String::new()
    }
}

#[must_use]
pub fn get_user_agent_device(user_agent_client: &Client) -> String {
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

#[must_use]
pub fn get_device_login_event(
    user_id: i64,
    ip_address: String,
    event_type: String,
    user_agent_client: Option<Client>,
) -> Option<DeviceLoginEvent> {
    user_agent_client
        .map(|client| get_user_agent_device_login_data(user_id, ip_address, event_type, &client))
}

pub fn get_user_agent_device_login_data(
    user_id: i64,
    ip_address: String,
    event_type: String,
    user_agent_client: &Client,
) -> DeviceLoginEvent {
    let mut model = None;
    if let Some(ua_model) = &user_agent_client.device.model {
        model = Some(ua_model.to_string());
    }

    let mut brand = None;
    if let Some(ua_brand) = &user_agent_client.device.brand {
        brand = Some(ua_brand.to_string());
    }

    let family = user_agent_client.device.family.to_string();
    let os_family = user_agent_client.os.family.to_string();
    let browser = user_agent_client.user_agent.family.to_string();

    DeviceLoginEvent::new(
        user_id, ip_address, model, family, brand, os_family, browser, event_type,
    )
}

pub async fn check_new_device_login(
    pool: &DbPool,
    mail_tx: &UnboundedSender<Mail>,
    user: &User,
    ip_address: String,
    event_type: String,
    agent: Option<Client<'_>>,
) -> Result<(), TemplateError> {
    if let Some(user_id) = user.id {
        if let Some(device_login_event) =
            get_device_login_event(user_id, ip_address, event_type, agent.clone())
        {
            if let Ok(Some(created_device_login_event)) = device_login_event
                .check_if_device_already_logged_in(pool)
                .await
            {
                send_new_device_login_email(
                    &user.email,
                    mail_tx,
                    agent,
                    created_device_login_event.ip_address,
                    created_device_login_event.created,
                )
                .await?;
            }
        }
    }

    Ok(())
}
