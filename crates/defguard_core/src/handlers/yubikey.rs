use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use defguard_common::db::{Id, models::YubiKey};
use utoipa::ToSchema;

use super::{ApiErrorResponse, ApiResponse, ApiResult, user_for_admin_or_self};
use crate::{appstate::AppState, auth::SessionInfo, error::WebError};

/// Delete a YubiKey of a user
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/yubikey/{key_id}",
    tag = "user",
    params(
        ("username" = String, Path, description = "Name of the user."),
        ("key_id" = i64, Path, description = "ID of the YubiKey."),
    ),
    responses(
        (status = 200, description = "YubiKey deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User or YubiKey not found.", body = ApiErrorResponse, example = json!({"msg": "YubiKey not found"})),
        (status = 500, description = "Unable to delete YubiKey.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_yubikey(
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path((username, key_id)): Path<(String, Id)>,
) -> ApiResult {
    debug!("Deleting yubikey {key_id} by {:?}", &session.user.id);
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    let Some(yubikey) = YubiKey::find_by_id(&appstate.pool, key_id).await? else {
        error!("Yubikey with id {key_id} not found");
        return Err(WebError::ObjectNotFound("YubiKey not found".into()));
    };
    if !session.is_admin && yubikey.user_id != user.id {
        warn!(
            "User {} tried to delete yubikey {key_id} of user {} without being an admin.",
            user.id, yubikey.user_id
        );
        return Err(WebError::Forbidden("Not allowed to delete YubiKey"));
    }
    yubikey.delete(&appstate.pool).await?;
    info!("Yubikey {key_id} deleted by user {}", user.id);
    Ok(ApiResponse::with_status(StatusCode::OK))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct RenameRequest {
    name: String,
}

/// Rename a YubiKey of a user
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/yubikey/{key_id}/rename",
    tag = "user",
    request_body = RenameRequest,
    params(
        ("username" = String, Path, description = "Name of the user."),
        ("key_id" = i64, Path, description = "ID of the YubiKey."),
    ),
    responses(
        (status = 200, description = "YubiKey renamed.", body = Object, example = json!({"id": 1, "name": "work key", "serial": "12345678", "user_id": 1})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User or YubiKey not found.", body = ApiErrorResponse, example = json!({"msg": "YubiKey not found"})),
        (status = 500, description = "Unable to rename YubiKey.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn rename_yubikey(
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path((username, key_id)): Path<(String, Id)>,
    Json(data): Json<RenameRequest>,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    debug!("User {} attempts to rename yubikey {key_id}", user.id);
    let Some(mut yubikey) = YubiKey::find_by_id(&appstate.pool, key_id).await? else {
        error!("Yubikey with id {key_id} not found");
        return Err(WebError::ObjectNotFound("YubiKey not found".into()));
    };
    if !session.is_admin && yubikey.user_id != user.id {
        warn!(
            "User {}, tried to rename yubikey {key_id} of user {} without being an admin.",
            user.id, yubikey.user_id
        );
        return Err(WebError::Forbidden(""));
    }
    yubikey.name = data.name;
    yubikey.save(&appstate.pool).await?;
    info!("Yubikey {} renamed by user {}", yubikey.id, user.id);
    Ok(ApiResponse::json(yubikey, StatusCode::OK))
}
