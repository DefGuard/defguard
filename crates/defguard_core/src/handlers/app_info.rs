use axum::{extract::State, http::StatusCode};
use defguard_common::{
    VERSION,
    db::models::{Settings, WireguardNetwork},
};
use serde_json::json;

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::SessionInfo,
    enterprise::{
        db::models::openid_provider::OpenIdProvider,
        is_business_license_active, is_enterprise_free,
        license::{LicenseTier, get_cached_license},
        limits::{LimitsExceeded, get_counts},
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
    // Which license tier (if any) is active
    tier: Option<LicenseTier>,
}

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
    license_info: LicenseInfo,
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
    let enterprise = is_business_license_active();
    let license = get_cached_license();
    let counts = get_counts();
    let limits_exceeded = counts.get_exceeded_limits(license.as_ref());
    let any_limit_exceeded = limits_exceeded.any();
    let tier = license.as_ref().map(|license| license.tier.clone());

    let res = AppInfo {
        network_present: !networks.is_empty(),
        smtp_enabled: settings.smtp_configured(),
        version: VERSION.into(),
        license_info: LicenseInfo {
            enterprise,
            limits_exceeded,
            any_limit_exceeded,
            is_enterprise_free: is_enterprise_free(),
            tier,
        },
        ldap_info: LdapInfo {
            enabled: settings.ldap_enabled,
            ad: settings.ldap_uses_ad,
        },
        external_openid_enabled,
        initial_setup_completed: settings.initial_setup_completed,
    };

    Ok(ApiResponse::new(json!(res), StatusCode::OK))
}
