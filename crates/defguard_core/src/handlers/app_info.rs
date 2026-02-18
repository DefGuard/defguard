use axum::{extract::State, http::StatusCode};
use defguard_common::{
    VERSION,
    db::models::{Settings, WireguardNetwork},
};

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState, auth::SessionInfo, enterprise::db::models::openid_provider::OpenIdProvider,
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
    initial_setup_completed: bool,
}

pub(crate) async fn get_app_info(
    State(appstate): State<AppState>,
    _session: SessionInfo,
) -> ApiResult {
    // both `await`s are executed upfront to avoid holding license `RwLock` across an await point
    let networks = WireguardNetwork::all(&appstate.pool).await?;
    let external_openid_enabled = OpenIdProvider::get_current(&appstate.pool).await?.is_some();

    let settings = Settings::get_current_settings();

    let res = AppInfo {
        network_present: !networks.is_empty(),
        smtp_enabled: settings.smtp_configured(),
        version: VERSION.into(),
        ldap_info: LdapInfo {
            enabled: settings.ldap_enabled,
            ad: settings.ldap_uses_ad,
        },
        external_openid_enabled,
        initial_setup_completed: settings.initial_setup_completed,
    };

    Ok(ApiResponse::json(res, StatusCode::OK))
}
