use std::sync::{Arc, Mutex};

use axum::{Extension, extract::State, http::StatusCode};
use defguard_version::{DefguardComponent, tracing::VersionInfo};
use serde_json::{Value, json};

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    grpc::{gateway::map::GatewayMap, state::PROXY_STATE},
    updates::get_update,
};

pub(crate) async fn check_new_version(_admin: AdminRole, session: SessionInfo) -> ApiResult {
    debug!(
        "User {} is checking if there is a new version available",
        session.user.username
    );
    let json = if let Some(update) = get_update().as_ref() {
        debug!("A new version is available, returning the update information");
        json!(update)
    } else {
        debug!("No new version available");
        // Front-end expects empty JSON.
        Value::Null
    };
    Ok(ApiResponse {
        json,
        status: StatusCode::OK,
    })
}

// FIXME: Switch to SSE and generally make it better.
pub(crate) async fn outdated_components(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    let mut components = if let Some(outdated_gateways) = appstate
        .incompatible_components
        .lock()
        .expect("Failed to lock appstate.incompatible_components")
        .get(&DefguardComponent::Gateway)
    {
        outdated_gateways
            .iter()
            .filter_map(|opt| {
                opt.as_ref().map(|version| VersionInfo {
                    component: Some(DefguardComponent::Gateway),
                    info: None,
                    version: Some(version.to_string()),
                    is_supported: false,
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    if let Ok(state) = PROXY_STATE.read() {
        if state.version.is_some() {
            components.push((*state).clone());
        }
    }
    Ok(ApiResponse::new(json!(components), StatusCode::OK))
}
