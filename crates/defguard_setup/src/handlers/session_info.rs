use axum::{Extension, http::StatusCode};
use defguard_common::db::models::{ActiveWizard, User, Wizard};
use defguard_core::{
    auth::SessionExtractor,
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
struct SessionInfoResponse {
    authorized: bool,
    is_admin: bool,
    active_wizard: Option<ActiveWizard>,
}

pub async fn get_session_info(
    Extension(pool): Extension<PgPool>,
    session: Result<SessionExtractor, WebError>,
) -> ApiResult {
    let wizard = Wizard::get(&pool).await?;
    let active_wizard = if wizard.completed {
        None
    } else {
        Some(wizard.active_wizard)
    };

    let Ok(SessionExtractor(session)) = session else {
        return Ok(ApiResponse::json(
            SessionInfoResponse {
                authorized: false,
                is_admin: false,
                active_wizard,
            },
            StatusCode::OK,
        ));
    };

    let Some(user) = User::find_by_id(&pool, session.user_id).await? else {
        return Ok(ApiResponse::json(
            SessionInfoResponse {
                authorized: false,
                is_admin: false,
                active_wizard,
            },
            StatusCode::OK,
        ));
    };

    let is_admin = user.is_admin(&pool).await?;

    Ok(ApiResponse::json(
        SessionInfoResponse {
            authorized: true,
            is_admin,
            active_wizard,
        },
        StatusCode::OK,
    ))
}
