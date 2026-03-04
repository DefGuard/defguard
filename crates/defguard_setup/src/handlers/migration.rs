use std::sync::{Arc, Mutex};

use axum::{Extension, Json};
use defguard_common::db::models::{
    ActiveWizard, Settings, Wizard, group::Group, migration_wizard::MigrationWizardState,
    settings::update_current_settings,
};
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
use tracing::{debug, info};

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
    default_admin_group_name: String,
    default_authentication: u32,
    default_mfa_code_lifetime: u32,
    public_proxy_url: String,
}

pub async fn set_general_config(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(general_config): Json<GeneralConfig>,
) -> ApiResult {
    info!("Applying initial general configuration settings");
    debug!(
        "General configuration received: defguard_url={}, default_admin_group_name={}, default_authentication={}, default_mfa_code_lifetime={}, public_proxy_url={}",
        general_config.defguard_url,
        general_config.default_admin_group_name,
        general_config.default_authentication,
        general_config.default_mfa_code_lifetime,
        general_config.public_proxy_url,
    );
    let default_admin_group_name = general_config.default_admin_group_name.clone();
    let mut settings = Settings::get_current_settings();
    settings.public_proxy_url = general_config.public_proxy_url;
    settings.defguard_url = general_config.defguard_url;
    settings.default_admin_group_name = general_config.default_admin_group_name;
    settings.authentication_period_days = general_config
        .default_authentication
        .try_into()
        .map_err(|err| {
            WebError::BadRequest(format!("Invalid authentication period days: {err}"))
        })?;
    settings.mfa_code_timeout_seconds = general_config
        .default_mfa_code_lifetime
        .try_into()
        .map_err(|err| WebError::BadRequest(format!("Invalid MFA code timeout seconds: {err}")))?;
    update_current_settings(&pool, settings).await?;
    debug!("Settings persisted");

    if let Some(mut group) = Group::find_by_name(&pool, &default_admin_group_name).await? {
        debug!(
            "Admin group {} found, marking as admin",
            default_admin_group_name
        );
        group.is_admin = true;
        group.save(&pool).await?;
    } else {
        debug!(
            "Admin group {} not found, creating",
            default_admin_group_name
        );
        let mut group = Group::new(&default_admin_group_name);
        group.is_admin = true;
        group.save(&pool).await?;
    };

    info!("Initial general configuration applied");

    Ok(ApiResponse::with_status(StatusCode::OK))
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
    MigrationWizardState::clear(&mut *transaction);
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
