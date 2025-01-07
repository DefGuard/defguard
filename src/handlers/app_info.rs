use axum::{extract::State, http::StatusCode};
use serde_json::json;

use super::{ApiResponse, ApiResult, VERSION};
use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::{Settings, WireguardNetwork},
    enterprise::{
        is_enterprise_enabled,
        limits::{get_counts, LimitsExceeded},
    },
};

/// Additional information about core state.
#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    network_present: bool,
    smtp_enabled: bool,
    /// Whether the enterprise features are enabled.
    enterprise: bool,
    /// Which limits are exceeded.
    limits_exceeded: LimitsExceeded,
    /// Is any of the limits exceeded.
    any_limit_exceeded: bool,
}

pub(crate) async fn get_app_info(
    State(appstate): State<AppState>,
    _session: SessionInfo,
) -> ApiResult {
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let settings = Settings::get_settings(&appstate.pool).await?;
    let enterprise = is_enterprise_enabled();
    let limits_exceeded = get_counts().get_exceeded_limits();
    let any_exceeded = limits_exceeded.any();
    let res = AppInfo {
        network_present: !networks.is_empty(),
        smtp_enabled: settings.smtp_configured(),
        version: VERSION.into(),
        enterprise,
        limits_exceeded,
        any_limit_exceeded: any_exceeded,
    };

    Ok(ApiResponse::new(json!(res), StatusCode::OK))
}
