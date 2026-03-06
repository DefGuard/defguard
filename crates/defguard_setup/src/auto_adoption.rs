use std::time::Duration;

use anyhow::Context;
use defguard_certs::{CertificateAuthority, CertificateInfo, Csr, PemLabel, der_to_pem};
use defguard_common::{
    VERSION,
    auth::claims::{Claims, ClaimsType},
    config::DefGuardConfig,
    db::models::{
        Settings, WireguardNetwork,
        gateway::Gateway,
        proxy::Proxy,
        settings::update_current_settings,
        setup_auto_adoption::{
            AutoAdoptionComponentResult, AutoAdoptionWizardState, SetupAutoAdoptionComponent,
        },
        wireguard::{
            DEFAULT_DISCONNECT_THRESHOLD, DEFAULT_KEEPALIVE_INTERVAL, DEFAULT_WIREGUARD_MTU,
            LocationMfaMode, ServiceLocationMode,
        },
    },
};
use defguard_core::version::{MIN_GATEWAY_VERSION, MIN_PROXY_VERSION};
use defguard_proto::{
    gateway::{
        CertificateInfo as GatewayCertificateInfo, DerPayload as GatewayDerPayload,
        gateway_setup_client::GatewaySetupClient,
    },
    proxy::{
        CertificateInfo as ProxyCertificateInfo, DerPayload as ProxyDerPayload,
        proxy_setup_client::ProxySetupClient,
    },
};
use defguard_version::{Version, client::ClientVersionInterceptor};
use ipnetwork::IpNetwork;
use reqwest::Url;
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedReceiver;
use tonic::{
    Request, Status,
    service::Interceptor,
    transport::{Certificate, ClientTlsConfig, Endpoint},
};
use tracing::{debug, error, info, warn};

const TOKEN_CLIENT_ID: &str = "Defguard Core";
const STARTUP_ADOPTION_TIMEOUT: Duration = Duration::from_secs(10);
const AUTO_ADOPTION_CA_COMMON_NAME: &str = "Defguard Automatic Setup CA";
const AUTO_ADOPTION_CA_EMAIL: &str = "auto-adoption@defguard.local";
const AUTO_ADOPTION_CA_VALIDITY_DAYS: u32 = 3650;
const GATEWAY_NAME: &str = "Gateway";
const PROXY_NAME: &str = "Edge";
const KEEPALIVE_INTERVAL_SECONDS: Duration = Duration::from_secs(5);

async fn ensure_ca_for_auto_adoption(pool: &PgPool) -> Result<(), anyhow::Error> {
    let mut settings = Settings::get_current_settings();
    let has_cert = settings.ca_cert_der.is_some();
    let has_key = settings.ca_key_der.is_some();

    if has_cert && has_key {
        debug!("Auto-adoption mode: existing CA certificate/key found");
        return Ok(());
    }

    if has_cert && !has_key {
        warn!(
            "Auto-adoption mode requested but existing CA has no private key; generating new CA so startup adoption can proceed"
        );
    } else {
        info!("Auto-adoption mode requested with no CA configured; generating CA automatically");
    }

    let ca = CertificateAuthority::new(
        AUTO_ADOPTION_CA_COMMON_NAME,
        AUTO_ADOPTION_CA_EMAIL,
        AUTO_ADOPTION_CA_VALIDITY_DAYS,
    )
    .context("Failed to create automatic setup CA")?;

    settings.ca_cert_der = Some(ca.cert_der().to_vec());
    settings.ca_key_der = Some(ca.key_pair_der().to_vec());
    settings.ca_expiry = Some(
        ca.expiry()
            .context("Failed to determine automatic CA expiry")?,
    );

    update_current_settings(pool, settings)
        .await
        .context("Failed to persist automatically generated CA for auto-adoption")?;

    info!(
        "Automatic setup CA generated successfully for startup adoption mode (validity_days={AUTO_ADOPTION_CA_VALIDITY_DAYS})"
    );
    Ok(())
}

fn parse_host_port(input: &str) -> Result<(String, u16), anyhow::Error> {
    if let Some(rest) = input.strip_prefix('[') {
        let (host, port_part) = rest
            .split_once(']')
            .context("Invalid endpoint format. Expected [ipv6]:port")?;
        let port = port_part
            .strip_prefix(':')
            .context("Invalid endpoint format. Missing port separator ':'")?
            .parse::<u16>()
            .context("Invalid port in endpoint")?;
        return Ok((host.to_string(), port));
    }

    let (host, port) = input
        .rsplit_once(':')
        .context("Invalid endpoint format. Expected host:port")?;
    if host.trim().is_empty() {
        anyhow::bail!("Invalid endpoint format. Host cannot be empty");
    }

    Ok((
        host.to_string(),
        port.parse::<u16>().context("Invalid port in endpoint")?,
    ))
}

#[derive(Clone)]
struct AuthInterceptor {
    token: String,
}

impl AuthInterceptor {
    const fn new(token: String) -> Self {
        Self { token }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request.metadata_mut().insert(
            "authorization",
            format!("Bearer {}", self.token)
                .parse()
                .expect("failed to parse auth metadata"),
        );
        Ok(request)
    }
}

struct TaskGuard(tokio::task::JoinHandle<()>);

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn adoption_failure(message: impl Into<String>) -> (bool, Vec<String>, Option<CertificateInfo>) {
    let msg = message.into();
    (false, vec![msg], None)
}

fn format_component_log(timestamp: &str, level: &str, target: &str, message: &str) -> String {
    let level = level
        .strip_prefix("Level(")
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(level)
        .to_uppercase();

    format!("{timestamp} {level} {target}: message={message}")
}

fn collect_stream_logs(log_rx: &mut UnboundedReceiver<String>) -> Vec<String> {
    let mut logs = Vec::new();
    while let Ok(log) = log_rx.try_recv() {
        logs.push(log);
    }
    logs
}

fn adoption_failure_with_logs(
    log_rx: &mut UnboundedReceiver<String>,
) -> (bool, Vec<String>, Option<CertificateInfo>) {
    let logs = collect_stream_logs(log_rx);
    (false, logs, None)
}

async fn run_edge_adoption_attempt(
    _pool: &PgPool,
    host: &str,
    port: u16,
) -> (bool, Vec<String>, Option<CertificateInfo>) {
    let (log_tx, mut log_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let settings = Settings::get_current_settings();
    let Some(ca_cert_der) = settings.ca_cert_der else {
        return adoption_failure("CA certificate not found in settings");
    };
    let Some(ca_key_der) = settings.ca_key_der else {
        return adoption_failure(
            "CA private key not found in settings. Uploading CA cert without key cannot auto-adopt.",
        );
    };
    let endpoint_str = format!("http://{host}:{port}");
    let url = match Url::parse(&endpoint_str) {
        Ok(url) => url,
        Err(err) => return adoption_failure(format!("Invalid edge endpoint URL: {err}")),
    };

    let cert_pem = match der_to_pem(&ca_cert_der, PemLabel::Certificate) {
        Ok(pem) => pem,
        Err(err) => {
            return adoption_failure(format!("Failed to convert CA certificate to PEM: {err}"));
        }
    };

    let base_endpoint = match Endpoint::from_shared(endpoint_str.clone()) {
        Ok(endpoint) => endpoint,
        Err(err) => return adoption_failure(format!("Failed to build edge endpoint: {err}")),
    };

    let base_endpoint = base_endpoint
        .http2_keep_alive_interval(KEEPALIVE_INTERVAL_SECONDS)
        .tcp_keepalive(Some(KEEPALIVE_INTERVAL_SECONDS))
        .keep_alive_while_idle(true);

    let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(cert_pem));
    let endpoint = match base_endpoint.tls_config(tls) {
        Ok(endpoint) => endpoint,
        Err(err) => {
            return adoption_failure(format!("Failed to configure TLS for edge endpoint: {err}"));
        }
    };

    let core_version = match Version::parse(VERSION) {
        Ok(version) => version,
        Err(err) => return adoption_failure(format!("Failed to parse core version: {err}")),
    };

    let token = match Claims::new(
        ClaimsType::Gateway,
        url.to_string(),
        TOKEN_CLIENT_ID.to_string(),
        u32::MAX.into(),
    )
    .to_jwt()
    {
        Ok(token) => token,
        Err(err) => return adoption_failure(format!("Failed to generate setup token: {err}")),
    };

    let version_interceptor = ClientVersionInterceptor::new(core_version.clone());
    let auth_interceptor = AuthInterceptor::new(token);

    let mut client =
        ProxySetupClient::with_interceptor(endpoint.connect_lazy(), move |mut req: Request<()>| {
            req = version_interceptor.clone().call(req)?;
            auth_interceptor.clone().call(req)
        });

    let response_with_metadata =
        match tokio::time::timeout(STARTUP_ADOPTION_TIMEOUT, client.start(())).await {
            Ok(Ok(response)) => response,
            Ok(Err(err)) => {
                return adoption_failure(format!("Failed to start edge setup stream: {err}"));
            }
            Err(_) => {
                return adoption_failure(format!(
                    "Timed out connecting to edge setup endpoint after {} seconds",
                    STARTUP_ADOPTION_TIMEOUT.as_secs()
                ));
            }
        };

    let edge_version = response_with_metadata
        .metadata()
        .get(defguard_version::VERSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(defguard_version::Version::parse)
        .transpose()
        .unwrap_or(None);

    if let Some(edge_version) = edge_version {
        if edge_version < MIN_PROXY_VERSION {
            return adoption_failure_with_logs(&mut log_rx);
        }
    } else {
        return adoption_failure_with_logs(&mut log_rx);
    }

    let mut response = response_with_metadata.into_inner();
    let log_reader_task = tokio::spawn(async move {
        loop {
            match response.message().await {
                Ok(Some(entry)) => {
                    let formatted = format_component_log(
                        &entry.timestamp,
                        &entry.level,
                        &entry.target,
                        &entry.message,
                    );
                    if log_tx.send(formatted).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    let _ = log_tx.send(format!("Error reading log: {err}"));
                    break;
                }
            }
        }
    });
    let _log_task_guard = TaskGuard(log_reader_task);

    let Some(hostname) = url.host_str() else {
        error!("Failed to extract hostname from proxy URL");
        return adoption_failure_with_logs(&mut log_rx);
    };

    let csr_response = match client
        .get_csr(ProxyCertificateInfo {
            cert_hostname: hostname.to_string(),
        })
        .await
    {
        Ok(response) => response.into_inner(),
        Err(err) => {
            error!("Failed to get CSR from proxy: {err}");
            return adoption_failure_with_logs(&mut log_rx);
        }
    };

    let csr = match Csr::from_der(&csr_response.der_data) {
        Ok(csr) => csr,
        Err(err) => {
            error!("Failed to parse CSR: {err}");
            return adoption_failure_with_logs(&mut log_rx);
        }
    };

    let ca = match CertificateAuthority::from_cert_der_key_pair(&ca_cert_der, &ca_key_der) {
        Ok(ca) => ca,
        Err(err) => {
            return adoption_failure(format!("Failed to build certificate authority: {err}"));
        }
    };

    let cert = match ca.sign_csr(&csr) {
        Ok(cert) => cert,
        Err(err) => {
            error!("Failed to sign CSR: {err}");
            return adoption_failure_with_logs(&mut log_rx);
        }
    };

    if let Err(err) = client
        .send_cert(ProxyDerPayload {
            der_data: cert.der().to_vec(),
        })
        .await
    {
        error!("Failed to send certificate to proxy: {err}");
        return adoption_failure_with_logs(&mut log_rx);
    }

    let cert_info = match CertificateInfo::from_der(cert.der()) {
        Ok(info) => info,
        Err(err) => {
            error!("Failed to parse certificate info: {err}");
            return adoption_failure_with_logs(&mut log_rx);
        }
    };

    let mut logs = collect_stream_logs(&mut log_rx);
    if logs.is_empty() {
        logs = vec!["No runtime logs received from edge component".to_string()];
    }

    (true, logs, Some(cert_info))
}

async fn run_gateway_adoption_attempt(
    host: &str,
    port: u16,
) -> (bool, Vec<String>, Option<CertificateInfo>) {
    let (log_tx, mut log_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    let settings = Settings::get_current_settings();
    let Some(ca_cert_der) = settings.ca_cert_der else {
        return adoption_failure("CA certificate not found in settings");
    };
    let Some(ca_key_der) = settings.ca_key_der else {
        return adoption_failure(
            "CA private key not found in settings. Uploading CA cert without key cannot auto-adopt.",
        );
    };

    let endpoint_str = format!("http://{host}:{port}");
    let url = match Url::parse(&endpoint_str) {
        Ok(url) => url,
        Err(err) => return adoption_failure(format!("Invalid gateway endpoint URL: {err}")),
    };

    let cert_pem = match der_to_pem(&ca_cert_der, PemLabel::Certificate) {
        Ok(pem) => pem,
        Err(err) => {
            return adoption_failure(format!("Failed to convert CA certificate to PEM: {err}"));
        }
    };

    let base_endpoint = match Endpoint::from_shared(endpoint_str.clone()) {
        Ok(endpoint) => endpoint,
        Err(err) => return adoption_failure(format!("Failed to build gateway endpoint: {err}")),
    };

    let base_endpoint = base_endpoint
        .http2_keep_alive_interval(KEEPALIVE_INTERVAL_SECONDS)
        .tcp_keepalive(Some(KEEPALIVE_INTERVAL_SECONDS))
        .keep_alive_while_idle(true);

    let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(cert_pem));
    let endpoint = match base_endpoint.tls_config(tls) {
        Ok(endpoint) => endpoint,
        Err(err) => {
            return adoption_failure(format!(
                "Failed to configure TLS for gateway endpoint: {err}"
            ));
        }
    };

    let core_version = match Version::parse(VERSION) {
        Ok(version) => version,
        Err(err) => return adoption_failure(format!("Failed to parse core version: {err}")),
    };

    let token = match Claims::new(
        ClaimsType::Gateway,
        url.to_string(),
        TOKEN_CLIENT_ID.to_string(),
        u32::MAX.into(),
    )
    .to_jwt()
    {
        Ok(token) => token,
        Err(err) => return adoption_failure(format!("Failed to generate setup token: {err}")),
    };

    let version_interceptor = ClientVersionInterceptor::new(core_version.clone());
    let auth_interceptor = AuthInterceptor::new(token);

    let mut client = GatewaySetupClient::with_interceptor(
        endpoint.connect_lazy(),
        move |mut req: Request<()>| {
            req = version_interceptor.clone().call(req)?;
            auth_interceptor.clone().call(req)
        },
    );

    let response_with_metadata =
        match tokio::time::timeout(STARTUP_ADOPTION_TIMEOUT, client.start(())).await {
            Ok(Ok(response)) => response,
            Ok(Err(err)) => {
                return adoption_failure(format!("Failed to start gateway setup stream: {err}"));
            }
            Err(_) => {
                return adoption_failure(format!(
                    "Timed out connecting to gateway setup endpoint after {} seconds",
                    STARTUP_ADOPTION_TIMEOUT.as_secs()
                ));
            }
        };

    let gateway_version = response_with_metadata
        .metadata()
        .get(defguard_version::VERSION_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(defguard_version::Version::parse)
        .transpose()
        .unwrap_or(None);

    if let Some(gateway_version) = gateway_version {
        if gateway_version < MIN_GATEWAY_VERSION {
            return adoption_failure_with_logs(&mut log_rx);
        }
    } else {
        return adoption_failure_with_logs(&mut log_rx);
    }

    let mut response = response_with_metadata.into_inner();
    let log_reader_task = tokio::spawn(async move {
        loop {
            match response.message().await {
                Ok(Some(entry)) => {
                    let formatted = format_component_log(
                        &entry.timestamp,
                        &entry.level,
                        &entry.target,
                        &entry.message,
                    );
                    if log_tx.send(formatted).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    let _ = log_tx.send(format!("Error reading log: {err}"));
                    break;
                }
            }
        }
    });
    let _log_task_guard = TaskGuard(log_reader_task);

    let Some(hostname) = url.host_str() else {
        return adoption_failure_with_logs(&mut log_rx);
    };

    let csr_response = match client
        .get_csr(GatewayCertificateInfo {
            cert_hostname: hostname.to_string(),
        })
        .await
    {
        Ok(response) => response.into_inner(),
        Err(err) => {
            error!("Failed to get CSR from gateway: {err}");
            return adoption_failure_with_logs(&mut log_rx);
        }
    };

    let csr = match Csr::from_der(&csr_response.der_data) {
        Ok(csr) => csr,
        Err(err) => {
            error!("Failed to parse CSR: {err}");
            return adoption_failure_with_logs(&mut log_rx);
        }
    };

    let ca = match CertificateAuthority::from_cert_der_key_pair(&ca_cert_der, &ca_key_der) {
        Ok(ca) => ca,
        Err(err) => {
            return adoption_failure(format!("Failed to build certificate authority: {err}"));
        }
    };

    let cert = match ca.sign_csr(&csr) {
        Ok(cert) => cert,
        Err(err) => {
            error!("Failed to sign CSR: {err}");
            return adoption_failure_with_logs(&mut log_rx);
        }
    };

    if let Err(err) = client
        .send_cert(GatewayDerPayload {
            der_data: cert.der().to_vec(),
        })
        .await
    {
        error!("Failed to send certificate to gateway: {err}");
        return adoption_failure_with_logs(&mut log_rx);
    }

    let cert_info = match CertificateInfo::from_der(cert.der()) {
        Ok(info) => info,
        Err(err) => {
            error!("Failed to parse certificate info: {err}");
            return adoption_failure_with_logs(&mut log_rx);
        }
    };

    let mut logs = collect_stream_logs(&mut log_rx);
    if logs.is_empty() {
        logs = vec!["No runtime logs received from gateway component".to_string()];
    }

    (true, logs, Some(cert_info))
}

// Default WireGuard network address and port used when auto-adopting a gateway without an
// existing network.  The gateway's own gRPC host is reused as the WireGuard endpoint so peers
// can reach it.
const DEFAULT_AUTO_ADOPTION_NETWORK_ADDRESS: &str = "10.0.0.1/24";
const DEFAULT_AUTO_ADOPTION_WIREGUARD_PORT: i32 = 51820;

async fn process_startup_auto_adoption(
    pool: &PgPool,
    component: SetupAutoAdoptionComponent,
    endpoint: &str,
) -> Result<(), anyhow::Error> {
    let (host, port) = parse_host_port(endpoint)?;

    let (status, logs, cert_info) = match component {
        SetupAutoAdoptionComponent::Edge => run_edge_adoption_attempt(pool, &host, port).await,
        SetupAutoAdoptionComponent::Gateway => run_gateway_adoption_attempt(&host, port).await,
    };

    // On successful adoption: create the relevant DB records.
    if status {
        match component {
            SetupAutoAdoptionComponent::Gateway => {
                if let Some(cert_info) = cert_info {
                    if let Err(err) =
                        create_network_and_gateway(pool, &host, port, GATEWAY_NAME, cert_info).await
                    {
                        warn!(
                            "Gateway adoption TLS handshake succeeded but failed to persist \
                            network/gateway records: {err}"
                        );
                    }
                }
            }
            SetupAutoAdoptionComponent::Edge => {
                if let Some(cert_info) = cert_info {
                    if let Err(err) = create_proxy(pool, &host, port, PROXY_NAME, cert_info).await {
                        warn!(
                            "Edge adoption TLS handshake succeeded but failed to persist \
                            proxy record: {err}"
                        );
                    }
                }
            }
        }
    }

    let mut auto_state = AutoAdoptionWizardState::get(pool)
        .await
        .context("Failed to load auto-adoption wizard state")?
        .unwrap_or_default();

    auto_state.adoption_result.insert(
        component,
        AutoAdoptionComponentResult {
            success: status,
            logs: logs.clone(),
            updated_at: chrono::Utc::now().naive_utc(),
        },
    );
    auto_state.save(pool).await?;

    Ok(())
}

/// Creates a [`WireguardNetwork`] (location) pre-filled with auto-generated defaults and then
/// creates the associated [`Gateway`] record with the certificate data obtained during adoption.
async fn create_network_and_gateway(
    pool: &PgPool,
    host: &str,
    grpc_port: u16,
    common_name: &str,
    cert_info: CertificateInfo,
) -> Result<(), anyhow::Error> {
    // Re-use or create the network location.
    let network = if let Some(existing) = WireguardNetwork::find_by_name(pool, common_name)
        .await
        .context("Failed to query network by name")?
        .and_then(|mut v| {
            if v.is_empty() {
                None
            } else {
                Some(v.remove(0))
            }
        }) {
        info!(
            "Auto-adoption: reusing existing network location name={common_name} \
id={} for new gateway",
            existing.id
        );
        existing
    } else {
        let network_address: IpNetwork = DEFAULT_AUTO_ADOPTION_NETWORK_ADDRESS
            .parse()
            .context("Failed to parse default auto-adoption network address")?;

        let mut transaction = pool.begin().await.context("Failed to begin transaction")?;
        let network = WireguardNetwork::new(
            common_name.to_string(),
            vec![network_address],
            DEFAULT_AUTO_ADOPTION_WIREGUARD_PORT,
            host.to_string(),
            None,
            DEFAULT_WIREGUARD_MTU,
            0,
            Vec::new(),
            DEFAULT_KEEPALIVE_INTERVAL,
            DEFAULT_DISCONNECT_THRESHOLD,
            false,
            false,
            LocationMfaMode::Disabled,
            ServiceLocationMode::Disabled,
        )
        .save(&mut *transaction)
        .await
        .context("Failed to save auto-adopted WireguardNetwork")?;

        network
            .add_all_allowed_devices(&mut transaction)
            .await
            .context("Failed to assign IPs for existing devices in auto-adopted network")?;

        transaction
            .commit()
            .await
            .context("Failed to commit auto-adoption network transaction")?;

        info!(
            "Auto-adoption: created network location name={common_name} id={}",
            network.id
        );
        network
    };

    // Avoid duplicate gateway records for the same address:port.
    if let Some(existing) = Gateway::find_by_url(pool, host, grpc_port)
        .await
        .context("Failed to query existing gateways")?
    {
        info!(
            "Auto-adoption: gateway already registered at {host}:{grpc_port} (id={}); \
            skipping gateway record creation",
            existing.id
        );
        return Ok(());
    }

    let mut gateway = Gateway::new(
        network.id,
        common_name,
        host,
        i32::from(grpc_port),
        "Automatic setup",
    );
    gateway.certificate = Some(cert_info.serial);
    gateway.certificate_expiry = Some(cert_info.not_after);

    gateway
        .save(pool)
        .await
        .context("Failed to save auto-adopted Gateway")?;

    info!(
        "Auto-adoption: created gateway record name={common_name} address={host}:{grpc_port} \
        network_id={}",
        network.id
    );

    Ok(())
}

/// Creates a [`Proxy`] record in the database after a successful edge adoption.
async fn create_proxy(
    pool: &PgPool,
    host: &str,
    port: u16,
    common_name: &str,
    cert_info: CertificateInfo,
) -> Result<(), anyhow::Error> {
    if let Some(existing) = Proxy::find_by_address_port(pool, host, i32::from(port))
        .await
        .context("Failed to query existing proxies")?
    {
        info!(
            "Auto-adoption: proxy already registered at {host}:{port} (id={}); \
            skipping proxy record creation",
            existing.id
        );
        return Ok(());
    }

    let mut proxy = Proxy::new(common_name, host, i32::from(port), "Automatic setup");
    proxy.certificate = Some(cert_info.serial);
    proxy.certificate_expiry = Some(cert_info.not_after);

    proxy
        .save(pool)
        .await
        .context("Failed to save auto-adopted Proxy")?;

    info!("Auto-adoption: created proxy record name={common_name} address={host}:{port}");

    Ok(())
}

/// Stores and updates startup auto-adoption states for components requested via CLI flags.
pub async fn attempt_auto_adoption(
    pool: &PgPool,
    config: &DefGuardConfig,
) -> Result<(), anyhow::Error> {
    let mut auto_state = AutoAdoptionWizardState::get(pool)
        .await
        .context("Failed to load auto-adoption wizard state")?
        .unwrap_or_default();

    let edge_already_succeeded = auto_state
        .adoption_result
        .get(&SetupAutoAdoptionComponent::Edge)
        .is_some_and(|result| result.success);
    let gateway_already_succeeded = auto_state
        .adoption_result
        .get(&SetupAutoAdoptionComponent::Gateway)
        .is_some_and(|result| result.success);

    let should_run_edge = config.adopt_edge.is_some() && !edge_already_succeeded;
    let should_run_gateway = config.adopt_gateway.is_some() && !gateway_already_succeeded;
    let auto_mode_requested = should_run_edge || should_run_gateway;
    if auto_mode_requested {
        ensure_ca_for_auto_adoption(pool).await?;
    }

    if let Some(endpoint) = &config.adopt_edge {
        if edge_already_succeeded {
            info!(
                "Skipping startup auto-adoption for Edge component endpoint={endpoint} as it was already completed"
            );
        } else {
            info!("Starting startup auto-adoption for Edge component endpoint={endpoint}");
            if let Err(err) =
                process_startup_auto_adoption(pool, SetupAutoAdoptionComponent::Edge, endpoint)
                    .await
            {
                auto_state.adoption_result.insert(
                    SetupAutoAdoptionComponent::Edge,
                    AutoAdoptionComponentResult {
                        success: false,
                        logs: vec![format!("Startup auto-adoption failed: {err}")],
                        updated_at: chrono::Utc::now().naive_utc(),
                    },
                );
                auto_state.save(pool).await?;
            } else {
                info!("Startup auto-adoption for Edge component completed endpoint={endpoint}");
            }
        }
    }

    if let Some(endpoint) = &config.adopt_gateway {
        if gateway_already_succeeded {
            info!(
                "Skipping startup auto-adoption for Gateway component endpoint={endpoint} as it was already completed"
            );
        } else {
            info!("Starting startup auto-adoption for Gateway component endpoint={endpoint}");
            if let Err(err) =
                process_startup_auto_adoption(pool, SetupAutoAdoptionComponent::Gateway, endpoint)
                    .await
            {
                auto_state.adoption_result.insert(
                    SetupAutoAdoptionComponent::Gateway,
                    AutoAdoptionComponentResult {
                        success: false,
                        logs: vec![format!("Startup auto-adoption failed: {err}")],
                        updated_at: chrono::Utc::now().naive_utc(),
                    },
                );
                auto_state.save(pool).await?;
            } else {
                info!("Startup auto-adoption for Gateway component completed endpoint={endpoint}");
            }
        }
    }

    Ok(())
}
