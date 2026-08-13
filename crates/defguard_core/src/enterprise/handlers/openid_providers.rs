use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use defguard_common::db::models::{
    Settings, WireguardNetwork,
    settings::{OpenIdUsernameHandling, update_current_settings},
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
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
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
    pub disable_password_management: bool,
    pub directory_sync_user_groups: Option<String>,
    // Core settings
    pub create_account: bool,
    pub username_handling: OpenIdUsernameHandling,
}

/// Create an OpenID provider
#[utoipa::path(
    post,
    path = "/api/v1/openid/provider",
    tag = "OpenID",
    request_body = AddProviderData,
    responses(
        (status = 201, description = "OpenID provider created."),
        (status = 400, description = "Invalid provider configuration.", body = ApiErrorResponse, example = json!({"msg": "Failed to parse Google service account key"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to create OpenID provider.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
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
                .map(|s| s.trim().to_owned())
                .collect()
        }
    } else {
        Vec::new()
    };

    let user_groups = if let Some(user_groups) = provider_data.directory_sync_user_groups {
        if user_groups.is_empty() {
            None
        } else {
            Some(
                user_groups
                    .split(',')
                    .map(|s| s.trim().to_owned())
                    .collect(),
            )
        }
    } else {
        None
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
        provider_data.disable_password_management,
        user_groups,
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

/// Get an OpenID provider
#[utoipa::path(
    get,
    path = "/api/v1/openid/provider/{name}",
    tag = "OpenID",
    responses(
        (status = 200, description = "OpenID provider details.", body = Object, example = json!({"provider": {"name": "google", "base_url": "https://accounts.google.com", "client_id": "client-id"}, "settings": {"create_account": false, "username_handling": "remove_forbidden"}})),
        (status = 204, description = "No OpenID provider with this name."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get OpenID provider.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    params(
        ("name" = String, Path, description = "Name of the OpenID provider.",)
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
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

/// Delete an OpenID provider
///
/// Deletion always proceeds. Any location whose assigned MFA flows still reference OIDC is
/// returned in `affected_locations`: those flows become unsatisfiable, so their users cannot
/// complete MFA until an admin edits them. Callers should surface this as a warning.
#[utoipa::path(
    delete,
    path = "/api/v1/openid/provider/{name}",
    tag = "OpenID",
    responses(
        (status = 200, description = "OpenID provider deleted. `affected_locations` names any location whose MFA flows still reference OIDC and now cannot complete MFA.", body = Object, example = json!({"affected_locations": ["Warsaw office"]})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "OpenID provider not found."),
        (status = 500, description = "Unable to delete OpenID provider.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    params(
        ("name" = String, Path, description = "Name of the OpenID provider.",)
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
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
        // Deleting the provider leaves any flow step referencing OIDC unsatisfiable. Deletion is
        // still allowed to proceed, because removing a provider is frequently incident response
        // (a compromised or decommissioned IdP) and blocking it would keep that provider live for
        // SSO during exactly the window that matters most. The two rejected alternatives were
        // refusing the delete, which impedes revocation, and rewriting the affected flows, which
        // is unsafe because `location_mfa_flow` is many-to-many so one flow can serve several
        // locations and rewriting it would silently change policy for all of them.
        //
        // Access still fails closed: an unsatisfiable step makes connect-time MFA return
        // `failed_precondition`, so affected users are refused rather than let through. The
        // affected locations are returned to the caller so an admin can be warned and repair the
        // flows. The set is deliberately not filtered on `mfa_enabled`: a location with MFA
        // currently switched off would hit the same problem the moment it is re-enabled, so it
        // belongs in the warning.
        let affected = WireguardNetwork::all_with_oidc_in_flows(&mut *transaction).await?;
        let affected_locations: Vec<String> = affected.iter().map(|l| l.name.clone()).collect();

        provider.clone().delete(&mut *transaction).await?;
        transaction.commit().await?;

        if affected_locations.is_empty() {
            info!(
                "User {} deleted OpenID provider {}",
                session.user.username, provider.name
            );
        } else {
            warn!(
                "User {} deleted OpenID provider {}. {} location(s) still reference OIDC in their \
                 MFA flows and their users cannot complete MFA until those flows are edited: {}",
                session.user.username,
                provider.name,
                affected_locations.len(),
                affected_locations.join(", "),
            );
        }

        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::OpenIdProviderRemoved { provider }),
        })?;
        Ok(ApiResponse::new(
            json!({ "affected_locations": affected_locations }),
            StatusCode::OK,
        ))
    } else {
        warn!(
            "User {} failed to delete OpenID provider {name}. Such provider does not exist.",
            session.user.username,
        );
        Ok(ApiResponse::with_status(StatusCode::NOT_FOUND))
    }
}

/// Update an OpenID provider
#[utoipa::path(
    put,
    path = "/api/v1/openid/provider/{name}",
    tag = "OpenID",
    request_body = AddProviderData,
    responses(
        (status = 200, description = "OpenID provider updated."),
        (status = 400, description = "Invalid provider configuration.", body = ApiErrorResponse, example = json!({"msg": "Failed to parse Google service account key"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "OpenID provider not found."),
        (status = 500, description = "Unable to update OpenID provider.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    params(
        ("name" = String, Path, description = "Name of the OpenID provider.",)
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
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
        let private_key = match &provider_data.google_service_account_key {
            Some(key) => {
                if RsaPrivateKey::from_pkcs8_pem(key).is_ok() {
                    debug!(
                        "User {} provided a valid RSA private key for provider's directory sync. Using it.",
                        session.user.username
                    );
                    provider_data.google_service_account_key.clone()
                } else {
                    debug!(
                        "User {} did not provide a valid RSA private key for provider's directory sync or the key did not change. Using the existing key",
                        session.user.username
                    );
                    provider.google_service_account_key.clone()
                }
            }
            None => provider.google_service_account_key.clone(),
        };

        let okta_private_jwk = match &provider_data.okta_private_jwk {
            Some(key) => {
                if serde_json::from_str::<serde_json::Value>(key).is_ok() {
                    debug!(
                        "User {} provided a valid JWK private key for provider's Okta directory sync. Using it.",
                        session.user.username
                    );
                    provider_data.okta_private_jwk.clone()
                } else {
                    debug!(
                        "User {} did not provide a valid JWK private key for provider's Okta directory sync or the key did not change. Using the existing key.",
                        session.user.username
                    );
                    provider.okta_private_jwk.clone()
                }
            }
            None => provider.okta_private_jwk.clone(),
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
                    .map(|s| s.trim().to_owned())
                    .collect()
            }
        } else {
            Vec::new()
        };

        let user_groups = if let Some(user_groups) = provider_data.directory_sync_user_groups {
            if user_groups.is_empty() {
                None
            } else {
                Some(
                    user_groups
                        .split(',')
                        .map(|s| s.trim().to_owned())
                        .collect(),
                )
            }
        } else {
            None
        };

        provider.base_url = provider_data.base_url;
        provider.kind = provider_data.kind;
        provider.client_id = provider_data.client_id;
        provider.client_secret = provider_data.client_secret;
        provider.display_name = provider_data.display_name;
        provider.google_service_account_key = private_key;
        provider.google_service_account_email = provider_data.google_service_account_email;
        provider.admin_email = provider_data.admin_email;
        provider.directory_sync_enabled = provider_data.directory_sync_enabled;
        provider.directory_sync_interval = provider_data.directory_sync_interval;
        provider.directory_sync_user_behavior = provider_data.directory_sync_user_behavior.into();
        provider.directory_sync_admin_behavior = provider_data.directory_sync_admin_behavior.into();
        provider.directory_sync_target = provider_data.directory_sync_target.into();
        provider.okta_private_jwk = okta_private_jwk;
        provider.okta_dirsync_client_id = provider_data.okta_dirsync_client_id;
        provider.directory_sync_group_match = group_match;
        provider.jumpcloud_api_key = provider_data.jumpcloud_api_key;
        provider.prefetch_users = provider_data.prefetch_users;
        provider.directory_sync_user_groups = user_groups;
        provider.save(&mut *transaction).await?;
        transaction.commit().await?;

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

/// List OpenID providers
#[utoipa::path(
    get,
    path = "/api/v1/openid/provider",
    tag = "OpenID",
    responses(
        (status = 200, description = "All OpenID providers.", body = [Object], example = json!([{"id": 1, "name": "google", "base_url": "https://accounts.google.com", "client_id": "client-id"}])),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list OpenID providers.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_openid_providers(
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let providers = OpenIdProvider::all(&appstate.pool).await?;
    Ok(ApiResponse::json(providers, StatusCode::OK))
}

/// Get the current OpenID provider
#[utoipa::path(
    get,
    path = "/api/v1/openid/provider/current",
    tag = "OpenID",
    responses(
        (status = 200, description = "Current OpenID provider details.", body = Object, example = json!({"provider": {"name": "google", "base_url": "https://accounts.google.com", "client_id": "client-id"}, "settings": {"create_account": false, "username_handling": "remove_forbidden"}})),
        (status = 204, description = "No OpenID provider is configured."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to get OpenID provider.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn get_current_openid_provider(
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let settings = Settings::get_current_settings();
    let settings_json = json!({"create_account": settings.openid_create_account,
        "username_handling": settings.openid_username_handling});
    match OpenIdProvider::get_current(&appstate.pool).await? {
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

/// Test the directory sync connection of the current OpenID provider
#[utoipa::path(
    get,
    path = "/api/v1/test_directory_sync",
    tag = "OpenID",
    responses(
        (status = 200, description = "Result of the connection test.", body = Object, example = json!({"message": "Connection successful", "success": true})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to test directory sync connection.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
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
