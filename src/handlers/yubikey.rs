use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;

use super::{user_for_admin_or_self, ApiResponse, ApiResult};
use crate::{appstate::AppState, auth::SessionInfo, db::models::yubikey::YubiKey, error::WebError};

pub async fn delete_yubikey(
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path((username, key_id)): Path<(String, i64)>,
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
        return Err(WebError::Forbidden("Not allowed to delete YubiKey".into()));
    }
    yubikey.delete(&appstate.pool).await?;
    info!("Yubikey {key_id} deleted by user {}", user.id);
    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

#[derive(Debug, Deserialize, Clone)]
pub struct RenameRequest {
    name: String,
}

pub async fn rename_yubikey(
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path((username, key_id)): Path<(String, i64)>,
    Json(data): Json<RenameRequest>,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    debug!("User {} attempts to rename yubikey {}", user.id, key_id);
    let Some(mut yubikey) = YubiKey::find_by_id(&appstate.pool, key_id).await? else {
        error!("Yubikey with id {key_id} not found");
        return Err(WebError::ObjectNotFound("YubiKey not found".into()));
    };
    if !session.is_admin && yubikey.user_id != user.id {
        warn!(
            "User {}, tried to rename yubikey {key_id} of user {} without being an admin.",
            user.id, yubikey.user_id
        );
        return Err(WebError::Forbidden(String::new()));
    }
    yubikey.name = data.name;
    yubikey.save(&appstate.pool).await?;
    info!("Yubikey {} renamed by user {}", yubikey.id, user.id);
    Ok(ApiResponse {
        json: json!(yubikey),
        status: StatusCode::OK,
    })
}
