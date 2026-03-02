use axum::{extract::State, http::StatusCode};
use defguard_common::db::models::User;
use serde::Serialize;

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState, auth::SessionExtractor, db::models::wizard_flags::WizardFlags,
    error::WebError,
};

#[derive(Serialize)]
struct SessionInfoResponse {
    authorized: bool,
    is_admin: bool,
    wizard_flags: Option<WizardFlags>,
}

pub async fn get_session_info(
    State(appstate): State<AppState>,
    session: Result<SessionExtractor, WebError>,
) -> ApiResult {
    let pool = &appstate.pool;
    let flags = WizardFlags::get(pool).await?;

    let Ok(SessionExtractor(session)) = session else {
        return Ok(ApiResponse::json(
            SessionInfoResponse {
                authorized: false,
                is_admin: false,
                wizard_flags: if flags.initial_wizard_in_progress {
                    Some(flags)
                } else {
                    None
                },
            },
            StatusCode::OK,
        ));
    };

    let Some(user) = User::find_by_id(pool, session.user_id).await? else {
        return Ok(ApiResponse::json(
            SessionInfoResponse {
                authorized: false,
                is_admin: false,
                wizard_flags: None,
            },
            StatusCode::OK,
        ));
    };

    let user_admin = user.is_admin(pool).await?;

    Ok(ApiResponse::json(
        SessionInfoResponse {
            authorized: true,
            is_admin: user_admin,
            wizard_flags: if user_admin { Some(flags) } else { None },
        },
        StatusCode::OK,
    ))
}
