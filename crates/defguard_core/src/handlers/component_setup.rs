use std::{
    collections::VecDeque,
    convert::Infallible,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Extension,
    extract::{Path, Query},
    response::sse::{Event, KeepAlive, Sse},
};
use defguard_certs::der_to_pem;
use defguard_common::{
    VERSION,
    auth::claims::Claims,
    db::{
        Id,
        models::{
            Settings,
            gateway::Gateway,
            initial_setup_wizard::{InitialSetupState, InitialSetupStep},
            proxy::Proxy,
            wizard::Wizard,
        },
    },
    types::proxy::ProxyControlMessage,
};
use defguard_proto::{
    gateway::gateway_setup_client::GatewaySetupClient,
    proxy::{CertificateInfo, DerPayload, proxy_setup_client::ProxySetupClient},
};
use defguard_version::{Version, client::ClientVersionInterceptor};
use futures::Stream;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::mpsc::Sender;
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

#[derive(Debug, Serialize)]
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

    match serde_json::to_string(&response) {
        Ok(body) => Event::default().data(body),
        Err(e) => Event::default().data(fallback_message(&e.to_string(), last_step)),
    }
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
    log_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
}

impl SetupFlow {
    fn new(
        log_rx: tokio::sync::mpsc::UnboundedReceiver<String>,
        log_buffer: Arc<Mutex<VecDeque<String>>>,
    ) -> Self {
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
                .unwrap_or_else(|poisoned| poisoned.into_inner());
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
    let (log_tx, log_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
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

        // Step 1: Check configuration
        yield Ok(flow.step(SetupStep::CheckingConfiguration));
        match Proxy::find_by_address_port(&pool, &request.ip_or_domain, i32::from(request.grpc_port)).await {
            Ok(Some(proxy)) => {
                yield Ok(flow.error(&format!(
                    "An edge Proxy with address {}:{} is already registered with name \"{}\".",
                    request.ip_or_domain, request.grpc_port, proxy.name
                )));
                return;
            }
            Ok(None) => {
                debug!(
                    "Verified no existing proxy registration for {}:{}",
                    request.ip_or_domain, request.grpc_port
                );
            }
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to query existing proxy: {e}")));
                return;
            }
        }

        debug!("Configuration check passed");

        let url_str = format!("http://{}:{}", request.ip_or_domain, request.grpc_port);
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

        let settings = Settings::get_current_settings();
        let Some(ca_cert_der) = settings.ca_cert_der else {
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
            "Prepared secure connection endpoint for Edge at {}:{}",
            request.ip_or_domain, request.grpc_port
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
            "Initiating connection to Edge at {}:{}",
            request.ip_or_domain, request.grpc_port
        );

        let response_with_metadata = match tokio::time::timeout(CONNECTION_TIMEOUT, client.start(())).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                match e.code() {
                    tonic::Code::Unavailable => {
                        let error_msg = e.to_string();
                        if error_msg.contains("h2 protocol error") || error_msg.contains("http2 error") {
                            yield Ok(flow.error(&format!(
                                "Failed to connect to Edge at {}:{}: {}. This may indicate that the Edge is already configured with TLS. Please check if the Edge has already been set up.",
                                request.ip_or_domain, request.grpc_port, e
                            )));
                        } else {
                            yield Ok(flow.error(&format!(
                                "Failed to connect to Edge at {}:{}. Please ensure the address and port are correct and that the Edge component is running.",
                                request.ip_or_domain, request.grpc_port
                            )));
                        }
                    }
                    _ => {
                        yield Ok(flow.error(&format!("Failed to connect to Edge: {e}")));
                    }
                }
                return;
            }
            Err(_) => {
                yield Ok(flow.error(&format!(
                    "Connection to Edge at {}:{} timed out after 10 seconds.",
                    request.ip_or_domain, request.grpc_port
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
                    "Edge version {proxy_version} is older than Core version {version_clone}. Please update the Edge component.",
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

        let settings = Settings::get_current_settings();
        let Some(ca_cert_der) = settings.ca_cert_der else {
            yield Ok(flow.error("CA certificate not found in settings"));
            return;
        };
        let Some(ca_key_pair) = settings.ca_key_der else {
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

        let cert = match ca.sign_csr(&csr) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to sign CSR: {e}")));
                return;
            }
        };

        debug!("Successfully signed certificate for Edge");

        // Step 6: Configure TLS
        yield Ok(flow.step(SetupStep::ConfiguringTls));

        if let Err(e) = client.send_cert(DerPayload { der_data: cert.der().to_vec() }).await {
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
            &request.common_name,
            &request.ip_or_domain,
            i32::from(request.grpc_port),
            &session.user.fullname(),
        );
        proxy.certificate = Some(serial);
        proxy.certificate_expiry = Some(expiry);

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
                        step: InitialSetupStep::Confirmation,
                    };
                    if let Err(err) = state.save(&pool).await {
                        yield Ok(flow.error(&format!("Failed to update setup step in wizard: {err}")));
                        return;
                    }
                    debug!("Initial setup step advanced to 'Confirmation'");
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
    let (log_tx, log_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
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



        match Gateway::find_by_url(&pool, &request.ip_or_domain, request.grpc_port).await {
            Ok(Some(gateway)) => {
               yield Ok(flow.error(&format!("A Gateway with URL {}:{} is already registered with \
                   name \"{}\".", request.ip_or_domain, request.grpc_port, gateway.name)));
               return;
            }
            Ok(None) => {
                debug!("Verified no existing Gateway registration for {}:{}", request.ip_or_domain,
                    request.grpc_port);
            },
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to query existing Gateway: {e}")));
                return;
            }
        }

        let url_str = format!("http://{}:{}", request.ip_or_domain, request.grpc_port);
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

        let settings = Settings::get_current_settings();
        let Some(ca_cert_der) = settings.ca_cert_der else {
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

        debug!("Prepared secure connection endpoint for Gateway at {}:{}", request.ip_or_domain,
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

        debug!("Initiating connection to Gateway at {}:{}", request.ip_or_domain,
            request.grpc_port);

        let response_with_metadata = match tokio::time::timeout(
            CONNECTION_TIMEOUT,
            client.start(())
        ).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                match e.code() {
                    tonic::Code::Unavailable => {
                        let error_msg = e.to_string();
                        if error_msg.contains("h2 protocol error") || error_msg.contains("http2 error") {
                            yield Ok(flow.error(&format!(
                                "Failed to connect to Gateway at {}:{}: {e}. This may indicate \
                                that the Gateway is already configured with TLS. Please check if \
                                the Gateway has already been set up.",
                                request.ip_or_domain, request.grpc_port,
                            )));
                        } else {
                        yield Ok(flow.error(&format!(
                            "Failed to connect to Gateway at {}:{}. Please ensure the address and \
                            port are correct and that the Gateway is running.",
                            request.ip_or_domain, request.grpc_port
                        )));
                        }
                    }
                    _ => {
                        yield Ok(flow.error(&format!("Failed to connect to Gateway: {e}")));
                    }
                }
                return;
            }
            Err(_) => {
                yield Ok(flow.error(&format!(
                    "Connection to Gateway at {}:{} timed out after 10 seconds.",
                    request.ip_or_domain, request.grpc_port
                )));
                return;
            }
        };

        debug!("Successfully connected to Gateway");

        // Step 3: Check version
        yield Ok(
            flow.step(SetupStep::CheckingVersion)
        );

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
            .get_csr(defguard_proto::gateway::CertificateInfo {
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

        let settings = Settings::get_current_settings();

        let Some(ca_cert_der) = settings.ca_cert_der else {
            yield Ok(flow.error("CA certificate not found in settings"));
            return;
        };

        let Some(ca_key_pair) = settings.ca_key_der else {
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

        let cert = match ca.sign_csr(&csr) {
            Ok(c) => c,
            Err(e) => {
                yield Ok(flow.error(&format!("Failed to sign CSR: {e}")));
                return;
            }
        };

        debug!("Successfully signed certificate for Gateway");

        // Step 6: Configure TLS
        yield Ok(flow.step(SetupStep::ConfiguringTls));

        let response = defguard_proto::gateway::DerPayload {
            der_data: cert.der().to_vec(),
        };

        if let Err(e) = client.send_cert(response).await {
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
            request.ip_or_domain,
            request.grpc_port.into(),
            session.user.fullname(),
        );

        gateway.certificate = Some(serial);
        gateway.certificate_expiry = Some(expiry);

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
