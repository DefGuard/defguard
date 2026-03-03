use std::sync::{Arc, Mutex};

use axum::{Extension, Json};
use defguard_common::db::models::{
    Settings, User, group::Group, settings::update_current_settings,
};
use defguard_core::{
    auth::AdminOrSetupRole,
    db::models::wizard_flags::WizardFlags,
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

#[derive(Serialize, Deserialize, Debug)]
pub struct MigrationWizardLocationState {
    pub locations: Vec<i64>,
    pub current_location: i64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum MigrationWizardStep {
    #[default]
    #[serde(rename = "welcome")]
    Welcome,
    #[serde(rename = "general")]
    General,
    #[serde(rename = "ca")]
    Ca,
    #[serde(rename = "caSummary")]
    CaSummary,
    #[serde(rename = "edge")]
    Edge,
    #[serde(rename = "edgeAdoption")]
    EdgeAdoption,
    #[serde(rename = "confirmation")]
    Confirmation,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationWizardState {
    pub current_step: MigrationWizardStep,
    pub location_state: Option<MigrationWizardLocationState>,
}

impl Default for MigrationWizardState {
    fn default() -> Self {
        Self {
            current_step: MigrationWizardStep::Welcome,
            location_state: None,
        }
    }
}

pub async fn get_migration_state(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
) -> ApiResult {
    let mut transaction = pool.begin().await?;

    let raw_state: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT migration_wizard_state
         FROM wizard
         LIMIT 1",
    )
    .fetch_optional(&mut *transaction)
    .await?
    .flatten();

    let default_state = MigrationWizardState::default();

    let migration_state = match raw_state {
        Some(state) => match serde_json::from_value::<MigrationWizardState>(state) {
            Ok(parsed) => parsed,
            Err(error) => {
                warn!("Invalid migration_wizard_state format, resetting to NULL: {error}");
                sqlx::query(
                    "UPDATE wizard
                     SET migration_wizard_state = NULL
                     WHERE is_singleton = TRUE",
                )
                .execute(&mut *transaction)
                .await?;
                default_state
            }
        },
        None => default_state,
    };

    transaction.commit().await?;

    Ok(ApiResponse::new(json!(migration_state), StatusCode::OK))
}

pub async fn update_migration_state(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(data): Json<MigrationWizardState>,
) -> ApiResult {
    let state =
        serde_json::to_value(data).map_err(|error| WebError::Serialization(error.to_string()))?;

    sqlx::query(
        "UPDATE wizard
         SET migration_wizard_state = $1
         WHERE is_singleton = TRUE",
    )
    .bind(state)
    .execute(&pool)
    .await?;

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
    let settings = Settings::get_current_settings();
    debug!("Settings persisted");

    let admin_group =
        if let Some(mut group) = Group::find_by_name(&pool, &default_admin_group_name).await? {
            debug!(
                "Admin group {} found, marking as admin",
                default_admin_group_name
            );
            group.is_admin = true;
            group.save(&pool).await?;
            group
        } else {
            debug!(
                "Admin group {} not found, creating",
                default_admin_group_name
            );
            let mut group = Group::new(&default_admin_group_name);
            group.is_admin = true;
            group.save(&pool).await?
        };

    let admin_id = settings
        .default_admin_id
        .ok_or_else(|| WebError::DbError("Default admin user ID not set in settings".into()))?;

    let admin_user = User::find_by_id(&pool, admin_id).await?.ok_or_else(|| {
        WebError::ObjectNotFound(format!("Admin user with ID '{admin_id}' not found"))
    })?;
    debug!(
        "Assigning admin user {} to admin group {}",
        admin_user.username, admin_group.name
    );
    admin_user.add_to_group(&pool, &admin_group).await?;

    info!("Initial general configuration applied");

    Ok(ApiResponse::with_status(StatusCode::OK))
}

pub async fn finish_setup(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Extension(setup_shutdown_tx): Extension<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
) -> ApiResult {
    info!("Finishing migration");
    let mut wizard_flags = WizardFlags::get(&pool).await?;
    wizard_flags.migration_wizard_completed = true;
    wizard_flags.migration_wizard_in_progress = false;
    wizard_flags.initial_wizard_completed = true;
    wizard_flags.initial_wizard_in_progress = false;
    wizard_flags.save(&pool).await?;

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
