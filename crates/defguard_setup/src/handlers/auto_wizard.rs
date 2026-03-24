use std::convert::Infallible;

use axum::{
    Extension, Json,
    response::sse::{Event, KeepAlive, Sse},
};
use defguard_certs::{
    CertificateAuthority, CertificateInfo, Csr, PemLabel, der_to_pem, generate_key_pair,
    parse_pem_certificate,
};
use defguard_common::{
    db::models::{
        Certificates, WireguardNetwork,
        certificates::{CoreCertSource, ProxyCertSource},
        proxy::Proxy,
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
    handlers::{ApiResponse, ApiResult},
};
use defguard_proto::proxy::AcmeStep;
use futures::Stream;
use rcgen::DnType;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, query_scalar};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::auto_adoption::{call_proxy_finish_setup, call_proxy_issue_acme};

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
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InternalSslType {
    /// No SSL - plain HTTP, user manages reverse proxy / SSL termination themselves.
    None,
    /// Generate certificates using Defguard's internal Certificate Authority.
    DefguardCa,
    /// Upload a custom certificate and private key.
    OwnCert,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InternalUrlSettingsConfig {
    defguard_url: String,
    ssl_type: InternalSslType,
    cert_pem: Option<String>,
    key_pem: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct CertInfoResponse {
    pub common_name: String,
    pub valid_for_days: i64,
    pub not_before: String,
    pub not_after: String,
}

/// Updates internal URL settings and configures SSL for the core web server.
pub async fn set_internal_url_settings(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<InternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying Auto-adoption wizard internal URL settings");
    debug!(
        "Internal URL settings received: defguard_url={}, ssl_type={:?}",
        config.defguard_url, config.ssl_type,
    );

    let mut settings = defguard_common::db::models::Settings::get_current_settings();
    settings.defguard_url = config.defguard_url.clone();
    update_current_settings(&pool, settings).await?;

    let mut certs = Certificates::get_or_default(&pool)
        .await
        .map_err(WebError::from)?;

    let cert_info = match config.ssl_type {
        InternalSslType::None => {
            certs.core_http_cert_source = CoreCertSource::None;
            certs.core_http_cert_pem = None;
            certs.core_http_cert_key_pem = None;
            certs.core_http_cert_expiry = None;
            certs.save(&pool).await.map_err(WebError::from)?;
            None
        }
        InternalSslType::DefguardCa => {
            // Extract hostname from defguard_url for the SAN.
            let hostname = reqwest::Url::parse(&config.defguard_url)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_else(|| config.defguard_url.clone());

            // CA must already be present at this point.
            if certs.ca_cert_der.is_none() {
                return Err(WebError::BadRequest(
                    "CA certificate is not present; generate a CA first".to_string(),
                ));
            }

            // Generate server certificate signed by the CA.
            let ca_cert_der = certs.ca_cert_der.as_ref().expect("CA cert must be present");
            let ca_key_der = certs.ca_key_der.as_ref().ok_or_else(|| {
                WebError::BadRequest("CA private key not available for signing".to_string())
            })?;

            let ca = CertificateAuthority::from_cert_der_key_pair(ca_cert_der, ca_key_der)?;
            let key_pair = generate_key_pair()?;
            let san = vec![hostname.clone()];
            let dn = vec![(DnType::CommonName, hostname.as_str())];
            let csr = Csr::new(&key_pair, &san, dn)?;
            let server_cert = ca.sign_csr(&csr)?;

            let cert_der = server_cert.der().to_vec();
            let cert_pem = der_to_pem(&cert_der, PemLabel::Certificate)?;
            let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)?;
            let info = CertificateInfo::from_der(&cert_der)?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.core_http_cert_source = CoreCertSource::SelfSigned;
            certs.core_http_cert_pem = Some(cert_pem);
            certs.core_http_cert_key_pem = Some(key_pem);
            certs.core_http_cert_expiry = Some(expiry);
            certs.save(&pool).await.map_err(WebError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
        InternalSslType::OwnCert => {
            let cert_pem_str = config.cert_pem.ok_or_else(|| {
                WebError::BadRequest("cert_pem is required for own_cert".to_string())
            })?;
            let key_pem_str = config.key_pem.ok_or_else(|| {
                WebError::BadRequest("key_pem is required for own_cert".to_string())
            })?;

            let cert_der = parse_pem_certificate(&cert_pem_str)?;
            let info = CertificateInfo::from_der(cert_der.as_ref())?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.core_http_cert_source = CoreCertSource::Custom;
            certs.core_http_cert_pem = Some(cert_pem_str);
            certs.core_http_cert_key_pem = Some(key_pem_str);
            certs.core_http_cert_expiry = Some(expiry);
            certs.save(&pool).await.map_err(WebError::from)?;

            Some(CertInfoResponse {
                common_name: info.subject_common_name.clone(),
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
    };

    advance_auto_wizard_to_step(&pool, AutoAdoptionWizardStep::InternalUrlSslConfig).await?;

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
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExternalSslType {
    /// No SSL - plain HTTP, user manages reverse proxy / SSL termination themselves.
    None,
    /// Obtain certificate via ACME / Let's Encrypt.
    LetsEncrypt,
    /// Generate certificate using Defguard's internal Certificate Authority.
    DefguardCa,
    /// Upload a custom certificate and private key.
    OwnCert,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ExternalUrlSettingsConfig {
    public_proxy_url: String,
    #[serde(default)]
    ssl_type: ExternalSslType,
    cert_pem: Option<String>,
    key_pem: Option<String>,
}

impl Default for ExternalSslType {
    fn default() -> Self {
        Self::None
    }
}

/// Updates external proxy URL settings (step 4).
pub async fn set_external_url_settings(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    Json(config): Json<ExternalUrlSettingsConfig>,
) -> ApiResult {
    info!("Applying Auto-adoption wizard external URL settings");
    debug!(
        "External URL settings received: public_proxy_url={}, ssl_type={:?}",
        config.public_proxy_url, config.ssl_type,
    );

    let mut settings = defguard_common::db::models::Settings::get_current_settings();
    settings.public_proxy_url = config.public_proxy_url.clone();
    update_current_settings(&pool, settings).await?;

    let mut certs = Certificates::get_or_default(&pool)
        .await
        .map_err(WebError::from)?;

    // Look up the adopted proxy once so we can call FinishSetup on non-ACME paths.
    let adopted_proxy = Proxy::list(&pool).await.ok().and_then(|mut v| {
        if v.is_empty() {
            None
        } else {
            Some(v.remove(0))
        }
    });

    let cert_info = match config.ssl_type {
        ExternalSslType::None => {
            certs.proxy_http_cert_source = ProxyCertSource::None;
            certs.proxy_http_cert_pem = None;
            certs.proxy_http_cert_key_pem = None;
            certs.proxy_http_cert_expiry = None;
            certs.save(&pool).await.map_err(WebError::from)?;

            if let Some(proxy) = adopted_proxy {
                if let Err(e) =
                    call_proxy_finish_setup(&pool, &proxy.address, proxy.port as u16).await
                {
                    error!("Failed to signal FinishSetup to proxy (non-ACME): {e}");
                }
            }
            None
        }
        ExternalSslType::LetsEncrypt => {
            let hostname = reqwest::Url::parse(&config.public_proxy_url)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_else(|| config.public_proxy_url.clone());
            certs.proxy_http_cert_source = ProxyCertSource::LetsEncrypt;
            certs.acme_domain = Some(hostname);
            certs.proxy_http_cert_pem = None;
            certs.proxy_http_cert_key_pem = None;
            certs.proxy_http_cert_expiry = None;
            certs.save(&pool).await.map_err(WebError::from)?;
            // FinishSetup is NOT called here - the frontend will follow up with the
            // stream_external_url_lets_encrypt endpoint which calls call_proxy_issue_acme.
            None
        }
        ExternalSslType::DefguardCa => {
            let hostname = reqwest::Url::parse(&config.public_proxy_url)
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_else(|| config.public_proxy_url.clone());

            // CA must already be present at this point.
            if certs.ca_cert_der.is_none() {
                return Err(WebError::BadRequest(
                    "CA certificate is not present; generate a CA first".to_string(),
                ));
            }

            let ca_cert_der = certs.ca_cert_der.as_ref().expect("CA cert must be present");
            let ca_key_der = certs.ca_key_der.as_ref().ok_or_else(|| {
                WebError::BadRequest("CA private key not available for signing".to_string())
            })?;

            let ca = CertificateAuthority::from_cert_der_key_pair(ca_cert_der, ca_key_der)?;
            let key_pair = generate_key_pair()?;
            let san = vec![hostname.clone()];
            let dn = vec![(DnType::CommonName, hostname.as_str())];
            let csr = Csr::new(&key_pair, &san, dn)?;
            let server_cert = ca.sign_csr(&csr)?;

            let cert_der = server_cert.der().to_vec();
            let cert_pem = der_to_pem(&cert_der, PemLabel::Certificate)?;
            let key_pem = der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)?;
            let info = CertificateInfo::from_der(&cert_der)?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.proxy_http_cert_source = ProxyCertSource::SelfSigned;
            certs.proxy_http_cert_pem = Some(cert_pem);
            certs.proxy_http_cert_key_pem = Some(key_pem);
            certs.proxy_http_cert_expiry = Some(expiry);
            certs.save(&pool).await.map_err(WebError::from)?;

            if let Some(proxy) = adopted_proxy {
                if let Err(e) =
                    call_proxy_finish_setup(&pool, &proxy.address, proxy.port as u16).await
                {
                    error!("Failed to signal FinishSetup to proxy (DefguardCa): {e}");
                }
            }

            Some(CertInfoResponse {
                common_name: info.subject_common_name,
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
        ExternalSslType::OwnCert => {
            let cert_pem_str = config.cert_pem.ok_or_else(|| {
                WebError::BadRequest("cert_pem is required for own_cert".to_string())
            })?;
            let key_pem_str = config.key_pem.ok_or_else(|| {
                WebError::BadRequest("key_pem is required for own_cert".to_string())
            })?;

            let cert_der = parse_pem_certificate(&cert_pem_str)?;
            let info = CertificateInfo::from_der(cert_der.as_ref())?;
            let valid_for_days = (info.not_after.and_utc() - chrono::Utc::now()).num_days();
            let expiry = info.not_after;

            certs.proxy_http_cert_source = ProxyCertSource::Custom;
            certs.proxy_http_cert_pem = Some(cert_pem_str);
            certs.proxy_http_cert_key_pem = Some(key_pem_str);
            certs.proxy_http_cert_expiry = Some(expiry);
            certs.save(&pool).await.map_err(WebError::from)?;

            if let Some(proxy) = adopted_proxy {
                if let Err(e) =
                    call_proxy_finish_setup(&pool, &proxy.address, proxy.port as u16).await
                {
                    error!("Failed to signal FinishSetup to proxy (OwnCert): {e}");
                }
            }

            Some(CertInfoResponse {
                common_name: info.subject_common_name.clone(),
                valid_for_days,
                not_before: info.not_before.to_string(),
                not_after: info.not_after.to_string(),
            })
        }
    };

    advance_auto_wizard_to_step(&pool, AutoAdoptionWizardStep::ExternalUrlSslConfig).await?;

    info!("Auto-adoption wizard external URL settings applied");
    Ok(ApiResponse::new(
        json!({ "cert_info": cert_info }),
        StatusCode::CREATED,
    ))
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

#[derive(Deserialize, Serialize, Debug)]
pub struct UrlSettingsConfig {
    defguard_url: String,
    public_proxy_url: String,
}

/// Updates URL settings used by auto-adoption wizard (legacy - sets both at once).
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

#[derive(Debug, Serialize)]
struct AcmeSetupResponse {
    step: &'static str,
    error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn acme_event(step: &'static str) -> Event {
    let body = serde_json::to_string(&AcmeSetupResponse {
        step,
        error: false,
        message: None,
    })
    .unwrap_or_else(|_| format!(r#"{{"step":"{step}","error":false}}"#));
    Event::default().data(body)
}

fn acme_error_event(step: &'static str, message: String) -> Event {
    let body = serde_json::to_string(&AcmeSetupResponse {
        step,
        error: true,
        message: Some(message.clone()),
    })
    .unwrap_or_else(|_| format!(r#"{{"step":"{step}","error":true,"message":"{message}"}}"#));
    Event::default().data(body)
}

/// Maps a proto [`AcmeStep`] to the SSE step string expected by the frontend.
fn acme_step_name(step: AcmeStep) -> &'static str {
    match step {
        AcmeStep::Unspecified => "Connecting",
        AcmeStep::Connecting => "Connecting",
        AcmeStep::ValidatingDomain => "ValidatingDomain",
        AcmeStep::IssuingCertificate => "IssuingCertificate",
    }
}

/// Maximum time to wait for the ACME flow to complete end-to-end.
const ACME_TIMEOUT_SECS: u64 = 300;

/// Streams Let's Encrypt certificate issuance progress as Server-Sent Events.
///
/// Delegates the ACME HTTP-01 process directly to the proxy component via
/// [`call_proxy_issue_acme`], which reconnects to the proxy's `ProxySetup` gRPC
/// service over TLS.  The proxy must already be adopted (its setup server still
/// alive waiting for `IssueAcme` or `FinishSetup`) before this endpoint is called.
///
/// Core emits `Done` after it successfully saves the certificate to the database.
// GET: EventSource only supports GET
pub async fn stream_external_url_lets_encrypt(
    _: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let certs = match Certificates::get_or_default(&pool).await {
            Ok(c) => c,
            Err(e) => {
                yield Ok(acme_error_event("Connecting", format!("Failed to load certificates config: {e}")));
                return;
            }
        };

        let domain = match certs.acme_domain.clone() {
            Some(d) if !d.is_empty() => d,
            _ => {
                yield Ok(acme_error_event(
                    "Connecting",
                    "No ACME domain configured. Please re-submit the external URL settings with a Let's Encrypt domain.".to_string(),
                ));
                return;
            }
        };

        let account_credentials_json = certs.acme_account_credentials.clone().unwrap_or_default();

        let proxies = match Proxy::list(&pool).await {
            Ok(list) => list,
            Err(e) => {
                yield Ok(acme_error_event("Connecting", format!("Failed to load proxy list from DB: {e}")));
                return;
            }
        };

        let proxy = match proxies.into_iter().next() {
            Some(p) => p,
            None => {
                yield Ok(acme_error_event(
                    "Connecting",
                    "No proxy found in database. Please complete the edge adoption step first.".to_string(),
                ));
                return;
            }
        };

        let proxy_host = proxy.address.clone();
        let proxy_port = proxy.port as u16;
        info!("Triggering ACME HTTP-01 via ProxySetup gRPC for domain: {domain} proxy={proxy_host}:{proxy_port}");

        // Channel for real-time progress steps forwarded from the proxy.
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<AcmeStep>();
        // Oneshot for the final result (cert or error string).
        let (result_tx, mut result_rx) = tokio::sync::oneshot::channel::<Result<(String, String, String), String>>();

        let pool_clone = pool.clone();
        let domain_clone = domain.clone();
        let account_creds_clone = account_credentials_json.clone();
        tokio::spawn(async move {
            let result = call_proxy_issue_acme(
                &pool_clone,
                &proxy_host,
                proxy_port,
                domain_clone,
                true,
                account_creds_clone,
                progress_tx,
            ).await;
            let _ = result_tx.send(result);
        });

        // Track the most recently seen step name for error reporting.
        let mut current_step: &'static str = "Connecting";

        // Overall timeout guard.
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_secs(ACME_TIMEOUT_SECS);

        loop {
            tokio::select! {
                // Real progress step arrived from the proxy.
                maybe_step = progress_rx.recv() => {
                    match maybe_step {
                        Some(step) => {
                            current_step = acme_step_name(step);
                            yield Ok(acme_event(current_step));
                        }
                        None => {
                            // progress_tx was dropped - the ACME task finished.
                            // The final result will arrive on result_rx; keep looping.
                        }
                    }
                }

                // Final result from the ACME task.
                res = &mut result_rx => {
                    match res {
                        Ok(Ok((cert_pem, key_pem, account_credentials_json))) => {
                            match Certificates::get_or_default(&pool).await {
                                Ok(mut updated_certs) => {
                                    updated_certs.proxy_http_cert_pem = Some(cert_pem);
                                    updated_certs.proxy_http_cert_key_pem = Some(key_pem);
                                    updated_certs.acme_account_credentials = Some(account_credentials_json);
                                    updated_certs.proxy_http_cert_source = ProxyCertSource::LetsEncrypt;
                                    if let Err(e) = updated_certs.save(&pool).await {
                                        yield Ok(acme_error_event("Installing", format!("Failed to save certificate: {e}")));
                                        return;
                                    }
                                }
                                Err(e) => {
                                    yield Ok(acme_error_event("Installing", format!("Failed to reload certificates for saving: {e}")));
                                    return;
                                }
                            }
                            info!("ACME certificate issued and saved for domain: {domain}");
                            yield Ok(acme_event("Done"));
                            return;
                        }
                        Ok(Err(acme_err)) => {
                            let msg = format!("ACME issuance failed: {acme_err}");
                            error!("{msg}");
                            yield Ok(acme_error_event(current_step, msg));
                            return;
                        }
                        Err(_) => {
                            yield Ok(acme_error_event(current_step, "ACME task terminated unexpectedly.".to_string()));
                            return;
                        }
                    }
                }

                // Hard deadline - something hung on the proxy side.
                _ = tokio::time::sleep_until(deadline) => {
                    yield Ok(acme_error_event(current_step, format!("ACME certificate issuance timed out after {ACME_TIMEOUT_SECS} seconds.")));
                    return;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
