use std::{
    collections::{HashMap, VecDeque},
    convert::Infallible,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use axum::{
    Extension,
    extract::{Path, Query},
    response::sse::{Event, KeepAlive, Sse},
};
use chrono::NaiveDateTime;
use defguard_certs::der_to_pem;
use defguard_common::{
    VERSION,
    auth::claims::Claims,
    db::{
        Id,
        models::{
            Certificates, Settings,
            certificates::ProxyCertSource,
            gateway::Gateway,
            initial_setup_wizard::{InitialSetupState, InitialSetupStep},
            proxy::Proxy,
            wizard::Wizard,
        },
    },
    types::proxy::ProxyControlMessage,
    utils::strip_scheme,
};
use defguard_grpc_tls::certs::proxy_mtls_channel;
use defguard_proto::{
    common::{CertBundle, CertificateInfo},
    gateway::gateway_setup_client::GatewaySetupClient,
    proxy::{
        AcmeChallenge, AcmeLogs, AcmeStep, acme_issue_event, proxy_client::ProxyClient,
        proxy_setup_client::ProxySetupClient,
    },
};
use defguard_version::{Version, client::ClientVersionInterceptor};
use futures::Stream;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::{
    sync::{
        mpsc::{Sender, UnboundedReceiver, UnboundedSender, unbounded_channel},
        oneshot, watch,
    },
    time::{Instant, sleep_until, timeout},
};
use tokio_stream::StreamExt;
use tonic::{
    Request, Status,
    service::Interceptor,
    transport::{Certificate, ClientTlsConfig, Endpoint},
};
use tracing::Instrument;

use crate::{
    auth::{AdminOrSetupRole, SessionInfo},
    enterprise::is_enterprise_license_active,
    setup_logs::scope_setup_logs,
    version::{MIN_GATEWAY_VERSION, MIN_PROXY_VERSION},
};

const TOKEN_CLIENT_ID: &str = "Defguard Core";
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

/// Guard that aborts a tokio task when dropped
struct TaskGuard(tokio::task::JoinHandle<()>);

impl Drop for TaskGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProxySetupRequest {
    pub ip_or_domain: String,
    pub grpc_port: u16,
    pub common_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GatewaySetupRequest {
    pub common_name: String,
    pub ip_or_domain: String,
    pub grpc_port: u16,
}

#[derive(Debug, Serialize, Copy, Clone)]
#[serde(tag = "step", content = "data")]
pub enum SetupStep {
    CheckingConfiguration,
    CheckingAvailability,
    CheckingVersion,
    ObtainingCsr,
    SigningCertificate,
    ConfiguringTls,
    Done,
}

#[derive(Serialize)]
pub struct SetupResponse {
    #[serde(flatten)]
    pub step: SetupStep,
    /// Gateway or Edge version.
    pub version: Option<String>,
    pub message: Option<String>,
    pub logs: Option<Vec<String>>,
    pub error: bool,
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
            format!("Bearer {}", self.token).parse().unwrap(),
        );
        Ok(request)
    }
}

fn fallback_message(err: &str, last_step: SetupStep) -> String {
    format!(
        r#"{{"step":"{last_step:?}","message":"Failed to serialize error response: {err}","error":true}}"#,
    )
}

fn error_message(message: &str, last_step: SetupStep, logs: Option<Vec<String>>) -> Event {
    let response = SetupResponse {
        step: last_step,
        version: None,
        message: Some(message.to_string()),
        logs,
        error: true,
    };

    Event::default().data(match serde_json::to_string(&response) {
        Ok(body) => body,
        Err(e) => fallback_message(&e.to_string(), last_step),
    })
}

fn set_step_message(next_step: SetupStep) -> Event {
    let response = SetupResponse {
        step: next_step,
        version: None,
        message: None,
        logs: None,
        error: false,
    };

    match serde_json::to_string(&response) {
        Ok(body) => Event::default().data(body),
        Err(e) => Event::default().data(fallback_message(&e.to_string(), next_step)),
    }
}

struct SetupFlow {
    last_step: SetupStep,
    log_buffer: Arc<Mutex<VecDeque<String>>>,
    log_rx: UnboundedReceiver<String>,
}

impl SetupFlow {
    fn new(log_rx: UnboundedReceiver<String>, log_buffer: Arc<Mutex<VecDeque<String>>>) -> Self {
        Self {
            last_step: SetupStep::CheckingConfiguration,
            log_buffer,
            log_rx,
        }
    }

    fn step(&mut self, next_step: SetupStep) -> Event {
        self.last_step = next_step;
        set_step_message(next_step)
    }

    fn error(&mut self, message: &str) -> Event {
        error!("{message}");

        let mut collected_logs = {
            let mut guard = self
                .log_buffer
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            std::mem::take(&mut *guard).into_iter().collect::<Vec<_>>()
        };
        while let Ok(log) = self.log_rx.try_recv() {
            collected_logs.push(log);
        }
        let logs = if collected_logs.is_empty() {
            None
        } else {
            Some(collected_logs)
        };

        error_message(message, self.last_step, logs)
    }
}

/// This is the endpoint responsible for the whole edge proxy TLS setup flow.
/// It uses Server-Sent Events (SSE) to stream progress updates back to the frontend in real-time.
// This is a get request, since HTML's EventSource only supports GET
pub async fn setup_proxy_tls_stream(
    _admin: AdminOrSetupRole,
    Query(request): Query<ProxySetupRequest>,
    session: SessionInfo,
    Extension(pool): Extension<PgPool>,
    proxy_control_tx: Option<Extension<Sender<ProxyControlMessage>>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (log_tx, log_rx) = unbounded_channel::<String>();
    let log_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let inner_log_buffer = Arc::clone(&log_buffer);
    let inner_stream = async_stream::stream! {
        let mut flow = SetupFlow::new(log_rx, inner_log_buffer.clone());

        // check if tries to connect more then 1 proxy without active enterprise license
        if !is_enterprise_license_active() {
            match Proxy::list(&pool).await {
                Ok(current_proxies) => {
                    if !current_proxies.is_empty() {
                        yield Ok(flow.error(
                            "Enterprise license is required for connecting more than one Edge.",
                        ));
                        return;
                    }
                }
                Err(e) => {
                    yield Ok(flow.error(&format!("Failed to query existing proxies: {e}")));
                    return;
                }
            }
        }

        debug!("License check passed");

        let ip_or_domain = strip_scheme(&request.ip_or_domain);

        // Step 1: Check configuration
        yield Ok(flow.step(SetupStep::CheckingConfiguration));
        match Proxy::find_by_address_port(&pool, ip_or_domain, i32::from(request.grpc_port)).await {
            Ok(Some(proxy)) => {
                yield Ok(flow.error(&format!(
                    "Edge with address {ip_or_domain}:{} is already registered with name \"{}\".",
                     request.grpc_port, proxy.name
                )));
                return;
            }
            Ok(None) => {
                debug!(
                    "Verified no existing Edge registration for {ip_or_domain}:{}",
                     request.grpc_port
                );
            }
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to query existing Edge: {e}")));
                return;
            }
        }

        debug!("Configuration check passed");

        let url_str = format!("http://{ip_or_domain}:{}",  request.grpc_port);
        let url = match Url::parse(&url_str) {
            Ok(u) => u,
            Err(e) => {
                yield Ok(flow.error(&format!("Invalid URL: {e}")));
                return;
            }
        };

        debug!("Successfully validated Edge address: {url_str}");

        let endpoint = match Endpoint::from_shared(url_str) {
            Ok(e) => e,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to create endpoint: {e}")));
                return;
            }
        };

        let endpoint = endpoint
            .http2_keep_alive_interval(Duration::from_secs(5))
            .tcp_keepalive(Some(Duration::from_secs(5)))
            .keep_alive_while_idle(true);

        debug!("Connection endpoint configured with keep-alive settings");

        let certs = match Certificates::get(&pool).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                yield Ok(flow.error("CA certificate not found"));
                return;
            }
            Err(err) => {
                yield Ok(flow.error(&format!("Failed to load certificates: {err}")));
                return;
            }
        };
        let Some(ca_cert_der) = certs.ca_cert_der.clone() else {
            yield Ok(flow.error("CA certificate not found in settings"));
            return;
        };

        let cert_pem = match der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate) {
            Ok(pem) => pem,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to convert CA cert DER to PEM: {e}")));
                return;
            }
        };
        let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(&cert_pem));

        debug!("Loaded CA certificate for secure communication");

        let endpoint = match endpoint.tls_config(tls) {
            Ok(e) => e,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to configure TLS for endpoint: {e}")));
                return;
            }
        };

        debug!(
            "Prepared secure connection endpoint for Edge at {ip_or_domain}:{}",
             request.grpc_port
        );

        let version = match Version::parse(VERSION) {
            Ok(v) => v,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to parse version: {e}")));
                return;
            }
        };

        // Step 2: Check availability
        yield Ok(flow.step(SetupStep::CheckingAvailability));

        let version_clone = version.clone();
        let token = match Claims::new(
            defguard_common::auth::claims::ClaimsType::Gateway,
            url.to_string(),
            TOKEN_CLIENT_ID.to_string(),
            u32::MAX.into(),
        )
        .to_jwt()
        {
            Ok(token) => token,
            Err(err) => {
                yield Ok(flow.error(&format!("Failed to generate setup token: {err}")));
                return;
            }
        };

        debug!("Generated secure setup token for Edge authentication");

        let version_interceptor = ClientVersionInterceptor::new(version);
        let auth_interceptor = AuthInterceptor::new(token);
        let mut client = ProxySetupClient::with_interceptor(endpoint.connect_lazy(), move |mut req: Request<()>| {
            req = version_interceptor.clone().call(req)?;
            auth_interceptor.clone().call(req)
        });

        debug!(
            "Initiating connection to Edge at {ip_or_domain}:{}",
             request.grpc_port
        );

        let response_with_metadata = match timeout(CONNECTION_TIMEOUT, client.start(())).await {
            Ok(Ok(response)) => response,
            Ok(Err(status)) => {
                let error_msg = status.message();
                if error_msg.contains("h2 protocol error") || error_msg.contains("http2 error") {
                    yield Ok(flow.error(&format!(
                        "Failed to connect to Edge at {ip_or_domain}:{}: {error_msg}. This may indicate that \
                        the Edge is already configured with TLS. Please check if the Edge has \
                        already been set up.",
                         request.grpc_port
                    )));
                } else {
                    yield Ok(flow.error(&format!(
                        "Failed to connect to Edge at {ip_or_domain}:{}: {error_msg}. Please ensure the \
                        address and port are correct and that the Edge component is running.",
                         request.grpc_port
                    )));
                }
                return;
            }
            Err(_) => {
                yield Ok(flow.error(&format!(
                    "Connection to Edge at {ip_or_domain}:{} timed out after {} seconds",
                    request.grpc_port, CONNECTION_TIMEOUT.as_secs()
                )));
                return;
            }
        };

        debug!("Successfully connected to Edge");

        // Step 3: Check version
        yield Ok(flow.step(SetupStep::CheckingVersion));

        let proxy_version = response_with_metadata
            .metadata()
            .get(defguard_version::VERSION_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(defguard_version::Version::parse)
            .transpose()
            .unwrap_or(None);

        debug!("Edge metadata: {:?}", response_with_metadata.metadata());
        debug!("Edge version: {proxy_version:?}");

        if let Some(proxy_version) = proxy_version {
            if proxy_version < MIN_PROXY_VERSION {
                yield Ok(flow.error(&format!(
                    "Edge version {proxy_version} is older than Core version {version_clone}. \
                    Please update the Edge component.",
                )));
                return;
            }

            debug!(
                "Edge version {} is compatible with Core version {}",
                proxy_version, version_clone
            );

            let response = SetupResponse {
                step: SetupStep::CheckingVersion,
                version: Some(proxy_version.to_string()),
                message: None,
                logs: None,
                error: false,
            };

            match serde_json::to_string(&response) {
                Ok(body) => yield Ok(Event::default().data(body)),
                Err(e) => {
                    yield Ok(flow.error(&format!("Failed to serialize version response: {e}")));
                    return;
                }
            }
        } else {
            yield Ok(flow.error("Failed to determine Edge version"));
            return;
        }

        let mut response = response_with_metadata.into_inner();
        let spawn_log_buffer = inner_log_buffer.clone();
        let log_reader_task = tokio::spawn(
            scope_setup_logs(spawn_log_buffer, async move {
                    while let Some(log_entry) = response.next().await {
                        match log_entry {
                            Ok(entry) => {
                                let level = entry
                                    .level
                                    .strip_prefix("Level(")
                                    .and_then(|s| s.strip_suffix(")"))
                                    .unwrap_or(&entry.level)
                                    .to_uppercase();

                                let formatted = format!(
                                    "{} {} {}: message={}",
                                    entry.timestamp, level, entry.target, entry.message
                                );
                                if log_tx.send(formatted).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                let _ = log_tx.send(format!("Error reading log: {e}"));
                                break;
                            }
                        }
                    }
                })
                .instrument(tracing::Span::current()),
        );

        let _log_task_guard = TaskGuard(log_reader_task);

        // Step 4: Obtain CSR
        yield Ok(flow.step(SetupStep::ObtainingCsr));

        let Some(hostname) = url.host_str() else {
            yield Ok(flow.error("URL does not have a valid host"));
            return;
        };

        let csr_response = match client.get_csr(CertificateInfo { cert_hostname: hostname.to_string() }).await {
            Ok(r) => r.into_inner(),
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to obtain CSR: {e}")));
                return;
            }
        };

        let csr = match defguard_certs::Csr::from_der(&csr_response.der_data) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to parse CSR: {e}")));
                return;
            }
        };

        debug!("Received certificate signing request from Edge for hostname: {hostname}");

        // Step 5: Sign certificate
        yield Ok(flow.step(SetupStep::SigningCertificate));

        let Some(ca_cert_der) = certs.ca_cert_der else {
            yield Ok(flow.error("CA certificate not found in settings"));
            return;
        };
        let Some(ca_key_pair) = certs.ca_key_der else {
            yield Ok(flow.error("CA key pair not found in settings"));
            return;
        };

        let ca = match defguard_certs::CertificateAuthority::from_cert_der_key_pair(&ca_cert_der, &ca_key_pair) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to create CA: {e}")));
                return;
            }
        };

        debug!("Certificate authority loaded and ready to sign certificates");

        let cert = match ca.sign_server_cert(&csr) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to sign CSR: {e}")));
                return;
            }
        };

        debug!("Successfully signed certificate for Edge");

        // Step 6: Configure TLS
        yield Ok(flow.step(SetupStep::ConfiguringTls));

        let core_client = match ca.issue_core_client_cert(&request.common_name) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to issue Core client certificate: {e}")));
                return;
            }
        };

        let bundle = CertBundle {
            component_cert_der: cert.der().to_vec(),
            ca_cert_der: ca_cert_der.clone(),
            core_client_cert_der: core_client.cert_der.clone(),
        };
        if let Err(e) = client.send_cert(bundle).await {
            yield Ok(flow.error(&format!("Failed to send certificate: {e}")));
            return;
        }

        debug!("Certificate successfully delivered to Edge");

        let defguard_certs::CertificateInfo { not_after: expiry, serial, .. } =
            match defguard_certs::CertificateInfo::from_der(cert.der()) {
                Ok(dt) => dt,
                Err(err) => {
                    yield Ok(flow.error(&format!("Failed to get certificate expiry: {err}")));
                    return;
                }
            };

        debug!("Certificate expiry date determined: {expiry}");

        let mut proxy = Proxy::new(
            request.common_name.as_str(),
            ip_or_domain,
            i32::from(request.grpc_port),
            session.user.fullname().as_str(),
        );
        proxy.certificate_serial = Some(serial);
        proxy.certificate_expiry = Some(expiry);
        proxy.core_client_cert_der = Some(core_client.cert_der);
        proxy.core_client_cert_key_der = Some(core_client.key_der);
        proxy.core_client_cert_expiry = Some(core_client.expiry);

        let proxy = match proxy.save(&pool).await {
            Ok(p) => p,
            Err(err) => {
                yield Ok(flow.error(&format!("Failed to save Edge to database: {err}")));
                return;
            }
        };

        debug!(
            "Edge '{}' registered successfully with ID: {}",
            request.common_name, proxy.id
        );
        debug!("Establishing connection to newly configured Edge");

        if let Some(proxy_control_tx) = proxy_control_tx {
            if let Err(err) = proxy_control_tx.send(ProxyControlMessage::StartConnection(proxy.id)).await {
                yield Ok(flow.error(&format!(
                    "Failed send message to connect to Edge after setup: {err}"
                )));
                return;
            }
        } else {
            debug!("Edge control channel not available; skipping connection initiation");
        }

        info!("Edge setup completed successfully");

        match Wizard::get(&pool).await {
            Ok(wizard) => {
                if !wizard.completed {
                    let state = InitialSetupState {
                        step: InitialSetupStep::InternalUrlSettings,
                    };
                    if let Err(err) = state.save(&pool).await {
                        yield Ok(flow.error(&format!("Failed to update setup step in wizard: {err}")));
                        return;
                    }
                    debug!("Initial setup step advanced to 'InternalUrlSettings'");
                }
            }
            Err(err) => {
                yield Ok(flow.error(&format!("Failed to fetch wizard state: {err}")));
                return;
            }
        }

        // Step 7: Done
        yield Ok(flow.step(SetupStep::Done));
    };

    let adoption_span = tracing::info_span!("proxy_adoption");
    let stream = async_stream::stream! {
        tokio::pin!(inner_stream);
        while let Some(item) = scope_setup_logs(log_buffer.clone(), inner_stream.next())
            .instrument(adoption_span.clone())
            .await
        {
            yield item;
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// This is the endpoint responsible for the whole gateway TLS setup flow.
/// It uses Server-Sent Events (SSE) to stream progress updates back to the frontend in real-time.
// This is a get request, since HTML's EventSource only supports GET
pub async fn setup_gateway_tls_stream(
    _admin: AdminOrSetupRole,
    session: SessionInfo,
    Query(request): Query<GatewaySetupRequest>,
    Path(network_id): Path<Id>,
    Extension(pool): Extension<PgPool>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (log_tx, log_rx) = unbounded_channel::<String>();
    let log_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let inner_log_buffer = Arc::clone(&log_buffer);
    let inner_stream = async_stream::stream! {
        let mut flow = SetupFlow::new(log_rx, inner_log_buffer.clone());

        // check if tries to add more then 1 gateway to network without enterprise license
        if !is_enterprise_license_active() {
            match Gateway::find_by_location_id(&pool, network_id).await {
                Ok(gateways) => {
                    if !gateways.is_empty() {
                        yield Ok(flow.error("Enterprise license is required."));
                        return;
                    }
                },
                Err(e) => {
                    yield Ok(flow.error(&format!("Reading current gateways failed! error {e}")));
                    return;
                }
            }
        }

        // Step 1: Check configuration
        yield Ok(
            flow.step(SetupStep::CheckingConfiguration)
        );

        let ip_or_domain = strip_scheme(&request.ip_or_domain);

        match Gateway::find_by_url(&pool, ip_or_domain, request.grpc_port).await {
            Ok(Some(gateway)) => {
               yield Ok(flow.error(&format!("A Gateway with URL {ip_or_domain}:{} is already registered with \
                   name \"{}\".",  request.grpc_port, gateway.name)));
               return;
            }
            Ok(None) => {
                debug!("Verified no existing Gateway registration for {ip_or_domain}:{}",
                    request.grpc_port);
            },
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to query existing Gateway: {e}")));
                return;
            }
        }

        let url_str = format!("http://{ip_or_domain}:{}",  request.grpc_port);
        let url = match Url::parse(&url_str) {
            Ok(u) => u,
            Err(e) => {
                yield Ok(flow.error(&format!("Invalid URL: {e}")));
                return;
            }
        };

        debug!("Successfully validated Gateway address: {url_str}");

        let endpoint = match Endpoint::from_shared(url.to_string()) {
            Ok(e) => e,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to create endpoint: {e}")));
                return;
            }
        };

        let endpoint = endpoint
            .http2_keep_alive_interval(Duration::from_secs(5))
            .tcp_keepalive(Some(Duration::from_secs(5)))
            .keep_alive_while_idle(true);

        debug!("Connection endpoint configured with keep-alive settings");

        let certs = match Certificates::get(&pool).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                yield Ok(flow.error("CA certificate not found"));
                return;
            }
            Err(err) => {
                yield Ok(flow.error(&format!("Failed to load certificates: {err}")));
                return;
            }
        };
        let Some(ca_cert_der) = certs.ca_cert_der.clone() else {
            yield Ok(flow.error("CA certificate not found in settings"));
            return;
        };

        let cert_pem = match der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate) {
            Ok(pem) => pem,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to convert CA cert DER to PEM: {e}")));
                return;
            }
        };
        let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(&cert_pem));

        debug!("Loaded CA certificate for secure communication");

        let endpoint = match endpoint.tls_config(tls) {
            Ok(e) => e,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to configure TLS for endpoint: {e}")));
                return;
            }
        };

        debug!("Prepared secure connection endpoint for Gateway at {ip_or_domain}:{}",
            request.grpc_port);

        let version = match Version::parse(VERSION) {
            Ok(v) => v,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to parse version: {e}")));
                return;
            }
        };

        // Step 2: Check availability
        yield Ok(
            flow.step(SetupStep::CheckingAvailability)
        );

        let version_clone = version.clone();

        let token = match Claims::new(
            defguard_common::auth::claims::ClaimsType::Gateway,
            url.to_string(),
            TOKEN_CLIENT_ID.to_string(),
            u32::MAX.into(),
        )
        .to_jwt()
        {
            Ok(token) => token,
            Err(err) => {
                yield Ok(flow.error(&format!("Failed to generate setup token: {err}")));
                return;
            }
        };

        debug!("Generated secure setup token for Gateway authentication");

        let version_interceptor = ClientVersionInterceptor::new(version);
        let auth_interceptor = AuthInterceptor::new(token);

        let mut client = GatewaySetupClient::with_interceptor(
            endpoint.connect_lazy(),
            move |mut req: Request<()>| {
            req = version_interceptor.clone().call(req)?;
            auth_interceptor.clone().call(req)
            }
        );

        debug!("Initiating connection to Gateway at {ip_or_domain}:{}",
            request.grpc_port);

        let response_with_metadata = match timeout(
            CONNECTION_TIMEOUT,
            client.start(())
        ).await {
            Ok(Ok(response)) => response,
            Ok(Err(status)) => {
                let error_msg = status.message();
                if error_msg.contains("h2 protocol error") || error_msg.contains("http2 error") {
                    yield Ok(flow.error(&format!(
                        "Failed to connect to Gateway at {ip_or_domain}:{}: {error_msg}. This may indicate \
                        that the Gateway is already configured with TLS. Please, check if the \
                        Gateway has already been set up.",
                         request.grpc_port,
                    )));
                } else {
                    yield Ok(flow.error(&format!(
                        "Failed to connect to Gateway at {ip_or_domain}:{}: {error_msg}. Please ensure the \
                        address and port are correct and that the Gateway is running.",
                         request.grpc_port
                    )));
                }
                return;
            }
            Err(_) => {
                yield Ok(flow.error(&format!(
                    "Connection to Gateway at {ip_or_domain}:{} timed out after 10 seconds.",
                     request.grpc_port
                )));
                return;
            }
        };

        debug!("Successfully connected to Gateway");

        // Step 3: Check version
        yield Ok(flow.step(SetupStep::CheckingVersion));

        let gateway_version = response_with_metadata
            .metadata()
            .get(defguard_version::VERSION_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(defguard_version::Version::parse)
            .transpose()
            .unwrap_or(None);

        debug!("Gateway metadata: {:?}", response_with_metadata.metadata());
        debug!("Gateway version: {gateway_version:?}");

        if let Some(gateway_version) = gateway_version {
            if gateway_version < MIN_GATEWAY_VERSION {
                yield Ok(flow.error(&format!(
                    "Gateway version {gateway_version} is older than Core version {version_clone}. \
                    Please update the Gateway component.",
                )));
                return;
            }

            debug!("Gateway version {gateway_version} is compatible with Core version \
                {version_clone}");

            let response = SetupResponse {
                step: SetupStep::CheckingVersion,
                version: Some(gateway_version.to_string()),
                message: None,
                logs: None,
                error: false,
            };

            match serde_json::to_string(&response) {
                Ok(body) => {
                    yield Ok(
                        Event::default().data(body)
                    );
                },
                Err(e) => {
                    yield Ok(flow.error(&format!("Failed to serialize version response: {e}")));
                    return;
                }
            }
        } else {
            yield Ok(flow.error("Failed to determine Gateway version"));
            return;
        }

        let mut response = response_with_metadata.into_inner();

        let spawn_log_buffer = inner_log_buffer.clone();
        let log_reader_task = tokio::spawn(
            scope_setup_logs(spawn_log_buffer, async move {
                while let Some(log_entry) = response.next().await {
                    match log_entry {
                        Ok(entry) => {
                            let level = entry
                                .level
                                .strip_prefix("Level(")
                                .and_then(|s| s.strip_suffix(")"))
                                .unwrap_or(&entry.level)
                                .to_uppercase();

                            let formatted = format!(
                                "{} {level} {}: message={}",
                                entry.timestamp, entry.target, entry.message
                            );
                            if log_tx.send(formatted).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = log_tx.send(format!("Error reading log: {e}"));
                            break;
                        }
                    }
                }
            })
            .instrument(tracing::Span::current()),
        );

        // Create guard to ensure task is aborted on all exit paths
        let _log_task_guard = TaskGuard(log_reader_task);

        // Step 4: Obtain CSR
        yield Ok(flow.step(SetupStep::ObtainingCsr));

        let Some(hostname) = url.host_str() else {
            yield Ok(flow.error("URL does not have a valid host"));
            return;
        };

        let csr_response = match client
            .get_csr(CertificateInfo {
                cert_hostname: hostname.to_string(),
            })
            .await
        {
            Ok(r) => r.into_inner(),
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to obtain CSR: {e}")));
                return;
            }
        };

        let csr = match defguard_certs::Csr::from_der(&csr_response.der_data) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to parse CSR: {e}")));
                return;
            }
        };

        debug!("Received certificate signing request from Gateway for hostname: {hostname}");

        // Step 5: Sign certificate
        yield Ok(flow.step(SetupStep::SigningCertificate));

        let Some(ca_cert_der) = certs.ca_cert_der else {
            yield Ok(flow.error("CA certificate not found in settings"));
            return;
        };

        let Some(ca_key_pair) = certs.ca_key_der else {
            yield Ok(flow.error("CA key pair not found in settings"));
            return;
        };

        let ca = match defguard_certs::CertificateAuthority::from_cert_der_key_pair(
            &ca_cert_der,
            &ca_key_pair,
        ) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to create CA: {e}")));
                return;
            }
        };

        debug!("Certificate authority loaded and ready to sign certificates");

        let cert = match ca.sign_server_cert(&csr) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to sign CSR: {e}")));
                return;
            }
        };

        debug!("Successfully signed certificate for Gateway");

        // Step 6: Configure TLS
        yield Ok(flow.step(SetupStep::ConfiguringTls));

        let core_client = match ca.issue_core_client_cert(&request.common_name) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to issue Core client certificate: {e}")));
                return;
            }
        };

        let bundle = CertBundle {
            component_cert_der: cert.der().to_vec(),
            ca_cert_der: ca_cert_der.clone(),
            core_client_cert_der: core_client.cert_der.clone(),
        };
        if let Err(e) = client.send_cert(bundle).await {
            yield Ok(flow.error(&format!("Failed to send certificate: {e}")));
            return;
        }

        debug!("Certificate successfully delivered to Gateway");

        let defguard_certs::CertificateInfo {
            not_after: expiry,
            serial,
            ..
        } = match defguard_certs::CertificateInfo::from_der(cert.der()) {
            Ok(dt) => {
            dt
            },
            Err(err) => {
            yield Ok(flow.error(&format!("Failed to get certificate expiry: {err}")));
            return;
            }
        };

        debug!("Certificate expiry date determined: {expiry}");

        let mut gateway = Gateway::new(
            network_id,
            request.common_name,
            ip_or_domain.to_owned(),
            request.grpc_port.into(),
            session.user.fullname(),
        );

        gateway.certificate_serial = Some(serial);
        gateway.certificate_expiry = Some(expiry);
        gateway.core_client_cert_der = Some(core_client.cert_der);
        gateway.core_client_cert_key_der = Some(core_client.key_der);
        gateway.core_client_cert_expiry = Some(core_client.expiry);

        if let Err(err) = gateway.save(&pool).await {
            yield Ok(flow.error(&format!("Failed to save Gateway to database: {err}")));
            return;
        }

        debug!("Gateway setup completed successfully");

        // Step 7: Done
        yield Ok(flow.step(SetupStep::Done));
    };

    let adoption_span = tracing::info_span!("gateway_adoption");
    let stream = async_stream::stream! {
        tokio::pin!(inner_stream);
        while let Some(item) = scope_setup_logs(log_buffer.clone(), inner_stream.next())
            .instrument(adoption_span.clone())
            .await
        {
            yield item;
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Maximum time (seconds) allowed for the ACME flow to complete end-to-end.
const ACME_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Serialize)]
struct AcmeSetupResponse {
    step: &'static str,
    error: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logs: Option<Vec<String>>,
}

fn acme_event(step: &'static str) -> Event {
    let body = serde_json::to_string(&AcmeSetupResponse {
        step,
        error: false,
        message: None,
        logs: None,
    })
    .unwrap_or_else(|_| format!(r#"{{"step":"{step}","error":false}}"#));
    Event::default().data(body)
}

fn acme_error_event(step: &'static str, message: String, logs: Option<Vec<String>>) -> Event {
    let body = serde_json::to_string(&AcmeSetupResponse {
        step,
        error: true,
        message: Some(message.clone()),
        logs,
    })
    .unwrap_or_else(|_| format!(r#"{{"step":"{step}","error":true,"message":"{message}"}}"#));
    Event::default().data(body)
}

/// Maps a proto [`AcmeStep`] to the SSE step string expected by the frontend.
fn acme_step_name(step: AcmeStep) -> &'static str {
    match step {
        AcmeStep::Unspecified | AcmeStep::Connecting => "Connecting",
        AcmeStep::CheckingDomain => "CheckingDomain",
        AcmeStep::ValidatingDomain => "ValidatingDomain",
        AcmeStep::IssuingCertificate => "IssuingCertificate",
    }
}

fn parse_cert_expiry(cert_pem: &str) -> Option<NaiveDateTime> {
    let der = defguard_certs::parse_pem_certificate(cert_pem)
        .map_err(|e| warn!("Failed to parse ACME cert PEM for expiry: {e}"))
        .ok()?;
    defguard_certs::CertificateInfo::from_der(&der)
        .map(|info| info.not_after)
        .map_err(|e| warn!("Failed to extract expiry from ACME cert: {e}"))
        .ok()
}

fn public_proxy_hostname() -> Result<String, String> {
    let public_proxy_url = Settings::get_current_settings().public_proxy_url;
    let url = public_proxy_url.trim();

    if url.is_empty() {
        return Err(
            "Public Edge URL is not configured. Please re-submit the external URL settings \
             with a Let's Encrypt domain."
                .to_string(),
        );
    }

    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(ToString::to_string))
        .filter(|host| !host.is_empty())
        .ok_or_else(|| {
            "Public Edge URL is not configured with a valid hostname. Please re-submit the \
             external URL settings with a valid domain."
                .to_string()
        })
}

/// Connects to the proxy's permanent `Proxy` gRPC service and calls `TriggerAcme`.
///
/// Returns `(cert_pem, key_pem, account_credentials_json)` on success, or
/// `(error_message, log_lines)` on failure where `log_lines` are the proxy log entries
/// collected during the ACME run (sent by the proxy via an [`AcmeLogs`] event).
async fn call_proxy_trigger_acme(
    pool: &PgPool,
    proxy: &Proxy<Id>,
    domain: String,
    account_credentials_json: String,
    progress_tx: UnboundedSender<AcmeStep>,
) -> Result<(String, String, String), (String, Vec<String>)> {
    let certs = Certificates::get_or_default(pool)
        .await
        .map_err(|e| (format!("Failed to load certificates: {e}"), Vec::new()))?;
    let ca_cert_der = certs.ca_cert_der.ok_or_else(|| {
        (
            "CA certificate not found in settings".to_string(),
            Vec::new(),
        )
    })?;

    let cert_serial = proxy.certificate_serial.as_deref().ok_or_else(|| {
        (
            "Edge certificate serial not provisioned".to_string(),
            Vec::new(),
        )
    })?;

    // Seed a one-shot serial map so the rustls verifier validates the server cert serial.
    let (_, certs_rx) = watch::channel(Arc::new(HashMap::from([(
        proxy.id,
        cert_serial.to_string(),
    )])));

    let channel = proxy_mtls_channel(proxy, &ca_cert_der, certs_rx)
        .map_err(|e| (format!("Failed to build mTLS channel: {e}"), Vec::new()))?;

    let version = Version::parse(VERSION)
        .map_err(|e| (format!("Failed to parse core version: {e}"), Vec::new()))?;
    let version_interceptor = ClientVersionInterceptor::new(version);

    let mut client = ProxyClient::with_interceptor(channel, move |req: Request<()>| {
        version_interceptor.clone().call(req)
    });

    let mut stream = client
        .trigger_acme(AcmeChallenge {
            domain: domain.clone(),
            account_credentials_json,
        })
        .await
        .map_err(|e| (format!("TriggerAcme RPC failed: {e}"), Vec::new()))?
        .into_inner();

    let mut collected_logs: Vec<String> = Vec::new();

    loop {
        match stream.message().await {
            Ok(Some(event)) => match event.payload {
                Some(acme_issue_event::Payload::Progress(p)) => {
                    if let Ok(step) = AcmeStep::try_from(p.step) {
                        let _ = progress_tx.send(step);
                    }
                }
                Some(acme_issue_event::Payload::Certificate(cert)) => {
                    return Ok((cert.cert_pem, cert.key_pem, cert.account_credentials_json));
                }
                Some(acme_issue_event::Payload::Logs(AcmeLogs { lines })) => {
                    collected_logs = lines;
                }
                None => {
                    return Err((
                        "TriggerAcme stream sent an event with no payload".to_string(),
                        collected_logs,
                    ));
                }
            },
            Ok(None) => {
                return Err((
                    "TriggerAcme stream ended without delivering a certificate".to_string(),
                    collected_logs,
                ));
            }
            Err(e) => {
                return Err((
                    format!("Failed to read TriggerAcme response: {e}"),
                    collected_logs,
                ));
            }
        }
    }
}

/// Streams Let's Encrypt certificate issuance progress as Server-Sent Events.
///
/// Delegates the ACME HTTP-01 process to the proxy component via the `TriggerAcme`
/// RPC on the permanent `Proxy` gRPC service.  Reads proxy address and ACME
/// domain/credentials from the database - no query parameters needed.
///
/// On success, saves the certificate to the database and (when called post initial wizard)
/// broadcasts `HttpsCerts` to the proxy via `proxy_control_tx`.
// GET: EventSource only supports GET
pub async fn stream_proxy_acme(
    _admin: AdminOrSetupRole,
    Extension(pool): Extension<PgPool>,
    proxy_control_tx: Option<Extension<Sender<ProxyControlMessage>>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let certs = match Certificates::get_or_default(&pool).await {
            Ok(c) => c,
            Err(e) => {
                yield Ok(acme_error_event("Connecting", format!("Failed to load certificates: {e}"),
                    None));
                return;
            }
        };

        let domain = match public_proxy_hostname() {
            Ok(domain) => domain,
            Err(message) => {
                yield Ok(acme_error_event("Connecting", message, None));
                return;
            }
        };

        let account_credentials_json = certs.acme_account_credentials.clone().unwrap_or_default();

        let proxies = match Proxy::all_enabled(&pool).await {
            Ok(list) => list,
            Err(e) => {
                yield Ok(acme_error_event(
                    "Connecting",
                    format!("Failed to load Edge list from DB: {e}"),
                    None,
                ));
                return;
            }
        };

        let Some(proxy) = proxies.into_iter().next() else {
            yield Ok(acme_error_event(
                "Connecting",
                "No Edge found in database. Please complete the edge adoption step \
                 first."
                    .to_string(),
                None,
            ));
            return;
        };

        let proxy_host = &proxy.address;
        let proxy_port = proxy.port;
        info!(
            "Triggering ACME HTTP-01 via Edge gRPC TriggerAcme for domain: {domain} \
             Edge={proxy_host}:{proxy_port}"
        );

        let (progress_tx, mut progress_rx) =
            unbounded_channel::<AcmeStep>();
        let (result_tx, result_rx) =
            oneshot::channel::<Result<(String, String, String), (String, Vec<String>)>>();

        let pool_clone = pool.clone();
        let domain_clone = domain.clone();
        let acct_creds_clone = account_credentials_json.clone();
        tokio::spawn(async move {
            let result = call_proxy_trigger_acme(
                &pool_clone,
                &proxy,
                domain_clone,
                acct_creds_clone,
                progress_tx,
            )
            .await;
            let _ = result_tx.send(result);
        });

        let mut current_step: &'static str = "Connecting";
        let deadline = Instant::now()
            + Duration::from_secs(ACME_TIMEOUT_SECS);

        // Drain progress steps until the ACME task finishes (channel closed) or times out.
        loop {
            tokio::select! {
                maybe_step = progress_rx.recv() => {
                    match maybe_step {
                        Some(step) => {
                            current_step = acme_step_name(step);
                            yield Ok(acme_event(current_step));
                        }
                        None => {
                            // progress_tx dropped - ACME task finished; stop polling progress.
                            break;
                        }
                    }
                }

                () = sleep_until(deadline) => {
                    yield Ok(acme_error_event(
                        current_step,
                        format!(
                            "ACME certificate issuance timed out after \
                             {ACME_TIMEOUT_SECS} seconds."
                        ),
                        None,
                    ));
                    return;
                }
            }
        }

        // Progress channel closed - collect the final result.
        match result_rx.await {
            Ok(Ok((cert_pem, key_pem, new_account_credentials_json))) => {
                let acme_cert_expiry = parse_cert_expiry(&cert_pem);
                match Certificates::get_or_default(&pool).await {
                    Ok(mut updated_certs) => {
                        updated_certs.acme_domain = Some(domain.clone());
                        updated_certs.proxy_http_cert_pem = Some(cert_pem.clone());
                        updated_certs.proxy_http_cert_key_pem = Some(key_pem.clone());
                        updated_certs.proxy_http_cert_expiry = acme_cert_expiry;
                        updated_certs.acme_account_credentials =
                            Some(new_account_credentials_json);
                        updated_certs.proxy_http_cert_source =
                            ProxyCertSource::LetsEncrypt;
                        if let Err(e) = updated_certs.save(&pool).await {
                            yield Ok(acme_error_event(
                                "Installing",
                                format!("Failed to save certificate: {e}"),
                                None,
                            ));
                            return;
                        }
                    }
                    Err(e) => {
                        yield Ok(acme_error_event(
                            "Installing",
                            format!("Failed to reload certificates for saving: {e}"),
                            None,
                        ));
                        return;
                    }
                }

                // Post-wizard: broadcast certs to the proxy via bidi channel.
                if let Some(ref tx) = proxy_control_tx {
                    let msg = ProxyControlMessage::BroadcastHttpsCerts {
                        cert_pem,
                        key_pem,
                    };
                    if let Err(e) = tx.send(msg).await {
                        error!("Failed to broadcast HttpsCerts to Edge: {e}");
                    }
                }

                info!("ACME certificate issued and saved for domain: {domain}");
                yield Ok(acme_event("Done"));
            }
            Ok(Err((acme_err, logs))) => {
                let msg = format!("ACME issuance failed: {acme_err}");
                error!("{msg}");
                yield Ok(acme_error_event(current_step, msg, Some(logs)));
            }
            Err(_) => {
                yield Ok(acme_error_event(
                    current_step,
                    "ACME task terminated unexpectedly.".to_string(),
                    None,
                ));
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
