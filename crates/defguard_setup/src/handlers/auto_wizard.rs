use axum::{Extension, Json};
use defguard_certs::{PemLabel, der_to_pem};
use defguard_common::{
    db::models::{
        Certificates, WireguardNetwork,
        initial_setup_wizard::InitialSetupStep,
        settings::update_current_settings,
        setup_auto_adoption::{AutoAdoptionWizardState, AutoAdoptionWizardStep},
        wireguard::LocationMfaMode,
        wizard::{ActiveWizard, Wizard},
    },
    utils::{parse_address_list, parse_network_address_list},
};
use defguard_core::{
    auth::AdminOrSetupRole,
    error::WebError,
    handlers::{ApiResponse, ApiResult, core_certs},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, query_scalar};
use tracing::{debug, info};

use crate::handlers::initial_wizard::advance_initial_wizard_to_step;

pub(crate) async fn is_auto_wizard_active(pool: &PgPool) -> Result<bool, WebError> {
    let wizard = Wizard::get(pool).await?;
    Ok(wizard.active_wizard == ActiveWizard::AutoAdoption)
}

pub(crate) async fn advance_auto_wizard_to_step(
    pool: &PgPool,
    step: AutoAdoptionWizardStep,
) -> Result<(), WebError> {
    let mut auto_state = AutoAdoptionWizardState::get(pool)
        .await?
        .unwrap_or_default();
    if auto_state.step < step {
        auto_state.step = step;
        auto_state.save(pool).await?;
        info!("Advanced auto wizard setup to step {step:?}");
    } else {
        debug!(
            "Not advancing auto wizard setup step from {:?} to {:?} as it is not a forward step",
            auto_state.step, step
        );
    }

    Ok(())
}

/// SSL configuration type for Defguard's internal (core) web server.
pub use core_certs::InternalSslType;

#[derive(Deserialize, Serialize, Debug)]
pub struct InternalUrlSettingsConfig {
    defguard_url: String,
    ssl_type: InternalSslType,
    cert_pem: Option<String>,
    key_pem: Option<String>,
}

pub use core_certs::CertInfoResponse;

/// Core logic for applying internal URL settings and configuring SSL for the core web server.
/// Returns the cert info if a certificate was generated/uploaded, `None` for `ssl_type = None`.
pub(crate) async fn apply_internal_url_settings(
    pool: &PgPool,
    config: InternalUrlSettingsConfig,
) -> Result<Option<CertInfoResponse>, WebError> {
    core_certs::apply_internal_url_settings(
        pool,
        &config.defguard_url,
        core_certs::InternalUrlSettingsConfig {
            ssl_type: config.ssl_type,
            cert_pem: config.cert_pem,
            key_pem: config.key_pem,
        },
    )
    .await
}

/// Updates internal URL settings and configures SSL for the core web server.
pub async fn set_internal_url_settings(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<InternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying Auto-adoption wizard internal URL settings");
    let ssl_type = config.ssl_type.clone();
    let cert_info = apply_internal_url_settings(&pool, config).await?;

    // When ssl_type is None, there is no SSL config step to complete; skip straight to the
    // next step in each wizard.
    let auto_next = match ssl_type {
        InternalSslType::None => AutoAdoptionWizardStep::ExternalUrlSettings,
        _ => AutoAdoptionWizardStep::InternalUrlSslConfig,
    };
    let initial_next = match ssl_type {
        InternalSslType::None => InitialSetupStep::ExternalUrlSettings,
        _ => InitialSetupStep::InternalUrlSslConfig,
    };
    advance_auto_wizard_to_step(&pool, auto_next).await?;
    advance_initial_wizard_to_step(&pool, initial_next).await?;

    info!("Auto-adoption wizard internal URL settings applied");

    Ok(ApiResponse::new(
        json!({ "cert_info": cert_info }),
        StatusCode::CREATED,
    ))
}

/// Returns internal SSL certificate info (for the "Download certificate" step).
pub async fn get_internal_ssl_info(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
) -> ApiResult {
    let certs = Certificates::get_or_default(&pool)
        .await
        .map_err(WebError::from)?;

    // Return CA cert PEM (for browser import) if present.
    let ca_cert_pem = certs
        .ca_cert_der
        .as_deref()
        .and_then(|der| der_to_pem(der, PemLabel::Certificate).ok());

    Ok(ApiResponse::new(
        json!({ "ca_cert_pem": ca_cert_pem }),
        StatusCode::OK,
    ))
}

/// SSL configuration type for the external (proxy) web server.
pub use core_certs::ExternalSslType;

#[derive(Deserialize, Serialize, Debug)]
pub struct ExternalUrlSettingsConfig {
    public_proxy_url: String,
    #[serde(default)]
    ssl_type: ExternalSslType,
    cert_pem: Option<String>,
    key_pem: Option<String>,
}

/// Updates external proxy URL settings (step 4).
pub async fn set_external_url_settings(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<ExternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying Auto-adoption wizard external URL settings");
    let ssl_type = config.ssl_type.clone();
    let cert_info = apply_external_url_settings(&pool, config).await?;

    // When ssl_type is None, there is no SSL config step to complete; skip straight to the
    // next step in each wizard.
    let auto_next = match ssl_type {
        ExternalSslType::None => AutoAdoptionWizardStep::VpnSettings,
        _ => AutoAdoptionWizardStep::ExternalUrlSslConfig,
    };
    let initial_next = match ssl_type {
        ExternalSslType::None => InitialSetupStep::Confirmation,
        _ => InitialSetupStep::ExternalUrlSslConfig,
    };
    advance_auto_wizard_to_step(&pool, auto_next).await?;
    advance_initial_wizard_to_step(&pool, initial_next).await?;

    info!("Auto-adoption wizard external URL settings applied");
    Ok(ApiResponse::new(
        json!({ "cert_info": cert_info }),
        StatusCode::CREATED,
    ))
}

/// Core logic for applying external URL settings and configuring SSL for the proxy web server.
/// Returns the cert info if a certificate was generated/uploaded, `None` otherwise.
pub(crate) async fn apply_external_url_settings(
    pool: &PgPool,
    config: ExternalUrlSettingsConfig,
) -> Result<Option<CertInfoResponse>, WebError> {
    let mut settings = defguard_common::db::models::Settings::get_current_settings();
    settings.public_proxy_url = config.public_proxy_url.clone();
    update_current_settings(pool, settings).await?;

    core_certs::apply_external_url_settings(
        pool,
        &config.public_proxy_url,
        core_certs::ExternalUrlSettingsConfig {
            ssl_type: config.ssl_type,
            cert_pem: config.cert_pem,
            key_pem: config.key_pem,
        },
    )
    .await
}

/// Returns external SSL certificate info (for the "Download CA certificate" step).
pub async fn get_external_ssl_info(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
) -> ApiResult {
    let certs = Certificates::get_or_default(&pool)
        .await
        .map_err(WebError::from)?;

    let ca_cert_pem = certs
        .ca_cert_der
        .as_deref()
        .and_then(|der| der_to_pem(der, PemLabel::Certificate).ok());

    Ok(ApiResponse::new(
        json!({ "ca_cert_pem": ca_cert_pem }),
        StatusCode::OK,
    ))
}

#[allow(clippy::struct_field_names)]
#[derive(Deserialize, Serialize, Debug)]
pub struct VpnSettingsConfig {
    #[serde(rename = "vpn_public_ip")]
    public_ip: String,
    #[serde(rename = "vpn_wireguard_port")]
    wireguard_port: i32,
    #[serde(rename = "vpn_gateway_address")]
    gateway_address: String,
    #[serde(rename = "vpn_allowed_ips")]
    allowed_ips: String,
    #[serde(rename = "vpn_dns_server_ip")]
    dns_server_ip: String,
}

/// Updates first auto-adopted network location with VPN settings from auto-adoption wizard.
pub async fn set_vpn_settings(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(vpn_settings): Json<VpnSettingsConfig>,
) -> ApiResult {
    info!("Applying Auto-adoption wizard VPN settings");

    let first_network_id =
        query_scalar::<_, i64>("SELECT id FROM wireguard_network ORDER BY id ASC LIMIT 1")
            .fetch_optional(&pool)
            .await?
            .ok_or_else(|| {
                WebError::ObjectNotFound("No network location found to configure".to_string())
            })?;

    let mut network = WireguardNetwork::find_by_id(&pool, first_network_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Network location with ID '{first_network_id}' not found"
            ))
        })?;

    let addresses = parse_address_list(vpn_settings.gateway_address.as_str());
    if addresses.is_empty() {
        return Err(WebError::BadRequest(
            "Invalid gateway address value".to_string(),
        ));
    }

    let allowed_ips_input = vpn_settings.allowed_ips.trim();
    let allowed_ips = if allowed_ips_input.is_empty() {
        Vec::new()
    } else {
        let parsed = parse_network_address_list(allowed_ips_input);
        if parsed.is_empty() {
            return Err(WebError::BadRequest(
                "Invalid allowed IPs value".to_string(),
            ));
        }
        parsed
    };

    network.endpoint = vpn_settings.public_ip;
    network.port = vpn_settings.wireguard_port;
    let mut network = network.set_address(addresses)?;
    network.allowed_ips = allowed_ips;
    network.dns = {
        let dns = vpn_settings.dns_server_ip.trim();
        if dns.is_empty() {
            None
        } else {
            Some(dns.to_string())
        }
    };
    network.save(&pool).await?;

    advance_auto_wizard_to_step(&pool, AutoAdoptionWizardStep::MfaSettings).await?;

    debug!(
        "Auto-adoption VPN settings applied to network_id={} endpoint={} port={}",
        network.id, network.endpoint, network.port
    );

    Ok(ApiResponse::with_status(StatusCode::CREATED))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MfaSettingsConfig {
    #[serde(rename = "vpn_mfa_mode")]
    mfa_mode: LocationMfaMode,
}

/// Updates first auto-adopted network location with MFA mode from Auto-adoption wizard.
pub async fn set_mfa_settings(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(mfa_settings): Json<MfaSettingsConfig>,
) -> ApiResult {
    info!("Applying Auto-adoption wizard MFA settings");

    let first_network_id =
        query_scalar::<_, i64>("SELECT id FROM wireguard_network ORDER BY id ASC LIMIT 1")
            .fetch_optional(&pool)
            .await?
            .ok_or_else(|| {
                WebError::ObjectNotFound("No network location found to configure".to_string())
            })?;

    let mut network = WireguardNetwork::find_by_id(&pool, first_network_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Network location with ID '{first_network_id}' not found"
            ))
        })?;

    network.location_mfa_mode = mfa_settings.mfa_mode;
    network.save(&pool).await?;

    advance_auto_wizard_to_step(&pool, AutoAdoptionWizardStep::Summary).await?;

    debug!(
        "Auto-adoption MFA settings applied to network_id={} location_mfa_mode={:?}",
        network.id, network.location_mfa_mode
    );

    Ok(ApiResponse::with_status(StatusCode::CREATED))
}

pub async fn get_auto_adoption_result(Extension(pool): Extension<PgPool>) -> ApiResult {
    let state = AutoAdoptionWizardState::get(&pool).await?;
    Ok(ApiResponse::new(json!(state), StatusCode::OK))
}
