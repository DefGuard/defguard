use axum::{Extension, Json};
use defguard_common::{
    db::models::{
        WireguardNetwork,
        settings::update_current_settings,
        setup_auto_adoption::AutoAdoptionWizardStep,
        wireguard::LocationMfaMode,
        wizard::{ActiveWizard, Wizard},
    },
    utils::{parse_address_list, parse_network_address_list},
};
use defguard_core::{
    auth::AdminOrSetupRole,
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, query_scalar};
use tracing::{debug, info};

pub(crate) async fn is_auto_wizard_active(pool: &PgPool) -> Result<bool, WebError> {
    let wizard = Wizard::get(pool).await?;
    Ok(wizard.active_wizard == ActiveWizard::AutoAdoption)
}

pub(crate) async fn advance_auto_wizard_to_step(
    pool: &PgPool,
    step: AutoAdoptionWizardStep,
) -> Result<(), WebError> {
    let mut wizard = Wizard::get(pool).await?;
    let auto_state = wizard
        .auto_adoption_state
        .get_or_insert_with(Default::default);
    if auto_state.step < step {
        auto_state.step = step;
        wizard.save(pool).await?;
        info!("Advanced auto wizard setup to step {:?}", step);
    } else {
        debug!(
            "Not advancing auto wizard setup step from {:?} to {:?} as it is not a forward step",
            auto_state.step, step
        );
    }

    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UrlSettingsConfig {
    defguard_url: String,
    public_proxy_url: String,
}

/// Updates URL settings used by auto-adoption wizard.
pub async fn set_url_settings(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(url_settings): Json<UrlSettingsConfig>,
) -> ApiResult {
    info!("Applying Auto-adoption wizard URL settings");
    debug!(
        "URL settings received: defguard_url={}, public_proxy_url={}",
        url_settings.defguard_url, url_settings.public_proxy_url,
    );

    let mut settings = defguard_common::db::models::Settings::get_current_settings();
    settings.defguard_url = url_settings.defguard_url;
    settings.public_proxy_url = url_settings.public_proxy_url;
    update_current_settings(&pool, settings).await?;

    advance_auto_wizard_to_step(&pool, AutoAdoptionWizardStep::VpnSettings).await?;

    info!("Auto-adoption wizard URL settings applied");

    Ok(ApiResponse::with_status(StatusCode::CREATED))
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
    network.address = addresses;
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
    let wizard = Wizard::get(&pool).await?;
    Ok(ApiResponse::new(
        json!(wizard.auto_adoption_state),
        StatusCode::OK,
    ))
}
