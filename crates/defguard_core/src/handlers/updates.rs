use std::sync::{Arc, Mutex};

use axum::{Extension, http::StatusCode};
use defguard_version::tracing::VersionInfo;
use serde::Serialize;
use serde_json::{Value, json};

use super::{ApiResponse, ApiResult};
use crate::{
    auth::{AdminRole, SessionInfo},
    grpc::gateway::map::GatewayMap,
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

#[derive(Serialize)]
pub(crate) struct OutdatedComponents {
    gateways: Vec<VersionInfo>,
}

// FIXME: Switch to SSE and generally make it better.
pub(crate) async fn outdated_components(
    _admin: AdminRole,
    Extension(gateway_state): Extension<Arc<Mutex<GatewayMap>>>,
) -> ApiResult {
    let gateway_state = gateway_state
        .lock()
        .expect("Failed to acquire gateway state lock");
    Ok(ApiResponse::new(
        json!(OutdatedComponents {
            gateways: gateway_state.all_states_as_version_info()
        }),
        StatusCode::OK,
    ))
}
