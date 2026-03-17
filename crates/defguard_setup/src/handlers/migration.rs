use std::sync::{Arc, Mutex};

use axum::{Extension, Json};
use defguard_common::db::models::{ActiveWizard, Wizard, migration_wizard::MigrationWizardState};
use defguard_core::{
    auth::AdminOrSetupRole,
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::oneshot;
use tracing::info;

pub async fn get_migration_state(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
) -> ApiResult {
    let migration_state = MigrationWizardState::get(&pool).await?.unwrap_or_default();
    Ok(ApiResponse::new(json!(migration_state), StatusCode::OK))
}

pub async fn update_migration_state(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(data): Json<MigrationWizardState>,
) -> ApiResult {
    data.save(&pool).await?;

    Ok(ApiResponse::new(json!({}), StatusCode::OK))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GeneralConfig {
    defguard_url: String,
    default_mfa_code_lifetime: u32,
    public_proxy_url: String,
}

pub async fn finish_setup(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Extension(setup_shutdown_tx): Extension<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
) -> ApiResult {
    info!("Finishing migration");
    let mut transaction = pool
        .begin()
        .await
        .expect("Failed to initialize transaction");
    let mut wizard = Wizard::get(&mut *transaction).await?;
    wizard.active_wizard = ActiveWizard::None;
    wizard.completed = true;
    wizard.save(&mut *transaction).await?;
    MigrationWizardState::clear(&mut *transaction).await?;
    transaction.commit().await?;

    if let Some(tx) = setup_shutdown_tx
        .lock()
        .expect("Failed to lock migration shutdown sender")
        .take()
    {
        let _ = tx.send(());
        info!("Migration completed and shutdown signal sent");
    } else {
        return Err(WebError::BadRequest(
            "Migration shutdown sender no longer available".to_string(),
        ));
    }

    Ok(ApiResponse::with_status(StatusCode::OK))
}
