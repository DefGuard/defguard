use axum::{extract::State, http::StatusCode};
use serde_json::json;

use super::{ApiResponse, ApiResult, VERSION};
use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::{Settings, WireguardNetwork},
    enterprise::{
        is_enterprise_enabled, is_enterprise_free,
        license::get_cached_license,
        limits::{get_counts, LimitsExceeded},
    },
};

#[derive(Serialize)]
struct LicenseInfo {
    /// Whether the enterprise features are enabled.
    enterprise: bool,
    /// Which limits are exceeded.
    limits_exceeded: LimitsExceeded,
    /// Is any of the limits exceeded.
    any_limit_exceeded: bool,
    /// Whether the enterprise features are used for free.
    is_enterprise_free: bool,
}

/// Additional information about core state.
#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    network_present: bool,
    smtp_enabled: bool,
    license_info: LicenseInfo,
}

pub(crate) async fn get_app_info(
    State(appstate): State<AppState>,
    _session: SessionInfo,
) -> ApiResult {
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let settings = Settings::get_settings(&appstate.pool).await?;
    let enterprise = is_enterprise_enabled();
    let license = get_cached_license();
    let limits_exceeded = get_counts().get_exceeded_limits(license.as_ref());
    let any_limit_exceeded = limits_exceeded.any();
    let res = AppInfo {
        network_present: !networks.is_empty(),
        smtp_enabled: settings.smtp_configured(),
        version: VERSION.into(),
        license_info: LicenseInfo {
            enterprise,
            limits_exceeded,
            any_limit_exceeded,
            is_enterprise_free: is_enterprise_free(),
        },
    };

    Ok(ApiResponse::new(json!(res), StatusCode::OK))
}
