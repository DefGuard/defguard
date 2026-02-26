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
    wizard_flags: Option<WizardFlags>,
}

pub(crate) async fn get_session_info(
    State(appstate): State<AppState>,
    session: Result<SessionExtractor, WebError>,
) -> ApiResult {
    let pool = &appstate.pool;
    let flags = WizardFlags::get(pool).await?;

    let Ok(SessionExtractor(session)) = session else {
        if flags.initial_wizard_in_progress {
            return Ok(ApiResponse::json(
                SessionInfoResponse {
                    authorized: false,
                    wizard_flags: Some(flags),
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
            wizard_flags: Some(flags),
        },
        StatusCode::OK,
    ))
}
