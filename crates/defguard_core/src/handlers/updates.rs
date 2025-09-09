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
    let mut components: Vec<_> = appstate
        .incompatible_components
        .read()
        .expect("Failed to lock appstate.incompatible_components")
        .iter()
        .map(|metadata| VersionInfo {
            component: Some(metadata.component.clone()),
            info: None,
            version: metadata.version.as_ref().map(|version| version.to_string()),
            is_supported: false,
        })
        .collect();
    if let Ok(state) = PROXY_STATE.read() {
        if state.version.is_some() {
            components.push((*state).clone());
        }
    }
    Ok(ApiResponse::new(json!(components), StatusCode::OK))
}
