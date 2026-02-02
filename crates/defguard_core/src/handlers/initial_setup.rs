use std::sync::{Arc, Mutex};

use axum::{Extension, Json};
use defguard_certs::{der_to_pem, parse_certificate_info, parse_pem_certificate};
use defguard_common::db::models::{
        Settings, User,
        group::Group,
        settings::update_current_settings,
    };
use reqwest::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::oneshot;
use tracing::{debug, info};

use crate::{
    auth::AdminOrSetupRole,
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateAdmin {
    first_name: String,
    last_name: String,
    username: String,
    email: String,
    password: String,
}

pub async fn create_admin(
    Extension(pool): Extension<PgPool>,
    Json(admin): Json<CreateAdmin>,
) -> ApiResult {
    info!(
        "Creating initial admin user {} ({})",
        admin.username, admin.email
    );
    User::new(
        admin.username,
        Some(admin.password.as_str()),
        admin.last_name,
        admin.first_name,
        admin.email,
        None,
    )
    .save(&pool)
    .await?;

    info!("Initial admin user created");

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}

#[derive(Deserialize, Serialize, Debug)]
pub struct GeneralConfig {
    defguard_url: String,
    default_admin_group_name: String,
    default_authentication: u32,
    default_mfa_code_lifetime: u32,
    admin_username: String,
}

pub async fn set_general_config(
    Extension(pool): Extension<PgPool>,
    Json(general_config): Json<GeneralConfig>,
) -> ApiResult {
    info!("Applying initial general configuration settings");
    debug!(
        "General configuration received: defguard_url={}, default_admin_group_name={}, default_authentication={}, default_mfa_code_lifetime={}, admin_username={}",
        general_config.defguard_url,
        general_config.default_admin_group_name,
        general_config.default_authentication,
        general_config.default_mfa_code_lifetime,
        general_config.admin_username
    );
    let default_admin_group_name = general_config.default_admin_group_name.clone();
    let mut settings = Settings::get_current_settings();
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
            debug!("Admin group {} not found, creating", default_admin_group_name);
            let mut group = Group::new(&default_admin_group_name);
            group.is_admin = true;
            group.save(&pool).await?
        };

    let admin_user = User::find_by_username(&pool, &general_config.admin_username)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Admin user '{}' not found",
                general_config.admin_username
            ))
        })?;
    debug!(
        "Assigning admin user {} to admin group {}",
        general_config.admin_username, admin_group.name
    );
    admin_user.add_to_group(&pool, &admin_group).await?;

    info!("Initial general configuration applied");

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateCA {
    common_name: String,
    email: String,
    validity_period_years: u32,
}

pub async fn create_ca(
    Extension(pool): Extension<PgPool>,
    Json(ca_info): Json<CreateCA>,
) -> ApiResult {
    info!("Creating new certificate authority");
    debug!(
        "CA request details: common_name={}, email={}, validity_period_years={}",
        ca_info.common_name, ca_info.email, ca_info.validity_period_years
    );
    let mut settings = Settings::get_current_settings();
    let ca = defguard_certs::CertificateAuthority::new(
        &ca_info.common_name,
        &ca_info.email,
        ca_info.validity_period_years * 365,
    )?;

    let (cert_der, key_der) = (ca.cert_der().to_vec(), ca.key_pair_der().to_vec());

    settings.ca_cert_der = Some(cert_der);
    settings.ca_key_der = Some(key_der);
    settings.ca_expiry = Some(ca.expiry()?);

    update_current_settings(&pool, settings).await?;

    info!("Certificate authority created and stored");

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}

pub async fn get_ca() -> ApiResult {
    debug!("Fetching certificate authority details");
    let settings = Settings::get_current_settings();
    if let Some(ca_cert_der) = settings.ca_cert_der {
        let ca_pem = der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate)?;
        let info = parse_certificate_info(&ca_cert_der)?;
        let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();

        debug!(
            "Certificate authority details prepared: subject_common_name={}, valid_for_days={}",
            info.subject_common_name, valid_for_days
        );

        Ok(ApiResponse {
            json: json!({ "ca_cert_pem": ca_pem, "subject_common_name": info.subject_common_name, "not_before": info.not_before, "not_after": info.not_after, "valid_for_days": valid_for_days }),
            status: StatusCode::OK,
        })
    } else {
        Err(WebError::ObjectNotFound(
            "CA certificate not found".to_string(),
        ))
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UploadCA {
    cert_file: String,
}

pub async fn upload_ca(
    Extension(pool): Extension<PgPool>,
    Json(ca_info): Json<UploadCA>,
) -> ApiResult {
    info!("Uploading existing certificate authority");
    let cert_der = parse_pem_certificate(&ca_info.cert_file)?;
    let expiry = parse_certificate_info(&cert_der)?.not_after;

    let mut settings = Settings::get_current_settings();
    settings.ca_cert_der = Some(cert_der.to_vec());
    settings.ca_key_der = None; // Key is not provided when uploading CA
    settings.ca_expiry = Some(expiry);

    update_current_settings(&pool, settings).await?;

    info!("Certificate authority uploaded and stored");

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}


pub async fn finish_setup(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Extension(setup_shutdown_tx): Extension<Arc<Mutex<Option<oneshot::Sender<()>>>>>,
) -> ApiResult {
    info!("Finishing initial setup");
    let mut settings = Settings::get_current_settings();
    settings.initial_setup_completed = true;
    update_current_settings(&pool, settings).await?;
    if let Some(tx) = setup_shutdown_tx.lock().expect("Failed to lock setup shutdown sender").take() {
        let _ = tx.send(());
        info!("Initial setup completed and shutdown signal sent");
    } else {
        return Err(WebError::BadRequest(
            "Setup shutdown sender no longer available".to_string(),
        ));
    }
    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}
