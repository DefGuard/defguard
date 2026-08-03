use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use defguard_common::{
    db::{
        Id, NoId,
        models::oauth2client::{OAuth2Client, OAuth2ClientSafe},
    },
    random::gen_alphanumeric,
};
use serde_json::json;
use utoipa::ToSchema;

use super::{ApiErrorResponse, ApiResponse, ApiResult, webhooks::ChangeStateData};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
};

#[derive(Deserialize, Serialize, ToSchema)]
pub struct NewOpenIDClient {
    pub name: String,
    pub redirect_uri: Vec<String>,
    pub scope: Vec<String>,
    pub enabled: bool,
}

impl From<NewOpenIDClient> for OAuth2Client<NoId> {
    fn from(value: NewOpenIDClient) -> Self {
        let client_id = gen_alphanumeric(16);
        let client_secret = gen_alphanumeric(32);
        Self {
            id: NoId,
            client_id,
            client_secret,
            redirect_uri: value.redirect_uri,
            scope: value.scope,
            name: value.name,
            enabled: value.enabled,
        }
    }
}

/// Create an OAuth2/OpenID client application.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/",
    tag = "OAuth2",
    request_body = NewOpenIDClient,
    responses(
        (status = 201, description = "Client created.", body = Object),
        (status = 400, description = "Invalid client data.", body = ApiErrorResponse, example = json!({"msg": "Invalid redirect URI"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to create client.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn add_openid_client(
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
        return Ok(ApiResponse::new(
            json!({"msg": "invalid name"}),
            StatusCode::BAD_REQUEST,
        ));
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
    Ok(ApiResponse::json(client, StatusCode::CREATED))
}

/// GET: /api/v1/oauth
/// List OAuth2/OpenID client applications.
#[utoipa::path(
    get,
    path = "/api/v1/oauth/",
    tag = "OAuth2",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page, 1-100 (default: 50)"),
    ),
    responses(
        (status = 200, description = "Paginated list of clients.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list clients.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_openid_clients(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
) -> PaginatedApiResult<OAuth2Client<Id>> {
    let pagination = pagination.0;

    debug!("Listing OAuth clients");

    let clients = OAuth2Client::all_paginated(
        &appstate.pool,
        i64::from(pagination.per_page()),
        i64::from(pagination.offset()),
    )
    .await?;

    debug!("Listed OAuth clients");

    let count = OAuth2Client::count(&appstate.pool).await?;
    Ok(PaginatedApiResponse::new(clients, pagination, count as u32))
}

/// Get an OAuth2/OpenID client application.
///
/// Non-admin users receive a reduced representation without the client secret.
#[utoipa::path(
    get,
    path = "/api/v1/oauth/{client_id}",
    tag = "OAuth2",
    params(
        ("client_id" = String, Path, description = "OAuth2 client ID"),
    ),
    responses(
        (status = 200, description = "Client details.", body = Object),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Client not found.", body = ApiErrorResponse, example = json!({"msg": "client not found"})),
        (status = 500, description = "Unable to get client.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn get_openid_client(
    State(appstate): State<AppState>,
    Path(client_id): Path<String>,
    session: SessionInfo,
) -> ApiResult {
    match OAuth2Client::find_by_client_id(&appstate.pool, &client_id).await? {
        Some(client) => {
            if session.is_admin {
                Ok(ApiResponse::json(client, StatusCode::OK))
            } else {
                Ok(ApiResponse::json(
                    OAuth2ClientSafe::from(client),
                    StatusCode::OK,
                ))
            }
        }
        None => Ok(ApiResponse::with_status(StatusCode::NOT_FOUND)),
    }
}

/// Update an OAuth2/OpenID client application.
#[utoipa::path(
    put,
    path = "/api/v1/oauth/{client_id}",
    tag = "OAuth2",
    request_body = NewOpenIDClient,
    params(
        ("client_id" = String, Path, description = "OAuth2 client ID"),
    ),
    responses(
        (status = 200, description = "Client updated.", body = Object, example = json!({})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Client not found.", body = ApiErrorResponse, example = json!({"msg": "client not found"})),
        (status = 500, description = "Unable to update client.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn change_openid_client(
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
        return Ok(ApiResponse::new(
            json!({"msg": "invalid name"}),
            StatusCode::BAD_REQUEST,
        ));
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
    Ok(ApiResponse::with_status(status))
}

/// Enable or disable an OAuth2/OpenID client application.
#[utoipa::path(
    post,
    path = "/api/v1/oauth/{client_id}",
    tag = "OAuth2",
    request_body = ChangeStateData,
    params(
        ("client_id" = String, Path, description = "OAuth2 client ID"),
    ),
    responses(
        (status = 200, description = "Client state changed.", body = Object, example = json!({})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Client not found.", body = ApiErrorResponse, example = json!({"msg": "client not found"})),
        (status = 500, description = "Unable to change client state.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn change_openid_client_state(
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
    Ok(ApiResponse::with_status(status))
}

/// Delete an OAuth2/OpenID client application.
#[utoipa::path(
    delete,
    path = "/api/v1/oauth/{client_id}",
    tag = "OAuth2",
    params(
        ("client_id" = String, Path, description = "OAuth2 client ID"),
    ),
    responses(
        (status = 200, description = "Client deleted.", body = Object, example = json!({})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Client not found.", body = ApiErrorResponse, example = json!({"msg": "client not found"})),
        (status = 500, description = "Unable to delete client.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_openid_client(
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
    Ok(ApiResponse::with_status(status))
}
