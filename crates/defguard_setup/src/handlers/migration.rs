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

#[derive(Serialize)]
struct MigrationStateResponse {
    #[serde(flatten)]
    state: MigrationWizardState,
    ldap_migration_required: bool,
}

pub async fn get_migration_state(_: AdminRole, Extension(pool): Extension<PgPool>) -> ApiResult {
    let migration_state = MigrationWizardState::get(&pool).await?.unwrap_or_default();
    let (ldap_migration_required,): (bool,) = query_as(
        "SELECT EXISTS(SELECT 1 FROM \"user\" WHERE from_ldap AND POSITION(chr(92) IN ldap_rdn) > 0)",
    )
    .fetch_one(&pool)
    .await?;
    let response = MigrationStateResponse {
        state: migration_state,
        ldap_migration_required,
    };
    Ok(ApiResponse::new(json!(response), StatusCode::OK))
}

#[cfg(test)]
mod tests {
    use super::MigrationStateResponse;
    use defguard_common::db::models::migration_wizard::MigrationWizardState;
    use serde_json::json;

    #[test]
    fn migration_state_response_flattens_state_fields() {
        let response = MigrationStateResponse {
            state: MigrationWizardState::default(),
            ldap_migration_required: true,
        };
        let value = serde_json::to_value(response).expect("Failed to serialize response");

        assert_eq!(value["ldap_migration_required"], json!(true));
        assert!(value.get("state").is_none());
        assert!(value.get("current_step").is_some());
    }
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
