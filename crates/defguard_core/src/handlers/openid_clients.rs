use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use defguard_common::{
    db::{
        NoId,
        models::oauth2client::{OAuth2Client, OAuth2ClientSafe},
    },
    random::gen_alphanumeric,
};
use serde_json::json;

use super::{ApiResponse, ApiResult, webhooks::ChangeStateData};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    events::{ApiEvent, ApiEventType, ApiRequestContext},
};

#[derive(Deserialize, Serialize)]
pub struct NewOpenIDClient {
    pub name: String,
    pub redirect_uri: Vec<String>,
    pub scope: Vec<String>,
    pub enabled: bool,
}

impl Into<OAuth2Client<NoId>> for NewOpenIDClient {
    fn into(self) -> OAuth2Client<NoId> {
        let client_id = gen_alphanumeric(16);
        let client_secret = gen_alphanumeric(32);
        OAuth2Client {
            id: NoId,
            client_id,
            client_secret,
            redirect_uri: self.redirect_uri,
            scope: self.scope,
            name: self.name,
            enabled: self.enabled,
        }
    }
}

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
    if ammonia::is_html(&data.name) {
        warn!(
            "User {} attempted to create openid client with name containing HTML: {}",
            session.user.username, data.name
        );
        return Ok(ApiResponse {
            json: json!({"msg": "invalid name"}),
            status: StatusCode::BAD_REQUEST,
        });
    }
    let client: OAuth2Client = data.into();
    let client = client.save(&appstate.pool).await?;
    info!(
        "User {} added OpenID client {}",
        session.user.username, client.name
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::OpenIdAppAdded {
            app: client.clone(),
        }),
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
    if ammonia::is_html(&data.name) {
        warn!(
            "User {} attempted to edit openid client with name containing HTML: {}",
            session.user.username, data.name
        );
        return Ok(ApiResponse {
            json: json!({"msg": "invalid name"}),
            status: StatusCode::BAD_REQUEST,
        });
    }
    let mut transaction = appstate.pool.begin().await?;
    let status = match OAuth2Client::find_by_client_id(&mut *transaction, &client_id).await? {
        Some(mut client) => {
            // store client before mods
            let before = client.clone();
            client.name = data.name;
            client.redirect_uri = data.redirect_uri;
            client.enabled = data.enabled;
            client.scope = data.scope;
            client.save(&mut *transaction).await?;
            if before.scope != client.scope {
                client.clear_authorizations(&mut *transaction).await?;
            }
            transaction.commit().await?;
            info!(
                "User {} updated OpenID client {client_id} ({})",
                session.user.username, client.name
            );
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::OpenIdAppModified {
                    before,
                    after: client,
                }),
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
                event: Box::new(ApiEventType::OpenIdAppStateChanged {
                    enabled: client.enabled,
                    app: client,
                }),
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
                event: Box::new(ApiEventType::OpenIdAppRemoved { app: client }),
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
