use axum::{
    Extension,
    extract::{Json, Path, State},
    http::StatusCode,
};
use defguard_common::{
    db::{
        Id,
        models::{
            Settings, SettingsEssentials,
            settings::{LdapSyncStatus, SettingsPatch, update_current_settings},
        },
    },
    types::proxy::ProxyControlMessage,
};
use sqlx::PgPool;
use struct_patch::Patch;

use super::{ApiErrorResponse, ApiResponse, ApiResponseCode, ApiResult};
use crate::{
    AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::enterprise_settings::EnterpriseSettings,
        handlers::LicenseInfo,
        ldap::{LDAPConnection, sync::Authority},
        license::{
            License, LicenseTier, get_cached_license, update_cached_license, validate_license,
        },
        limits::{Counts, get_counts},
    },
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

    // If SMTP configuration or the public proxy URL changed, push updated
    // public settings to all connected proxies.
    if before.edge_public_settings_changed(&after)
        && let Ok(enterprise_settings) = EnterpriseSettings::get(&appstate.pool).await
    {
        let display_password_reset = enterprise_settings.edge_can_display_password_reset();
        if let Err(err) = appstate
            .proxy_control_tx
            .send(ProxyControlMessage::BroadcastPublicSettings {
                display_password_reset,
                display_download_step: enterprise_settings.display_download_step,
                public_url: after.configured_public_proxy_url(),
            })
            .await
        {
            error!("Failed to broadcast PublicSettings after settings change: {err:?}");
        }
    }

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

fn is_license_reactivation(
    current_license: Option<&License>,
    new_license: &License,
    counts: &Counts,
) -> bool {
    let Some(current_license) = current_license else {
        return false;
    };
    let current_license_invalid =
        validate_license(Some(current_license), counts, LicenseTier::Business).is_err();
    let new_license_valid =
        validate_license(Some(new_license), counts, LicenseTier::Business).is_ok();
    current_license_invalid && new_license_valid
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
        (status = 200, description = "Settings updated. The body carries a `license_reactivated` code when an invalid license has been replaced with a valid one.", body = Object, example = json!({"code": "license_reactivated"})),
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
    let mut license_reactivated = false;
    if let Some(license_key) = &license {
        if let Some(new_key) = license_key.as_deref()
            && let Ok(new_license) = License::from_base64(new_key)
        {
            let counts = get_counts();
            license_reactivated =
                is_license_reactivation(get_cached_license().as_ref(), &new_license, &counts);
            if license_reactivated {
                info!(
                    "Admin {} replaced a previously invalid license with a valid one",
                    session.user.username
                );
            }
        } else {
            info!("Couldn't obtain current license");
        }

        update_cached_license(license_key.as_deref())?;
        debug!("Updated cached license after saving settings patch");
    }

    // If SMTP configuration or the public proxy URL changed, push updated
    // public settings to all connected proxies.
    if before.edge_public_settings_changed(&after)
        && let Ok(enterprise_settings) = EnterpriseSettings::get(&appstate.pool).await
    {
        let display_password_reset = enterprise_settings.edge_can_display_password_reset();
        if let Err(err) = appstate
            .proxy_control_tx
            .send(ProxyControlMessage::BroadcastPublicSettings {
                display_password_reset,
                display_download_step: enterprise_settings.display_download_step,
                public_url: after.configured_public_proxy_url(),
            })
            .await
        {
            error!("Failed to broadcast PublicSettings after settings change: {err:?}");
        }
    }

    info!("Admin {} patched settings", session.user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::SettingsUpdatedPartial { before, after }),
    })?;

    if license_reactivated {
        Ok(ApiResponseCode::LicenseReactivated.into())
    } else {
        Ok(ApiResponse::default())
    }
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

/// Test the LDAP connection
///
/// Uses the settings from the request body, which do not have to be saved yet.
#[utoipa::path(
    post,
    path = "/api/v1/ldap/test",
    tag = "LDAP",
    request_body = Object,
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

/// Preview the changes a full LDAP sync would make
///
/// Uses the settings from the request body, which do not have to be saved yet. Read-only:
/// nothing is imported, removed or persisted.
#[utoipa::path(
    post,
    path = "/api/v1/ldap/dry_run",
    tag = "LDAP",
    request_body = Object,
    responses(
        (status = 200, description = "Dry run result.", body = Object, example = json!({
            "defguard": [{"username": "jane", "email": "jane@example.com", "first_name": "Jane", "last_name": "Doe", "action": "add"}],
            "ldap": [{"username": "john", "email": "john@example.com", "first_name": "John", "last_name": "Doe", "action": "remove"}]
        })),
        (status = 400, description = "Unable to connect to LDAP or to perform the dry run."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to perform LDAP dry run.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
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

#[cfg(test)]
mod tests {
    use chrono::{TimeDelta, Utc};

    use super::is_license_reactivation;
    use crate::enterprise::{
        license::{License, LicenseTier, SupportType},
        limits::Counts,
    };

    fn license(subscription: bool, valid_until_days: i64) -> License {
        License::new(
            "test-customer".into(),
            subscription,
            Some(Utc::now() + TimeDelta::days(valid_until_days)),
            None,
            None,
            LicenseTier::Business,
            SupportType::Basic,
            Vec::new(),
        )
    }

    #[test]
    fn reactivation_past_grace_subscription_replaced_by_valid() {
        assert!(is_license_reactivation(
            Some(&license(true, -20)),
            &license(true, 365),
            &Counts::default()
        ));
    }

    #[test]
    fn reactivation_expired_non_subscription_replaced_by_valid() {
        assert!(is_license_reactivation(
            Some(&license(false, -1)),
            &license(true, 365),
            &Counts::default()
        ));
    }

    #[test]
    fn reactivation_when_new_subscription_still_within_grace() {
        assert!(is_license_reactivation(
            Some(&license(true, -20)),
            &license(true, -5),
            &Counts::default()
        ));
    }

    #[test]
    fn no_reactivation_when_current_subscription_within_grace() {
        assert!(!is_license_reactivation(
            Some(&license(true, -5)),
            &license(true, 365),
            &Counts::default()
        ));
    }

    #[test]
    fn no_reactivation_when_current_still_valid() {
        assert!(!is_license_reactivation(
            Some(&license(true, 365)),
            &license(true, 365),
            &Counts::default()
        ));
    }

    #[test]
    fn no_reactivation_when_new_license_also_unusable() {
        assert!(!is_license_reactivation(
            Some(&license(true, -20)),
            &license(true, -20),
            &Counts::default()
        ));
    }

    #[test]
    fn no_reactivation_without_current_license() {
        assert!(!is_license_reactivation(
            None,
            &license(true, 365),
            &Counts::default()
        ));
    }
}
