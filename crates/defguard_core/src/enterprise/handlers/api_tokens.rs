use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use defguard_common::{db::models::user::User, random::gen_alphanumeric};
use serde_json::json;
use utoipa::ToSchema;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::api_tokens::{ApiToken, ApiTokenInfo},
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiErrorResponse, ApiResponse, ApiResult, user_for_admin_or_self, validate_name},
};

const API_TOKEN_LENGTH: usize = 32;

#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct AddApiTokenData {
    pub name: String,
}

/// Create an API token for a user
///
/// The token value is returned only in this response and cannot be retrieved later.
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/api_token",
    tag = "API token",
    request_body(content = AddApiTokenData, example = json!({"name": "ci-pipeline"})),
    params(
        ("username" = String, Path, description = "Name of the user."),
    ),
    responses(
        (status = 201, description = "API token created. Its value is returned only here.", body = Object, example = json!({"token": "dg-4vJqXk9wR2mNpL7sT1yZbH3cD8fG5aQe"})),
        (status = 400, description = "Invalid token name.", body = ApiErrorResponse, example = json!({"msg": "Invalid name"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user not found"})),
        (status = 500, description = "Unable to create API token.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn add_api_token(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path(username): Path<String>,
    Json(data): Json<AddApiTokenData>,
) -> ApiResult {
    debug!("Adding API token {:?} for user {username}", data.name);

    // authorize request
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;

    // prevent creating tokens for non-admin users
    if !user.is_admin(&appstate.pool).await? {
        error!(
            "User {} attempted to create API token for non-admin user {username}",
            session.user.username
        );
        return Err(WebError::Forbidden(
            "Cannot create API token for non-admin user",
        ));
    }

    if !user.is_active {
        error!(
            "User {} attempted to create API token for inactive user {username}",
            session.user.username
        );
        return Err(WebError::Forbidden(
            "Cannot create API token for inactive user",
        ));
    }

    // TODO: check if the name is already used

    if !validate_name(&data.name) {
        return Err(WebError::BadRequest(
            "Name contains forbidden characters".into(),
        ));
    }

    // generate token string
    // all API tokens start with a `dg-` prefix
    let token_string = format!("dg-{}", gen_alphanumeric(API_TOKEN_LENGTH));

    let token = ApiToken::new(
        user.id,
        Utc::now().naive_utc(),
        data.name.clone(),
        &token_string,
    )
    .save(&appstate.pool)
    .await?;

    info!("Added new API token {} for user {username}", data.name);
    if let Some(owner) = User::find_by_id(&appstate.pool, token.user_id).await? {
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::ApiTokenAdded { owner, token }),
        })?;
    }
    Ok(ApiResponse::new(
        json!({"token": token_string}),
        StatusCode::CREATED,
    ))
}

// GET on user, returns ApiTokenInfo vector in JSON
/// List API tokens of a user
///
/// Token values are never returned, only their metadata.
#[utoipa::path(
    get,
    path = "/api/v1/user/{username}/api_token",
    tag = "API token",
    params(
        ("username" = String, Path, description = "Name of the user."),
    ),
    responses(
        (status = 200, description = "All API tokens of the user.", body = Vec<ApiTokenInfo>),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user not found"})),
        (status = 500, description = "Unable to list API tokens.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn fetch_api_tokens(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    session: SessionInfo,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    let tokens_info: Vec<ApiTokenInfo> = ApiToken::find_by_user_id(&appstate.pool, user.id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    Ok(ApiResponse::json(tokens_info, StatusCode::OK))
}

/// Delete an API token of a user
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/api_token/{token_id}",
    tag = "API token",
    params(
        ("username" = String, Path, description = "Name of the user."),
        ("token_id" = i64, Path, description = "ID of the API token."),
    ),
    responses(
        (status = 200, description = "API token deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 400, description = "Token not found.", body = ApiErrorResponse, example = json!({"msg": "Key not found"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to delete API token.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn delete_api_token(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path((username, token_id)): Path<(String, i64)>,
) -> ApiResult {
    debug!("Removing API token {token_id} for user {username}");
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(token) = ApiToken::find_by_id(&appstate.pool, token_id).await? {
        if !session.is_admin && user.id != token.user_id {
            return Err(WebError::Forbidden(""));
        }
        token.clone().delete(&appstate.pool).await?;
        if let Some(owner) = User::find_by_id(&appstate.pool, token.user_id).await? {
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::ApiTokenRemoved {
                    owner,
                    token: token.clone(),
                }),
            })?;
        }
        info!(
            "User {} removed API token {}({token_id}) for user {username}",
            user.username, token.name
        );
    } else {
        error!("API token with id {token_id} not found");
        return Err(WebError::BadRequest("Key not found".into()));
    }

    Ok(ApiResponse::with_status(StatusCode::OK))
}

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct RenameRequest {
    pub name: String,
}

/// Rename an API token of a user
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/api_token/{token_id}/rename",
    tag = "API token",
    request_body = RenameRequest,
    params(
        ("username" = String, Path, description = "Name of the user."),
        ("token_id" = i64, Path, description = "ID of the API token."),
    ),
    responses(
        (status = 200, description = "API token renamed."),
        (status = 400, description = "Invalid name.", body = ApiErrorResponse, example = json!({"msg": "Invalid name"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges and an active enterprise license.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User or token not found.", body = ApiErrorResponse, example = json!({"msg": "token not found"})),
        (status = 500, description = "Unable to rename API token.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn rename_api_token(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Path((username, token_id)): Path<(String, i64)>,
    Json(data): Json<RenameRequest>,
) -> ApiResult {
    debug!("Renaming API token {token_id} for user {username}");

    if !validate_name(&data.name) {
        return Err(WebError::BadRequest(
            "Name contains forbidden characters".into(),
        ));
    }

    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(mut token) = ApiToken::find_by_id(&appstate.pool, token_id).await? {
        if !session.is_admin && user.id != token.user_id {
            return Err(WebError::Forbidden(""));
        }
        let old_name = token.name.clone();
        token.name = data.name;
        let new_name = token.name.clone();
        token.save(&appstate.pool).await?;
        if let Some(owner) = User::find_by_id(&appstate.pool, token.user_id).await? {
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::ApiTokenRenamed {
                    owner,
                    token: token.clone(),
                    old_name,
                    new_name,
                }),
            })?;
        }
        info!(
            "User {} renamed API token {}({token_id}) for user {username}",
            user.username, token.name
        );
    } else {
        error!("User {username} tried to rename non-existing API token with id {token_id}",);
        return Err(WebError::ObjectNotFound(String::new()));
    }

    Ok(ApiResponse::with_status(StatusCode::OK))
}
