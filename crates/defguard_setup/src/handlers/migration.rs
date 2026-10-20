use std::{
    collections::VecDeque,
    mem,
    sync::{Arc, Mutex, PoisonError},
};

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
use tracing::{error, info};

use defguard_core::setup_logs::scope_setup_logs;

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
        "SELECT NOT ldap_migration_completed AND EXISTS(SELECT 1 FROM \"user\" WHERE from_ldap AND POSITION(chr(92) IN ldap_rdn) > 0) FROM wizard WHERE is_singleton",
    )
    .fetch_one(&pool)
    .await?;
    let response = MigrationStateResponse {
        state: migration_state,
        ldap_migration_required,
    };
    Ok(ApiResponse::new(json!(response), StatusCode::OK))
}

pub async fn repair_ldap_migration(_: AdminRole, Extension(pool): Extension<PgPool>) -> ApiResult {
    let logs = Arc::new(Mutex::new(VecDeque::new()));
    let result = scope_setup_logs(Arc::clone(&logs), async {
        let result = ldap::migration::repair(&pool).await;
        if let Err(error) = &result {
            error!("LDAP repair failed: {error}");
        }
        result
    })
    .await;
    let logs = {
        let mut guard = logs.lock().unwrap_or_else(PoisonError::into_inner);
        mem::take(&mut *guard).into_iter().collect::<Vec<_>>()
    };
    match result {
        Ok(counts) => Ok(ApiResponse::new(
            json!({
                "found": counts.found,
                "migrated": counts.migrated,
                "logs": logs,
            }),
            StatusCode::OK,
        )),
        Err(error) => Ok(ApiResponse::new(
            json!({ "msg": error.to_string(), "logs": logs }),
            StatusCode::BAD_REQUEST,
        )),
    }
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

#[cfg(test)]
mod tests {
    use super::repair_ldap_migration;
    use axum::{Extension, body::to_bytes, response::IntoResponse};
    use defguard_core::{auth::AdminRole, setup_logs::CoreSetupLogLayer};
    use reqwest::StatusCode;
    use sqlx::postgres::PgPoolOptions;
    use tracing::instrument::WithSubscriber;
    use tracing_subscriber::prelude::*;

    #[tokio::test]
    async fn ldap_repair_failure_returns_separate_message_and_captured_logs() {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/defguard_test")
            .expect("Failed to create lazy pool");
        pool.close().await;

        let response = repair_ldap_migration(AdminRole, Extension(pool))
            .with_subscriber(tracing_subscriber::registry().with(CoreSetupLogLayer))
            .await
            .expect("Expected an HTTP error response")
            .into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        let value: serde_json::Value = serde_json::from_slice(&body).expect("Invalid JSON");

        let message = value["msg"].as_str().expect("Expected error message");
        let logs = value["logs"].as_array().expect("Expected logs array");
        assert!(!message.is_empty());
        assert!(!logs.is_empty());
    }
}
