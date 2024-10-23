use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::{webhooks::ChangeStateData, ApiResponse};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::models::{
        oauth2client::{OAuth2Client, OAuth2ClientSafe},
        NewOpenIDClient,
    },
    error::WebError,
};

pub async fn add_openid_client(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<NewOpenIDClient>,
) -> Result<ApiResponse, WebError> {
    let client = OAuth2Client::from_new(data).save(&appstate.pool).await?;
    debug!(
        "User {} adding OpenID client {}",
        session.user.username, client.name
    );
    info!(
        "User {} added OpenID client {}",
        session.user.username, client.name
    );
    Ok(ApiResponse {
        json: json!(client),
        status: StatusCode::CREATED,
    })
}

pub async fn list_openid_clients(
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> Result<ApiResponse, WebError> {
    let openid_clients = OAuth2Client::all(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(openid_clients),
        status: StatusCode::OK,
    })
}

pub async fn get_openid_client(
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
    session: SessionInfo,
) -> Result<ApiResponse, WebError> {
    match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(openid_client) => {
            if session.is_admin {
                Ok(ApiResponse {
                    json: json!(openid_client),
                    status: StatusCode::OK,
                })
            } else {
                Ok(ApiResponse {
                    json: json!(OAuth2ClientSafe::from(openid_client)),
                    status: StatusCode::OK,
                })
            }
        }
        None => Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        }),
    }
}

pub async fn change_openid_client(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
    Json(data): Json<NewOpenIDClient>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} updating OpenID client {client_id}...",
        session.user.username
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(mut openid_client) => {
            openid_client.name = data.name;
            openid_client.redirect_uri = data.redirect_uri;
            openid_client.enabled = data.enabled;
            openid_client.scope = data.scope;
            openid_client.save(&appstate.pool).await?;
            info!(
                "User {} updated OpenID client {client_id} ({})",
                session.user.username, openid_client.name
            );
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

pub async fn change_openid_client_state(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
    Json(data): Json<ChangeStateData>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} updating OpenID client {client_id} enabled state",
        session.user.username
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(mut openid_client) => {
            openid_client.enabled = data.enabled;
            openid_client.save(&appstate.pool).await?;
            info!(
                "User {} updated OpenID client {client_id} ({}) enabled state to {}",
                session.user.username, openid_client.name, openid_client.enabled,
            );
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

pub async fn delete_openid_client(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} deleting OpenID client {client_id}",
        session.user.username
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(openid_client) => {
            openid_client.delete(&appstate.pool).await?;
            info!(
                "User {} deleted OpenID client {client_id}",
                session.user.username
            );
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}
