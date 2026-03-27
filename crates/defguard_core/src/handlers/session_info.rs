use axum::{extract::State, http::StatusCode};
use defguard_common::db::models::{ActiveWizard, User, Wizard};
use serde::Serialize;

use super::{ApiResponse, ApiResult};
use crate::{appstate::AppState, auth::SessionExtractor, error::WebError};

#[derive(Serialize)]
struct SessionInfoResponse {
    authorized: bool,
    is_admin: bool,
    active_wizard: Option<ActiveWizard>,
    username: Option<String>,
}

pub async fn get_session_info(
    State(appstate): State<AppState>,
    session: Result<SessionExtractor, WebError>,
) -> ApiResult {
    let pool = &appstate.pool;
    let wizard = Wizard::get(pool).await?;
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
                username: None,
            },
            StatusCode::OK,
        ));
    };

    let Some(user) = User::find_by_id(pool, session.user_id).await? else {
        return Ok(ApiResponse::json(
            SessionInfoResponse {
                authorized: false,
                is_admin: false,
                active_wizard,
                username: None,
            },
            StatusCode::OK,
        ));
    };

    let user_admin = user.is_admin(pool).await?;

    Ok(ApiResponse::json(
        SessionInfoResponse {
            authorized: true,
            is_admin: user_admin,
            active_wizard,
            username: Some(user.username),
        },
        StatusCode::OK,
    ))
}
