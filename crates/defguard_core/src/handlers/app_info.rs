use axum::{extract::State, http::StatusCode};
use defguard_common::{
    REPORTED_VERSION,
    db::models::{Settings, WireguardNetwork},
};

use super::{ApiErrorResponse, ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::SessionInfo,
    enterprise::{db::models::openid_provider::OpenIdProvider, is_business_license_active},
};

#[derive(Serialize)]
struct LdapInfo {
    /// Whether that integration is enabled (at least one way synchronization)
    enabled: bool,
    /// Whether AD is used
    ad: bool,
}

/// Additional information about core state.
#[derive(Serialize)]
pub struct AppInfo {
    version: String,
    network_present: bool,
    smtp_enabled: bool,
    ldap_info: LdapInfo,
    external_openid_enabled: bool,
}

/// Get information about this defguard instance.
#[utoipa::path(
    get,
    path = "/api/v1/info",
    tag = "system",
    responses(
        (status = 200, description = "Instance information: enabled modules, version, license state.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to get instance information.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_app_info(State(appstate): State<AppState>, _session: SessionInfo) -> ApiResult {
    // both `await`s are executed upfront to avoid holding license `RwLock` across an await point
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let external_openid_enabled = OpenIdProvider::get_current(&appstate.pool).await?.is_some();

    let settings = Settings::get_current_settings();
    let mut smtp_enabled = settings.smtp_configured();
    // XOAUTH2 is only for the business licence.
    if settings.smtp.is_xoauth2() && !is_business_license_active() {
        smtp_enabled = false;
    }

    let res = AppInfo {
        network_present: !networks.is_empty(),
        smtp_enabled,
        version: REPORTED_VERSION.into(),
        ldap_info: LdapInfo {
            enabled: settings.ldap_enabled,
            ad: settings.ldap_uses_ad,
        },
        external_openid_enabled,
    };

    Ok(ApiResponse::json(res, StatusCode::OK))
}
