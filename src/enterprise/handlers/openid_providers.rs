use axum::{extract::State, http::StatusCode, Json};

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::openid_provider::OpenIdProvider,
    handlers::{ApiResponse, ApiResult},
};

use serde_json::json;

#[derive(Debug, Deserialize, Serialize)]
pub struct AddProviderData {
    name: String,
    base_url: String,
    client_id: String,
    client_secret: String,
}

impl AddProviderData {
    pub fn new(name: String, base_url: String, client_id: String, client_secret: String) -> Self {
        Self {
            name,
            base_url,
            client_id,
            client_secret,
        }
    }
}

pub async fn add_openid_provider(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(provider_data): Json<AddProviderData>,
) -> ApiResult {
    let mut new_provider = OpenIdProvider::new(
        provider_data.name,
        provider_data.base_url,
        provider_data.client_id,
        provider_data.client_secret,
    );
    debug!(
        "User {} adding OpenID provider {}",
        session.user.username, new_provider.name
    );
    // Currently, we only support one OpenID provider at a time
    new_provider.upsert(&appstate.pool).await?;
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
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    match OpenIdProvider::get_current(&appstate.pool).await? {
        Some(provider) => Ok(ApiResponse {
            json: json!(provider),
            status: StatusCode::OK,
        }),
        None => Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        }),
    }
}

pub async fn delete_openid_provider(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(provider_data): Json<AddProviderData>,
) -> ApiResult {
    debug!(
        "User {} deleting OpenID provider {}",
        session.user.username, provider_data.name
    );
    let provider = OpenIdProvider::find_by_name(&appstate.pool, &provider_data.name).await?;
    if let Some(provider) = provider {
        provider.delete(&appstate.pool).await?;
        info!(
            "User {} deleted OpenID client {}",
            session.user.username, provider_data.name
        );
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::OK,
        })
    } else {
        warn!(
            "User {} failed to delete OpenID client {}. Such client does not exist.",
            session.user.username, provider_data.name
        );
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        })
    }
}

pub async fn modify_openid_provider(
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
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let providers = OpenIdProvider::all(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(providers),
        status: StatusCode::OK,
    })
}