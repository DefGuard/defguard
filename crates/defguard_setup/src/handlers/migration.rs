use std::sync::{Arc, Mutex};

use axum::{Extension, Json};
use defguard_common::db::models::{ActiveWizard, Wizard, migration_wizard::MigrationWizardState};
use defguard_core::{
    auth::AdminRole,
    enterprise::ldap,
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, query_as};
use tokio::sync::oneshot;
use tracing::info;

use crate::handlers::auto_wizard::{
    ExternalUrlSettingsConfig, InternalUrlSettingsConfig, apply_external_url_settings,
    apply_internal_url_settings,
};

pub async fn get_migration_state(_: AdminRole, Extension(pool): Extension<PgPool>) -> ApiResult {
    let migration_state = MigrationWizardState::get(&pool).await?.unwrap_or_default();
    let (ldap_migration_required,): (bool,) = query_as(
        "SELECT EXISTS(SELECT 1 FROM \"user\" WHERE from_ldap AND POSITION(chr(92) IN ldap_rdn) > 0)",
    )
    .fetch_one(&pool)
    .await?;
    let mut response = json!(migration_state);
    response["ldap_migration_required"] = json!(ldap_migration_required);
    Ok(ApiResponse::new(response, StatusCode::OK))
}

pub async fn repair_ldap_migration(_: AdminRole, Extension(pool): Extension<PgPool>) -> ApiResult {
    let counts = ldap::migration::repair(&pool)
        .await
        .map_err(|error| WebError::BadRequest(error.to_string()))?;
    Ok(ApiResponse::new(json!(counts), StatusCode::OK))
}

pub async fn update_migration_state(
    _: AdminRole,
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
    _: AdminRole,
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
            "Migration shutdown sender no longer available".to_owned(),
        ));
    }

    Ok(ApiResponse::with_status(StatusCode::OK))
}

pub async fn migration_set_internal_url_settings(
    _: AdminRole,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<InternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying migration wizard internal URL settings");
    let cert_info = apply_internal_url_settings(&pool, config).await?;
    info!("Migration wizard internal URL settings applied");
    Ok(ApiResponse::new(
        json!({ "cert_info": cert_info }),
        StatusCode::CREATED,
    ))
}

pub async fn migration_set_external_url_settings(
    _: AdminRole,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<ExternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying migration wizard external URL settings");
    let cert_info = apply_external_url_settings(&pool, config).await?;
    info!("Migration wizard external URL settings applied");
    Ok(ApiResponse::new(
        json!({ "cert_info": cert_info }),
        StatusCode::CREATED,
    ))
}
