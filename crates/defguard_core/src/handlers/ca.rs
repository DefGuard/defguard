use axum::{Json, extract::State};
use defguard_common::db::models::{Settings, settings::update_current_settings};
use reqwest::StatusCode;
use serde_json::json;

use crate::{
    appstate::AppState,
    auth::AdminRole,
    handlers::{ApiResponse, ApiResult},
};

#[derive(Deserialize, Serialize, Debug)]
pub struct CreateCA {
    common_name: String,
    email: String,
    validity_period_days: u32,
}

pub async fn create_ca(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Json(ca_info): Json<CreateCA>,
) -> ApiResult {
    let mut settings = Settings::get_current_settings();
    let ca = defguard_certs::CertificateAuthority::new(
        &ca_info.common_name,
        &ca_info.email,
        ca_info.validity_period_days,
    )?;

    let (cert_der, key_der) = (ca.cert_der().to_vec(), ca.key_pair_der().to_vec());

    settings.ca_cert_der = Some(cert_der);
    settings.ca_key_der = Some(key_der);
    settings.ca_expiry = Some(ca.expiry()?);

    update_current_settings(&appstate.pool, settings).await?;

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}
