use axum::{
    Extension,
    extract::{Json, Path, State},
    http::StatusCode,
};
use defguard_common::db::{
    Id,
    models::{
        Settings, SettingsEssentials,
        settings::{LdapSyncStatus, SettingsPatch, update_current_settings},
    },
};
use sqlx::PgPool;
use struct_patch::Patch;

use super::{ApiErrorResponse, ApiResponse, ApiResult};
use crate::{
    AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{handlers::LicenseInfo, ldap::LDAPConnection, license::update_cached_license},
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
};

static DEFAULT_NAV_LOGO_URL: &str = "/svg/defguard-nav-logo.svg";
static DEFAULT_MAIN_LOGO_URL: &str = "/svg/logo-defguard-white.svg";

/// Get instance settings
#[utoipa::path(
    get,
    path = "/api/v1/settings",
    tag = "settings",
    responses(
        (status = 200, description = "Instance settings.", body = Settings),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_settings(_admin: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Retrieving settings");
    if let Some(mut settings) = Settings::get(&appstate.pool).await? {
        if settings.nav_logo_url.is_empty() {
            settings.nav_logo_url = DEFAULT_NAV_LOGO_URL.into();
        }
        if settings.main_logo_url.is_empty() {
            settings.main_logo_url = DEFAULT_MAIN_LOGO_URL.into();
        }
        return Ok(ApiResponse::json(settings, StatusCode::OK));
    }
    debug!("Retrieved settings");
    Ok(ApiResponse::default())
}

/// Replace instance settings
///
/// The whole settings object has to be sent. Use `PATCH` to update selected fields only.
#[utoipa::path(
    put,
    path = "/api/v1/settings",
    tag = "settings",
    request_body = Settings,
    responses(
        (status = 200, description = "Settings updated."),
        (status = 400, description = "Invalid settings.", body = ApiErrorResponse, example = json!({"msg": "Invalid settings"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to update settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn update_settings(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(mut data): Json<Settings>,
) -> ApiResult {
    debug!("User {} updating settings", session.user.username);

    // fetch current settings for event
    let before = Settings::get_current_settings();
    let license = data.license.clone();

    data.uuid = before.uuid;
    data.validate()?;
    // clone for event
    let after = data.clone();

    update_current_settings(&appstate.pool, data).await?;
    update_cached_license(license.as_deref())?;

    info!("User {} updated settings", session.user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::SettingsUpdated { before, after }),
    })?;

    Ok(ApiResponse::default())
}

/// Get settings required to render the web UI
///
/// Public endpoint. Returns only non-sensitive settings.
#[utoipa::path(
    get,
    path = "/api/v1/settings_essentials",
    tag = "settings",
    responses(
        (status = 200, description = "Essential settings.", body = Object, example = json!({
            "instance_name": "defguard",
            "main_logo_url": "/svg/logo-defguard-white.svg",
            "nav_logo_url": "/svg/defguard-nav-logo.svg",
            "wireguard_enabled": true,
            "webhooks_enabled": true,
            "worker_enabled": false,
            "openid_enabled": true
        })),
        (status = 500, description = "Unable to get essential settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
)]
pub async fn get_settings_essentials(Extension(pool): Extension<PgPool>) -> ApiResult {
    debug!("Retrieving essential settings");
    let mut settings = SettingsEssentials::get_settings_essentials(&pool).await?;
    if settings.nav_logo_url.is_empty() {
        settings.nav_logo_url = DEFAULT_NAV_LOGO_URL.into();
    }
    if settings.main_logo_url.is_empty() {
        settings.main_logo_url = DEFAULT_MAIN_LOGO_URL.into();
    }

    info!("Retrieved essential settings");

    Ok(ApiResponse::json(settings, StatusCode::OK))
}

/// Restore default branding settings
#[utoipa::path(
    put,
    path = "/api/v1/settings/{id}",
    tag = "settings",
    params(
        ("id" = i64, Path, description = "Not used."),
    ),
    responses(
        (status = 200, description = "Instance settings, with the branding fields restored to defaults.", body = Settings),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to restore default branding settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn set_default_branding(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(_id): Path<Id>, // TODO: check with front-end and remove.
    session: SessionInfo,
    context: ApiRequestContext,
) -> ApiResult {
    debug!(
        "User {} restoring default branding settings",
        session.user.username
    );
    let settings = Settings::get(&appstate.pool).await?;
    match settings {
        Some(mut settings) => {
            settings.instance_name = "Defguard".into();
            settings.nav_logo_url = DEFAULT_NAV_LOGO_URL.into();
            settings.main_logo_url = DEFAULT_MAIN_LOGO_URL.into();
            update_current_settings(&appstate.pool, settings.clone()).await?;
            info!(
                "User {} restored default branding settings",
                session.user.username
            );
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::SettingsDefaultBrandingRestored),
            })?;
            Ok(ApiResponse::json(settings, StatusCode::OK))
        }
        None => Err(WebError::DbError("Cannot restore settings".into())),
    }
}

/// Update selected instance settings
///
/// Only the fields present in the request body are modified. Sending `null` clears a field.
#[utoipa::path(
    patch,
    path = "/api/v1/settings",
    tag = "settings",
    request_body = Object,
    responses(
        (status = 200, description = "Settings updated."),
        (status = 400, description = "Invalid settings.", body = ApiErrorResponse, example = json!({"msg": "Invalid settings"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to update settings.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn patch_settings(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<SettingsPatch>,
) -> ApiResult {
    debug!("Admin {} is patching settings", session.user.username);
    let mut settings = Settings::get_current_settings();
    // prepare clone for emitting an event
    let before = settings.clone();
    let license = data.license.clone();

    // update LDAP sync status if relevant settings have been changed
    if let Some(ldap_enabled) = data.ldap_enabled
        && !ldap_enabled
    {
        settings.ldap_sync_status = LdapSyncStatus::OutOfSync;
    }
    if let Some(ldap_authority) = data.ldap_is_authoritative
        && settings.ldap_is_authoritative != ldap_authority
    {
        settings.ldap_sync_status = LdapSyncStatus::OutOfSync;
    }
    if let Some(ldap_sync_groups) = &data.ldap_sync_groups
        && &settings.ldap_sync_groups != ldap_sync_groups
    {
        settings.ldap_sync_status = LdapSyncStatus::OutOfSync;
    }

    settings.apply(data);
    settings.validate()?;

    // clone for event
    let after = settings.clone();
    update_current_settings(&appstate.pool, settings).await?;
    if let Some(license_key) = &license {
        update_cached_license(license_key.as_deref())?;
        debug!("Updated cached license after saving settings patch");
    }

    info!("Admin {} patched settings", session.user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::SettingsUpdatedPartial { before, after }),
    })?;
    Ok(ApiResponse::default())
}

/// Test the LDAP connection using the currently saved settings
#[utoipa::path(
    get,
    path = "/api/v1/ldap/test",
    tag = "LDAP",
    responses(
        (status = 200, description = "LDAP connection established."),
        (status = 400, description = "Unable to connect to LDAP."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to test LDAP connection.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn test_ldap_settings(_admin: AdminRole, _license: LicenseInfo) -> ApiResult {
    debug!("Testing LDAP connection");
    match LDAPConnection::create().await {
        Ok(_) => {
            debug!("LDAP connected successfully");
            Ok(ApiResponse::with_status(StatusCode::OK))
        }
        Err(err) => {
            debug!("LDAP connection rejected: {err}");
            Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST))
        }
    }
}
