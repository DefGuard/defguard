use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use defguard_common::db::models::{
    Settings, WireguardNetwork,
    settings::{OpenIdUsernameHandling, update_current_settings},
    wireguard::LocationMfaMode,
};
use rsa::{RsaPrivateKey, pkcs8::DecodePrivateKey};
use serde_json::json;
use utoipa::ToSchema;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::openid_provider::{OpenIdProvider, OpenIdProviderKind},
        directory_sync::test_directory_sync_connection,
    },
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
};

#[derive(Deserialize, Serialize, ToSchema)]
pub struct AddProviderData {
    pub name: String,
    pub base_url: String,
    pub kind: OpenIdProviderKind,
    pub client_id: String,
    pub client_secret: String,
    pub display_name: Option<String>,
    pub admin_email: Option<String>,
    pub google_service_account_email: Option<String>,
    pub google_service_account_key: Option<String>,
    pub directory_sync_enabled: bool,
    pub directory_sync_interval: i32,
    pub directory_sync_user_behavior: String,
    pub directory_sync_admin_behavior: String,
    pub directory_sync_target: String,
    pub okta_private_jwk: Option<String>,
    pub okta_dirsync_client_id: Option<String>,
    pub directory_sync_group_match: Option<String>,
    pub jumpcloud_api_key: Option<String>,
    pub prefetch_users: bool,
    // Core settings
    pub create_account: bool,
    pub username_handling: OpenIdUsernameHandling,
}

/// Add OpenID provider.
///
/// # Returns
/// - HTTP Status "created" on success.
#[utoipa::path(
    post,
    path = "/api/v1/openid/provider",
    tag = "OpenID",
    params(
        ("data" = AddProviderData, Path, description = "OpenID provider data",)
    ),
    responses(
        (status = CREATED, description = "Add OpenID provider"),
    ),
)]
pub(crate) async fn add_openid_provider(
    _license: LicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(provider_data): Json<AddProviderData>,
) -> ApiResult {
    debug!(
        "User {} adding OpenID provider {}",
        session.user.username, provider_data.name
    );
    let current_provider = OpenIdProvider::get_current(&appstate.pool).await?;

    // The key is sent from the frontend only when user explicitly changes it, as we never send it
    // back. Check if the thing received from the frontend is a valid RSA private key (signaling
    // user intent to change key) or is it just some empty string or other junk.
    let private_key = match &provider_data.google_service_account_key {
        Some(key) => {
            if RsaPrivateKey::from_pkcs8_pem(key).is_ok() {
                debug!(
                    "User {} provided a valid RSA private key for provider's directory sync. Using \
                    it.",
                    session.user.username
                );
                provider_data.google_service_account_key.clone()
            } else if let Some(provider) = &current_provider {
                debug!(
                    "User {} did not provide a valid RSA private key for provider's directory sync \
                    or the key did not change. Using the existing key",
                    session.user.username
                );
                provider.google_service_account_key.clone()
            } else {
                warn!(
                    "User {} did not provide a valid RSA private key for provider's directory \
                    sync.",
                    session.user.username
                );
                None
            }
        }
        None => None,
    };

    let okta_private_jwk = match &provider_data.okta_private_jwk {
        Some(key) => {
            if serde_json::from_str::<serde_json::Value>(key).is_ok() {
                debug!(
                    "User {} provided a valid JWK private key for provider's Okta directory sync. \
                    Using it.",
                    session.user.username
                );
                provider_data.okta_private_jwk.clone()
            } else if let Some(provider) = &current_provider {
                debug!(
                    "User {} did not provide a valid JWK private key for provider's Okta directory \
                    sync or the key did not change. Using the existing key.",
                    session.user.username
                );
                provider.okta_private_jwk.clone()
            } else {
                warn!(
                    "User {} did not provide a valid JWK private key for provider's Okta directory \
                    sync.",
                    session.user.username
                );
                None
            }
        }
        None => None,
    };

    let mut settings = Settings::get_current_settings();
    settings.openid_create_account = provider_data.create_account;
    settings.openid_username_handling = provider_data.username_handling;
    update_current_settings(&appstate.pool, settings).await?;

    let group_match = if let Some(group_match) = provider_data.directory_sync_group_match {
        if group_match.is_empty() {
            Vec::new()
        } else {
            group_match
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        }
    } else {
        Vec::new()
    };

    // Currently, we only support one OpenID provider at a time
    let new_provider = OpenIdProvider::new(
        provider_data.name,
        provider_data.base_url,
        provider_data.kind,
        provider_data.client_id,
        provider_data.client_secret,
        provider_data.display_name,
        private_key,
        provider_data.google_service_account_email,
        provider_data.admin_email,
        provider_data.directory_sync_enabled,
        provider_data.directory_sync_interval,
        provider_data.directory_sync_user_behavior.into(),
        provider_data.directory_sync_admin_behavior.into(),
        provider_data.directory_sync_target.into(),
        okta_private_jwk,
        provider_data.okta_dirsync_client_id,
        group_match,
        provider_data.jumpcloud_api_key,
        provider_data.prefetch_users,
    )
    .upsert(&appstate.pool)
    .await?;
    info!(
        "User {} added OpenID client {}",
        session.user.username, new_provider.name
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::OpenIdProviderModified {
            provider: new_provider,
        }),
    })?;

    Ok(ApiResponse::with_status(StatusCode::CREATED))
}

/// Get OpenID provider by name.
///
/// # Returns
/// - HTTP Status "OK" on success.
#[utoipa::path(
    get,
    path = "/api/v1/openid/provider/{name}",
    tag = "OpenID",
    responses(
        (status = OK, description = "Get OpenID provider"),
    ),
    params(
        ("name" = String, Path, description = "The name of a provider",)
    )
)]
pub(crate) async fn get_openid_provider(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult {
    let settings = Settings::get_current_settings();
    let settings_json = json!({"create_account": settings.openid_create_account,
        "username_handling": settings.openid_username_handling});
    match OpenIdProvider::find_by_name(&appstate.pool, &name).await? {
        Some(mut provider) => {
            // Get rid of it, it should stay on the backend only.
            provider.google_service_account_key = None;
            provider.okta_private_jwk = None;
            Ok(ApiResponse::new(
                json!({"provider": provider, "settings": settings_json}),
                StatusCode::OK,
            ))
        }
        None => Ok(ApiResponse::new(
            json!({"provider": null, "settings": settings_json}),
            StatusCode::NO_CONTENT,
        )),
    }
}

/// Delete OpenID provider.
///
/// # Returns
/// - HTTP Status "OK" on success.
#[utoipa::path(
    delete,
    path = "/api/v1/openid/provider/{name}",
    tag = "OpenID",
    responses(
        (status = OK, description = "Delete OpenID provider"),
    ),
    params(
        ("name" = String, Path, description = "The name of a provider",)
    )
)]
pub(crate) async fn delete_openid_provider(
    _license: LicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
) -> ApiResult {
    debug!(
        "User {} deleting OpenID provider {name}",
        session.user.username
    );
    let mut transaction = appstate.pool.begin().await?;
    let provider = OpenIdProvider::find_by_name(&mut *transaction, &name).await?;
    if let Some(provider) = provider {
        provider.clone().delete(&mut *transaction).await?;
        // fetch all locations using external MFA
        let locations = WireguardNetwork::all_using_external_mfa(&mut *transaction).await?;
        if locations.is_empty() {
            debug!("No locations are using OIDC provider for external MFA");
        }
        // fall back to internal MFA in all relevant locations
        for mut location in locations {
            debug!(
                "Falling back to internal MFA for {location} because exteral OIDC provider has \
                been removed"
            );
            location.location_mfa_mode = LocationMfaMode::Internal;
            location.save(&mut *transaction).await?;
        }
        transaction.commit().await?;
        info!(
            "User {} deleted OpenID provider {}",
            session.user.username, provider.name
        );
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::OpenIdProviderRemoved { provider }),
        })?;
        Ok(ApiResponse::with_status(StatusCode::OK))
    } else {
        warn!(
            "User {} failed to delete OpenID provider {name}. Such provider does not exist.",
            session.user.username,
        );
        Ok(ApiResponse::with_status(StatusCode::NOT_FOUND))
    }
}

/// Modify OpenID provider.
///
/// # Returns
/// - HTTP Status "OK" on success.
#[utoipa::path(
    put,
    path = "/api/v1/openid/provider/{name}",
    tag = "OpenID",
    responses(
        (status = OK, description = "Modify OpenID provider"),
    ),
    params(
        ("name" = String, Path, description = "The name of a provider",)
    )
)]
pub(crate) async fn modify_openid_provider(
    _license: LicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(provider_data): Json<AddProviderData>,
) -> ApiResult {
    debug!(
        "User {} modifying OpenID provider {}",
        session.user.username, provider_data.name
    );
    let mut transaction = appstate.pool.begin().await?;
    let provider = OpenIdProvider::find_by_name(&mut *transaction, &provider_data.name).await?;
    if let Some(mut provider) = provider {
        provider.base_url = provider_data.base_url;
        provider.kind = provider_data.kind;
        provider.client_id = provider_data.client_id;
        provider.client_secret = provider_data.client_secret;
        provider.save(&mut *transaction).await?;
        info!(
            "User {} modified OpenID client {}",
            session.user.username, provider.name
        );
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::OpenIdProviderModified { provider }),
        })?;

        Ok(ApiResponse::with_status(StatusCode::OK))
    } else {
        warn!(
            "User {} failed to modify OpenID client {}. Such client does not exist.",
            session.user.username, provider_data.name
        );
        Ok(ApiResponse::with_status(StatusCode::NOT_FOUND))
    }
}

/// List all OpenID providers.
///
/// # Returns
/// - Array of all OpenID providers and HTTP status "OK" on success.
#[utoipa::path(
    get,
    path = "/api/v1/openid/provider",
    tag = "OpenID",
    responses(
        (status = OK, description = "List of OpenID providers"),
    ),
)]
pub(crate) async fn list_openid_providers(
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let providers = OpenIdProvider::all(&appstate.pool).await?;
    Ok(ApiResponse::json(providers, StatusCode::OK))
}

pub(crate) async fn test_dirsync_connection(
    _license: LicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} testing directory sync connection",
        session.user.username
    );

    if let Err(err) = test_directory_sync_connection(&appstate.pool).await {
        error!(
            "User {} tested directory sync connection, the connection failed: {err}",
            session.user.username,
        );
        return Ok(ApiResponse::new(
            json!({"message": err.to_string(), "success": false}),
            StatusCode::OK,
        ));
    }
    debug!(
        "User {} tested directory sync connection, the connection was successful",
        session.user.username
    );
    Ok(ApiResponse::new(
        json!({"message": "Connection successful", "success": true}),
        StatusCode::OK,
    ))
}
