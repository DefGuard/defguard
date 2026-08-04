use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;

use super::{ApiErrorResponse, ApiResponse, ApiResult};
use crate::{appstate::AppState, auth::AdminRole, error::WebError};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CheckResource {
    Email,
    Username,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CheckReservedParams {
    pub resource: CheckResource,
    pub value: String,
}

async fn email_exists(pool: &PgPool, email: &str) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM "user" WHERE LOWER(email) = LOWER($1)) AS "exists!""#,
        email
    )
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

async fn username_exists(pool: &PgPool, username: &str) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM "user" WHERE username = $1) AS "exists!""#,
        username
    )
    .fetch_one(pool)
    .await?;
    Ok(exists)
}

/// Check whether an email address or username is already taken.
#[utoipa::path(
    get,
    path = "/api/v1/reserved",
    tag = "system",
    params(
        ("resource" = CheckResource, Query, description = "Type of the checked value: `email` or `username`."),
        ("value" = String, Query, description = "Value to check."),
    ),
    responses(
        (status = 200, description = "Availability of the value.", body = Object, example = json!({"available": true})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 409, description = "The value is already taken.", body = ApiErrorResponse, example = json!({"msg": "admin is already taken"})),
        (status = 500, description = "Internal server error.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn check_reserved(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Query(params): Query<CheckReservedParams>,
) -> ApiResult {
    let exists = match params.resource {
        CheckResource::Email => email_exists(&appstate.pool, &params.value).await?,
        CheckResource::Username => username_exists(&appstate.pool, &params.value).await?,
    };

    if exists {
        Err(WebError::ObjectAlreadyExists(format!(
            "{} is already taken",
            params.value
        )))
    } else {
        Ok(ApiResponse::new(
            serde_json::json!({"available": true}),
            StatusCode::OK,
        ))
    }
}
