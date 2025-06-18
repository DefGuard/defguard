use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::{webhooks::ChangeStateData, ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::models::{
        oauth2client::{OAuth2Client, OAuth2ClientSafe},
        NewOpenIDClient,
    },
    events::{ApiEvent, ApiEventType, ApiRequestContext},
};

pub async fn add_openid_client(
    _admin: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<NewOpenIDClient>,
) -> ApiResult {
    debug!(
        "User {} adding OpenID client {}",
        session.user.username, data.name
    );
    let client = OAuth2Client::from_new(data).save(&appstate.pool).await?;
    info!(
        "User {} added OpenID client {}",
        session.user.username, client.name
    );
    appstate.emit_event(ApiEvent {
        context,
        event: ApiEventType::OpenIdAppAdded {
            app: client.clone(),
        },
    })?;
    Ok(ApiResponse {
        json: json!(client),
        status: StatusCode::CREATED,
    })
}

pub async fn list_openid_clients(_admin: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    let clients = OAuth2Client::all(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(clients),
        status: StatusCode::OK,
    })
}

pub async fn get_openid_client(
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
    session: SessionInfo,
) -> ApiResult {
    match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(client) => {
            if session.is_admin {
                Ok(ApiResponse {
                    json: json!(client),
                    status: StatusCode::OK,
                })
            } else {
                Ok(ApiResponse {
                    json: json!(OAuth2ClientSafe::from(client)),
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
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
    Json(data): Json<NewOpenIDClient>,
) -> ApiResult {
    debug!(
        "User {} updating OpenID client {client_id}...",
        session.user.username
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(mut client) => {
            // store client before mods
            let before = client.clone();
            client.name = data.name;
            client.redirect_uri = data.redirect_uri;
            client.enabled = data.enabled;
            client.scope = data.scope;
            client.save(&appstate.pool).await?;
            info!(
                "User {} updated OpenID client {client_id} ({})",
                session.user.username, client.name
            );
            appstate.emit_event(ApiEvent {
                context,
                event: ApiEventType::OpenIdAppModified {
                    before,
                    after: client,
                },
            })?;
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
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
    Json(data): Json<ChangeStateData>,
) -> ApiResult {
    debug!(
        "User {} updating OpenID client {client_id} enabled state",
        session.user.username
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(mut client) => {
            client.enabled = data.enabled;
            client.save(&appstate.pool).await?;
            info!(
                "User {} updated OpenID client {client_id} ({}) enabled state to {}",
                session.user.username, client.name, client.enabled,
            );
            appstate.emit_event(ApiEvent {
                context,
                event: ApiEventType::OpenIdAppStateChanged {
                    enabled: client.enabled,
                    app: client,
                },
            })?;
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
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
) -> ApiResult {
    debug!(
        "User {} deleting OpenID client {client_id}",
        session.user.username
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(client) => {
            client.clone().delete(&appstate.pool).await?;
            info!(
                "User {} deleted OpenID client {client_id}",
                session.user.username
            );
            appstate.emit_event(ApiEvent {
                context,
                event: ApiEventType::OpenIdAppRemoved { app: client },
            })?;
            StatusCode::OK
        }
        None => StatusCode::NOT_FOUND,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}
