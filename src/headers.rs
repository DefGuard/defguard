use std::sync::Arc;

use tera::Context;
use uaparser::{Client, Parser, UserAgentParser};

use crate::appstate::AppState;

pub fn create_user_agent_parser() -> Arc<UserAgentParser> {
    Arc::new(
        UserAgentParser::builder()
            .build_from_yaml("user_agent_header_regexes.yaml")
            .expect("Parser creation failed"),
    )
}

pub fn parse_user_agent(appstate: AppState, user_agent: &String) -> Option<uaparser::Client> {
    if user_agent.is_empty() {
        None
    } else {
        Some(appstate.user_agent_parser.parse(user_agent.as_str()))
    }
}

pub fn get_device_type(user_agent_client: Option<Client>) -> String {
    let mut device_type = String::new();
    if let Some(client) = user_agent_client {
        device_type = get_user_agent_device(&client);
    }

    device_type.to_string()
}

pub fn init_context_user_agent(user_agent_client: Option<Client>) -> Context {
    let mut context = Context::new();

    if let Some(client) = user_agent_client {
        let device_type = get_user_agent_device(&client);
        context.insert("device_type", &device_type);
    }

    context
}

pub fn get_user_agent_device(user_agent_client: &Client) -> String {
    let device_type = user_agent_client
        .device
        .model
        .clone()
        .unwrap_or(std::borrow::Cow::Borrowed("unknown model"));

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

    let mut device_os = user_agent_client.os.family.to_string() + " ";
    device_os.push_str(&device_version);
    device_os.push_str(", ");
    device_os.push_str(&user_agent_client.user_agent.family);

    format!("{}, OS: {}", device_type, device_os)
}
