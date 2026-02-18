use axum::{
    Extension,
    extract::{Json, Path, State},
    http::StatusCode,
};
use defguard_common::db::models::{
    Settings, SettingsEssentials,
    settings::{LdapSyncStatus, SettingsPatch, update_current_settings},
};
use sqlx::PgPool;
use struct_patch::Patch;

use super::{ApiResponse, ApiResult};
use crate::{
    AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{handlers::LicenseInfo, ldap::LDAPConnection, license::update_cached_license},
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
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
        return Ok(ApiResponse::json(settings, StatusCode::OK));
    }
    debug!("Retrieved settings");
    Ok(ApiResponse::default())
}

pub async fn update_settings(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(mut data): Json<Settings>,
) -> ApiResult {
    debug!("User {} updating settings", session.user.username);

    // fetch current settings for event
    let before = Settings::get_current_settings();

    update_cached_license(data.license.as_deref())?;
    data.uuid = before.uuid;
    data.validate()?;
    // clone for event
    let after = data.clone();

    update_current_settings(&appstate.pool, data).await?;

    info!("User {} updated settings", session.user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::SettingsUpdated { before, after }),
    })?;

    Ok(ApiResponse::default())
}

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
                event: Box::new(ApiEventType::SettingsDefaultBrandingRestored),
            })?;
            Ok(ApiResponse::json(settings, StatusCode::OK))
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
    debug!("Admin {} patching settings", session.user.username);
    let mut settings = Settings::get_current_settings();
    // prepare clone for emitting an event
    let before = settings.clone();

    // Handle updating the cached license
    if let Some(license_key) = &data.license {
        update_cached_license(license_key.as_deref())?;
        debug!("Saving the new license key to the database as part of the settings patch");
    }

    if let Some(ldap_enabled) = data.ldap_enabled {
        if !ldap_enabled {
            settings.ldap_sync_status = LdapSyncStatus::OutOfSync;
        }
    }

    if let Some(ldap_authority) = data.ldap_is_authoritative {
        if settings.ldap_is_authoritative != ldap_authority {
            settings.ldap_sync_status = LdapSyncStatus::OutOfSync;
        }
    }

    if let Some(ldap_sync_groups) = &data.ldap_sync_groups {
        if &settings.ldap_sync_groups != ldap_sync_groups {
            settings.ldap_sync_status = LdapSyncStatus::OutOfSync;
        }
    }

    settings.apply(data);
    settings.validate()?;
    // clone for event
    let after = settings.clone();
    update_current_settings(&appstate.pool, settings).await?;

    info!("Admin {} patched settings.", session.user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::SettingsUpdatedPartial { before, after }),
    })?;
    Ok(ApiResponse::default())
}

pub async fn test_ldap_settings(_admin: AdminRole, _license: LicenseInfo) -> ApiResult {
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
