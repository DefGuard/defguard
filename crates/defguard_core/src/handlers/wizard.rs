use axum::{Json, extract::State, http::StatusCode};
use defguard_common::db::models::{migration_wizard::MigrationWizardState, wizard::Wizard};
use serde_json::json;

use super::{ApiResponse, ApiResult};
use crate::{appstate::AppState, auth::AdminRole};

pub(crate) async fn get_wizard_flags(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let wizard = Wizard::get(&appstate.pool).await?;

    Ok(ApiResponse::json(wizard, StatusCode::OK))
}

pub(crate) async fn get_migration_wizard_state(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let wizard = Wizard::get(&appstate.pool).await?;

    Ok(ApiResponse::new(
        json!({
            "migration_state": wizard.migration_wizard_state
        }),
        StatusCode::OK,
    ))
}

pub(crate) async fn update_migration_wizard_state(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<MigrationWizardState>,
) -> ApiResult {
    let mut wizard = Wizard::get(&appstate.pool).await?;
    wizard.migration_wizard_state = Some(data);
    wizard.save(&appstate.pool).await?;

    Ok(ApiResponse::new(json!({}), StatusCode::OK))
}
