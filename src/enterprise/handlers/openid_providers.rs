use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use rsa::{pkcs8::DecodePrivateKey, RsaPrivateKey};
use serde_json::json;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::openid_provider::OpenIdProvider,
    handlers::{ApiResponse, ApiResult},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct AddProviderData {
    name: String,
    base_url: String,
    client_id: String,
    client_secret: String,
    display_name: Option<String>,
    admin_email: Option<String>,
    google_service_account_email: Option<String>,
    google_service_account_key: Option<String>,
    directory_sync_enabled: bool,
    directory_sync_interval: i32,
    directory_sync_user_behavior: String,
    directory_sync_admin_behavior: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteProviderData {
    name: String,
}

// impl AddProviderData {
//     #[must_use]
//     pub(crate) fn new(
//         name: &str,
//         base_url: &str,
//         client_id: &str,
//         client_secret: &str,
//         display_name: Option<&str>,
//         admin_email: Option<&str>,
//         google_service_account_email: Option<&str>,
//         google_service_account_key: Option<&str>,
//         directory_sync_enabled: bool,
//         directory_sync_interval: i32,
//         directory_sync_user_behavior: &str,
//     ) -> Self {
//         Self {
//             name: name.into(),
//             base_url: base_url.into(),
//             client_id: client_id.into(),
//             client_secret: client_secret.into(),
//             display_name: display_name.map(|s| s.into()),
//             admin_email: admin_email.map(|s| s.into()),
//             google_service_account_email: google_service_account_email.map(|s| s.into()),
//             google_service_account_key: google_service_account_key.map(|s| s.into()),
//             directory_sync_enabled,
//             directory_sync_interval,
//             directory_sync_user_behavior: directory_sync_user_behavior.into(),
//         }
//     }
// }

pub async fn add_openid_provider(
    _license: LicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(provider_data): Json<AddProviderData>,
) -> ApiResult {
    let current_provider = OpenIdProvider::get_current(&appstate.pool).await?;

    // The key is sent from the frontend only when user explicitly changes it, as we never send it back.
    // Check if the thing received from the frontend is a valid RSA private key (signaling user intent to change key)
    // or is it just some empty string or other junk.
    let private_key = match &provider_data.google_service_account_key {
        Some(key) => {
            if RsaPrivateKey::from_pkcs8_pem(key).is_ok() {
                debug!(
                    "User {} provided a valid RSA private key for provider's directory sync, using it",
                    session.user.username
                );
                provider_data.google_service_account_key.clone()
            } else if let Some(provider) = &current_provider {
                debug!(
                    "User {} did not provide a valid RSA private key for provider's directory sync or the key did not change, using the existing key",
                    session.user.username
                );
                provider.google_service_account_key.clone()
            } else {
                warn!(
                    "User {} did not provide a valid RSA private key for provider's directory sync",
                    session.user.username
                );
                None
            }
        }
        None => None,
    };

    // Currently, we only support one OpenID provider at a time
    let new_provider = OpenIdProvider::new(
        provider_data.name,
        provider_data.base_url,
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
    )
    .upsert(&appstate.pool)
    .await?;
    debug!(
        "User {} adding OpenID provider {}",
        session.user.username, new_provider.name
    );
    info!(
        "User {} added OpenID client {}",
        session.user.username, new_provider.name
    );

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}

pub async fn get_current_openid_provider(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    match OpenIdProvider::get_current(&appstate.pool).await? {
        Some(mut provider) => {
            // Get rid of it, it should stay on the backend only.
            provider.google_service_account_key = None;
            Ok(ApiResponse {
                json: json!(provider),
                status: StatusCode::OK,
            })
        }
        None => Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        }),
    }
}

pub async fn delete_openid_provider(
    _license: LicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(provider_data): Path<DeleteProviderData>,
) -> ApiResult {
    debug!(
        "User {} deleting OpenID provider {}",
        session.user.username, provider_data.name
    );
    let provider = OpenIdProvider::find_by_name(&appstate.pool, &provider_data.name).await?;
    if let Some(provider) = provider {
        provider.delete(&appstate.pool).await?;
        info!(
            "User {} deleted OpenID provider {}",
            session.user.username, provider_data.name
        );
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::OK,
        })
    } else {
        warn!(
            "User {} failed to delete OpenID provider {}. Such provider does not exist.",
            session.user.username, provider_data.name
        );
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        })
    }
}

pub async fn modify_openid_provider(
    _license: LicenseInfo,
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(provider_data): Json<AddProviderData>,
) -> ApiResult {
    debug!(
        "User {} modifying OpenID provider {}",
        session.user.username, provider_data.name
    );
    let provider = OpenIdProvider::find_by_name(&appstate.pool, &provider_data.name).await?;
    if let Some(mut provider) = provider {
        provider.base_url = provider_data.base_url;
        provider.client_id = provider_data.client_id;
        provider.client_secret = provider_data.client_secret;
        provider.save(&appstate.pool).await?;
        info!(
            "User {} modified OpenID client {}",
            session.user.username, provider.name
        );
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::OK,
        })
    } else {
        warn!(
            "User {} failed to modify OpenID client {}. Such client does not exist.",
            session.user.username, provider_data.name
        );
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        })
    }
}

pub async fn list_openid_providers(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let providers = OpenIdProvider::all(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(providers),
        status: StatusCode::OK,
    })
}
