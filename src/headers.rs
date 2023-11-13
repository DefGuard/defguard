use std::{borrow::Borrow, sync::Arc};

use uaparser::{Client, Parser, UserAgentParser};

use crate::appstate::AppState;

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
        device_version.push_str(major.to_string().as_str());

        if let Some(minor) = &user_agent_client.os.minor {
            device_version.push('.');
            device_version.push_str(minor.to_string().as_str());

            if let Some(patch) = &user_agent_client.os.patch {
                device_version.push('.');
                device_version.push_str(patch.to_string().as_str());
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
