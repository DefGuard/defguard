use axum::{Json, extract::State, http::StatusCode};
use serde_json::json;

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState,
    auth::AdminRole,
    db::models::{migration_wizard::MigrationWizardState, wizard_flags::WizardFlags},
};

pub(crate) async fn get_wizard_flags(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let flags = WizardFlags::get(&appstate.pool).await?;

    Ok(ApiResponse::json(flags, StatusCode::OK))
}

pub(crate) async fn get_migration_wizard_state(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let migration_state = MigrationWizardState::get(&appstate.pool).await?;

    Ok(ApiResponse::new(
        json!({
            "migration_state": migration_state
        }),
        StatusCode::OK,
    ))
}

pub(crate) async fn update_migration_wizard_state(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<MigrationWizardState>,
) -> ApiResult {
    data.save(&appstate.pool).await?;

    Ok(ApiResponse::new(json!({}), StatusCode::OK))
}
