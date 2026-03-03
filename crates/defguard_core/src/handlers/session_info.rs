use axum::{extract::State, http::StatusCode};
use defguard_common::db::models::{User, Wizard};
use serde::Serialize;

use super::{ApiResponse, ApiResult};
use crate::{appstate::AppState, auth::SessionExtractor, error::WebError};

#[derive(Serialize)]
struct SessionInfoResponse {
    authorized: bool,
    wizard_flags: Option<Wizard>,
}

pub(crate) async fn get_session_info(
    State(appstate): State<AppState>,
    session: Result<SessionExtractor, WebError>,
) -> ApiResult {
    let pool = &appstate.pool;
    let wizard = Wizard::get(pool).await?;

    let Ok(SessionExtractor(session)) = session else {
        if wizard.is_active() {
            return Ok(ApiResponse::json(
                SessionInfoResponse {
                    authorized: false,
                    wizard_flags: Some(wizard),
                },
                StatusCode::OK,
            ));
        } else {
            return Ok(ApiResponse::json(
                SessionInfoResponse {
                    authorized: false,
                    wizard_flags: None,
                },
                StatusCode::OK,
            ));
        }
    };

    let Some(user) = User::find_by_id(pool, session.user_id).await? else {
        return Ok(ApiResponse::json(
            SessionInfoResponse {
                authorized: false,
                wizard_flags: None,
            },
            StatusCode::OK,
        ));
    };

    if !user.is_admin(pool).await? {
        return Ok(ApiResponse::json(
            SessionInfoResponse {
                authorized: true,
                wizard_flags: None,
            },
            StatusCode::OK,
        ));
    }

    Ok(ApiResponse::json(
        SessionInfoResponse {
            authorized: true,
            wizard_flags: Some(wizard),
        },
        StatusCode::OK,
    ))
}
