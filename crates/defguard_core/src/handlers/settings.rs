use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;
use struct_patch::Patch;

use super::{ApiResponse, ApiResult};
use crate::{
    auth::{AdminRole, SessionInfo},
    db::{
        models::settings::{update_current_settings, SettingsEssentials, SettingsPatch},
        Settings,
    },
    enterprise::{
        ldap::{sync::SyncStatus, LDAPConnection},
        license::update_cached_license,
    },
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    AppState,
};

static DEFAULT_NAV_LOGO_URL: &str = "/svg/defguard-nav-logo.svg";
static DEFAULT_MAIN_LOGO_URL: &str = "/svg/logo-defguard-white.svg";

pub async fn get_settings(_admin: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Retrieving settings");
    if let Some(mut settings) = Settings::get(&appstate.pool).await? {
        if settings.nav_logo_url.is_empty() {
            settings.nav_logo_url = DEFAULT_NAV_LOGO_URL.into();
        }
        if settings.main_logo_url.is_empty() {
            settings.main_logo_url = DEFAULT_MAIN_LOGO_URL.into();
        }
        return Ok(ApiResponse {
            json: json!(settings),
            status: StatusCode::OK,
        });
    }
    debug!("Retrieved settings");
    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

pub async fn update_settings(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<Settings>,
) -> ApiResult {
    debug!("User {} updating settings", session.user.username);

    update_cached_license(data.license.as_deref())?;
    data.validate()?;
    update_current_settings(&appstate.pool, data).await?;

    info!("User {} updated settings", session.user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: ApiEventType::SettingsUpdated,
    })?;

    Ok(ApiResponse::default())
}

pub async fn get_settings_essentials(State(appstate): State<AppState>) -> ApiResult {
    debug!("Retrieving essential settings");
    let mut settings = SettingsEssentials::get_settings_essentials(&appstate.pool).await?;
    if settings.nav_logo_url.is_empty() {
        settings.nav_logo_url = DEFAULT_NAV_LOGO_URL.into();
    }
    if settings.main_logo_url.is_empty() {
        settings.main_logo_url = DEFAULT_MAIN_LOGO_URL.into();
    }

    info!("Retrieved essential settings");

    Ok(ApiResponse {
        json: json!(settings),
        status: StatusCode::OK,
    })
}

pub async fn set_default_branding(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(_id): Path<i64>, // TODO: check with front-end and remove.
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
                event: ApiEventType::SettingsDefaultBrandingRestored,
            })?;
            Ok(ApiResponse {
                json: json!(settings),
                status: StatusCode::OK,
            })
        }
        None => Err(WebError::DbError("Cannot restore settings".into())),
    }
}

pub async fn patch_settings(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<SettingsPatch>,
) -> ApiResult {
    debug!(
        "Admin {} patching settings with {data:?}",
        session.user.username
    );
    let mut settings = Settings::get_current_settings();

    // Handle updating the cached license
    if let Some(license_key) = &data.license {
        update_cached_license(license_key.as_deref())?;
        debug!("Saving the new license key to the database as part of the settings patch");
    }

    if let Some(ldap_enabled) = data.ldap_enabled {
        if !ldap_enabled {
            settings.ldap_sync_status = SyncStatus::OutOfSync;
        }
    }

    if let Some(ldap_authority) = data.ldap_is_authoritative {
        if settings.ldap_is_authoritative != ldap_authority {
            settings.ldap_sync_status = SyncStatus::OutOfSync;
        }
    }

    if let Some(ldap_sync_groups) = &data.ldap_sync_groups {
        if &settings.ldap_sync_groups != ldap_sync_groups {
            settings.ldap_sync_status = SyncStatus::OutOfSync;
        }
    }

    settings.apply(data);
    settings.validate()?;
    update_current_settings(&appstate.pool, settings).await?;

    info!("Admin {} patched settings.", session.user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: ApiEventType::SettingsUpdatedPartial,
    })?;
    Ok(ApiResponse::default())
}

pub async fn test_ldap_settings(_admin: AdminRole) -> ApiResult {
    debug!("Testing LDAP connection");
    match LDAPConnection::create().await {
        Ok(_) => {
            debug!("LDAP connected successfully");
            Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::OK,
            })
        }
        Err(err) => {
            debug!("LDAP connection rejected: {err}");
            Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::BAD_REQUEST,
            })
        }
    }
}
