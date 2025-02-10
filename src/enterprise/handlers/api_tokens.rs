use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::json;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::api_tokens::{ApiToken, ApiTokenInfo},
    error::WebError,
    handlers::{user_for_admin_or_self, ApiResponse, ApiResult},
    random::gen_alphanumeric,
};

use super::LicenseInfo;

const API_TOKEN_LENGTH: usize = 32;

#[derive(Deserialize, Serialize, Debug)]
pub struct AddApiTokenData {
    pub name: String,
}

pub async fn add_api_token(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
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
            "Cannot create API token for non-admin user".into(),
        ));
    };

    // TODO: check if the name is already used

    // generate token string
    // all API tokens start with a `dg-` prefix
    let token_string = format!("dg-{}", gen_alphanumeric(API_TOKEN_LENGTH));

    ApiToken::new(
        user.id,
        Utc::now().naive_utc(),
        data.name.clone(),
        &token_string,
    )
    .save(&appstate.pool)
    .await?;

    info!("Added new API token {} for user {username}", data.name);

    Ok(ApiResponse {
        json: json!({"token": token_string}),
        status: StatusCode::CREATED,
    })
}

// GET on user, returns ApiTokenInfo vector in JSON
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
        .map(|token| token.into())
        .collect();

    Ok(ApiResponse {
        json: json!(tokens_info),
        status: StatusCode::OK,
    })
}

pub async fn delete_api_token(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path((username, token_id)): Path<(String, i64)>,
) -> ApiResult {
    debug!("Removing API token {token_id} for user {username}");
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(token) = ApiToken::find_by_id(&appstate.pool, token_id).await? {
        if !session.is_admin && user.id != token.user_id {
            return Err(WebError::Forbidden(String::new()));
        }
        token.delete(&appstate.pool).await?;
    } else {
        error!("API token with id {token_id} not found");
        return Err(WebError::BadRequest("Key not found".into()));
    }

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RenameRequest {
    pub name: String,
}

pub async fn rename_api_token(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path((username, token_id)): Path<(String, i64)>,
    Json(data): Json<RenameRequest>,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(mut token) = ApiToken::find_by_id(&appstate.pool, token_id).await? {
        if !session.is_admin && user.id != token.user_id {
            return Err(WebError::Forbidden(String::new()));
        }
        token.name = data.name;
        token.save(&appstate.pool).await?;
    } else {
        error!("User {username} tried to rename non-existing API token with id {token_id}",);
        return Err(WebError::ObjectNotFound(String::new()));
    }

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}
