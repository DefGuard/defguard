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
        models::settings::{SettingsEssentials, SettingsPatch},
        Settings,
    },
    error::WebError,
    ldap::LDAPConnection,
    AppState,
};

pub async fn get_settings(State(appstate): State<AppState>) -> ApiResult {
    debug!("Retrieving settings");
    if let Some(mut settings) = Settings::find_by_id(&appstate.pool, 1).await? {
        if settings.nav_logo_url == "" {
            settings.nav_logo_url = "/svg/defguard-nav-logo.svg".into();
        }
        if settings.main_logo_url == "" {
            settings.main_logo_url = "/svg/logo-defguard-white.svg".into();
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
    Json(mut data): Json<Settings>,
) -> ApiResult {
    debug!("User {} updating settings", session.user.username);
    data.id = Some(1);
    data.save(&appstate.pool).await?;
    info!("User {} updated settings", session.user.username);
    Ok(ApiResponse::default())
}

pub async fn get_settings_essentials(State(appstate): State<AppState>) -> ApiResult {
    debug!("Retrieving essential settings");
    let mut settings = SettingsEssentials::get_settings_essentials(&appstate.pool).await?;
    if settings.nav_logo_url == "" {
        settings.nav_logo_url = "/svg/defguard-nav-logo.svg".into();
    }
    if settings.main_logo_url == "" {
        settings.main_logo_url = "/svg/logo-defguard-white.svg".into();
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
    Path(id): Path<i64>,
    session: SessionInfo,
) -> ApiResult {
    debug!(
        "User {} restoring default branding settings",
        session.user.username
    );
    let settings = Settings::find_by_id(&appstate.pool, id).await?;
    match settings {
        Some(mut settings) => {
            settings.instance_name = "Defguard".into();
            settings.nav_logo_url = "/svg/defguard-nav-logo.svg".into();
            settings.main_logo_url = "/svg/logo-defguard-white.svg".into();
            settings.save(&appstate.pool).await?;
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
    debug!("Admin {} patching settings.", &session.user.username);
    let mut settings = Settings::get_settings(&appstate.pool).await?;
    settings.apply(data);
    settings.save(&appstate.pool).await?;
    info!("Admin {} patched settings.", &session.user.username);
    Ok(ApiResponse::default())
}

pub async fn test_ldap_settings(_admin: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    debug!("Testing LDAP connection");
    if LDAPConnection::create(&appstate.pool).await.is_ok() {
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
