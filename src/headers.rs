use axum::headers::UserAgent;
use tera::Context;
use uaparser::{Parser, Client};

use crate::appstate::AppState;

pub fn parse_user_agent(appstate: AppState, user_agent: &UserAgent) -> uaparser::Client {
    return appstate.user_agent_parser.parse(user_agent.as_str());
}

pub fn init_context_user_agent(user_agent_client: Option<Client>) -> Context {
    let mut context = Context::new();

    if user_agent_client.is_some() {
        let device_type = get_user_agent_device(user_agent_client.unwrap().clone());
        context.insert("device_type", &device_type);
    }

    return context;
}

pub fn get_user_agent_device(user_agent_client: Client) -> String {
    let device_type = match user_agent_client.device.model {
        Some(v) => v.to_string(),
        None => "".to_string(),
    };

    let device_os_major = match user_agent_client.os.major {
        Some(v) => v.to_string(),
        None => "".to_string(),
    };

    let device_os_minor = match user_agent_client.os.minor {
        Some(v) => v.to_string(),
        None => "".to_string(),
    };

    let device_os_patch = match user_agent_client.os.patch {
        Some(v) => v.to_string(),
        None => "".to_string(),
    };

    let mut device_version_list = vec![device_os_major, device_os_minor, device_os_patch];
    device_version_list.retain(|ver| !ver.is_empty());
    let device_version = device_version_list.join(".");

    let mut device_os = user_agent_client.os.family.to_string();
    device_os.push_str(" ");
    device_os.push_str(&device_version);
    device_os.push_str(", ");
    device_os.push_str(&user_agent_client.user_agent.family);

    return format!("{}, OS: {}", device_type.to_string(), device_os.to_string());
}
