use std::sync::{Arc, Mutex};

use axum::{Extension, http::StatusCode};
use serde_json::{Value, json};

use super::{ApiResponse, ApiResult};
use crate::{
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
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    let gateway_state = gateway_state
        .lock()
        .expect("Failed to acquire gateway state lock");
    let mut version_info = gateway_state.all_states_as_version_info();
    if let Ok(state) = PROXY_STATE.read() {
        version_info.push((*state).clone());
    }
    Ok(ApiResponse::new(json!(version_info), StatusCode::OK))
}
