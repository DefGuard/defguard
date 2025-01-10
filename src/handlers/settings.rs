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
    enterprise::license::update_cached_license,
    error::WebError,
    ldap::LDAPConnection,
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
    State(appstate): State<AppState>,
    Json(data): Json<Settings>,
) -> ApiResult {
    debug!("User {} updating settings", session.user.username);

    update_cached_license(data.license.as_deref())?;
    data.validate()?;
    update_current_settings(&appstate.pool, data).await?;

    info!("User {} updated settings", session.user.username);

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
    Json(data): Json<SettingsPatch>,
) -> ApiResult {
    debug!("Admin {} patching settings.", session.user.username);
    let mut settings = Settings::get_current_settings();

    // Handle updating the cached license
    if let Some(license_key) = &data.license {
        update_cached_license(license_key.as_deref())?;
        debug!("Saving the new license key to the database as part of the settings patch");
    };

    settings.apply(data);
    settings.validate()?;
    update_current_settings(&appstate.pool, settings).await?;

    info!("Admin {} patched settings.", session.user.username);
    Ok(ApiResponse::default())
}

pub async fn test_ldap_settings(_admin: AdminRole) -> ApiResult {
    debug!("Testing LDAP connection");
    if LDAPConnection::create().await.is_ok() {
        debug!("LDAP connected successfully");
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::OK,
        })
    } else {
        debug!("LDAP connection rejected");
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        })
    }
}
