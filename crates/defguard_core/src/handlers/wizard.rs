use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;

use super::{ApiResponse, ApiResult};
use crate::{
    appstate::AppState, auth::AdminRole, db::models::migration_wizard::MigrationWizardState,
};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub(crate) struct WizardFlags {
    pub migration_wizard_needed: bool,
    pub migration_wizard_in_progress: bool,
    pub migration_wizard_completed: bool,
    pub initial_wizard_completed: bool,
    pub initial_wizard_in_progress: bool,
}

pub(crate) async fn get_wizard_flags(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let flags = sqlx::query_as!(
        WizardFlags,
        "SELECT
            migration_wizard_needed,
            migration_wizard_in_progress,
            migration_wizard_completed,
            initial_wizard_completed,
            initial_wizard_in_progress
         FROM wizard
         LIMIT 1"
    )
    .fetch_one(&appstate.pool)
    .await?;

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
