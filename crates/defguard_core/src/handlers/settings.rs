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

use super::{ApiResponse, ApiResult};
use crate::{
    AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        handlers::LicenseInfo,
        ldap::{LDAPConnection, sync::Authority},
        license::update_cached_license,
    },
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

/// Tests the LDAP connection using the provided (not yet saved) settings.
pub(crate) async fn test_submitted_ldap_settings(
    _admin: AdminRole,
    _license: LicenseInfo,
    Json(settings): Json<Settings>,
) -> ApiResult {
    debug!("Testing LDAP connection with provided settings");
    match LDAPConnection::create_with_settings(settings).await {
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

/// Previews the user changes a full LDAP sync would make using the provided (not yet saved)
/// settings. This is strictly read-only: nothing is imported, removed or persisted.
pub(crate) async fn ldap_dry_run(
    _admin: AdminRole,
    _license: LicenseInfo,
    State(appstate): State<AppState>,
    Json(settings): Json<Settings>,
) -> ApiResult {
    debug!("Performing LDAP dry run with provided settings");
    let authority = if settings.ldap_is_authoritative {
        Authority::LDAP
    } else {
        Authority::Defguard
    };

    let mut connection = match LDAPConnection::create_with_settings(settings).await {
        Ok(connection) => connection,
        Err(err) => {
            debug!("LDAP dry run connection rejected: {err}");
            return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
        }
    };

    match connection.dry_run(&appstate.pool, authority).await {
        Ok(result) => {
            debug!("LDAP dry run completed successfully");
            Ok(ApiResponse::json(result, StatusCode::OK))
        }
        Err(err) => {
            debug!("LDAP dry run failed: {err}");
            Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST))
        }
    }
}
