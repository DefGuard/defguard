use axum::{extract::State, http::StatusCode};
use defguard_version::{DefguardComponent, tracing::VersionInfo};
use serde_json::{Value, json};

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
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
) -> ApiResult {
    // gateways
    let mut outdated_components: Vec<_> = appstate
        .incompatible_components
        .read()
        .expect("Failed to lock appstate.incompatible_components")
        .gateways
        .iter()
        .map(|data| VersionInfo {
            component: Some(DefguardComponent::Gateway),
            info: None,
            version: Some(format!(
                "{} ({})",
                data.version
                    .as_ref()
                    .map(|version| version.to_string())
                    .unwrap_or("unknown version".to_string()),
                data.hostname
                    .clone()
                    .unwrap_or("unknown hostname".to_string())
            )),
            is_supported: false,
        })
        .collect();

    // // proxy
    // if let Ok(state) = PROXY_STATE.read() {
    //     if !state.is_supported {
    //         outdated_components.push((*state).clone());
    //     }
    // }
    Ok(ApiResponse::new(json!(outdated_components), StatusCode::OK))
}
