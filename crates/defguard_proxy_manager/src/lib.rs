use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use defguard_certs::der_to_pem;
use defguard_common::{VERSION, config::server_config, db::models::Settings};
use defguard_core::{
    db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token, TokenError},
    enrollment_management::clear_unused_enrollment_tokens,
    enterprise::{
        db::models::openid_provider::OpenIdProvider,
        directory_sync::sync_user_groups_if_configured,
        grpc::polling::PollingServer,
        handlers::openid_login::{
            SELECT_ACCOUNT_SUPPORTED_PROVIDERS, build_state, make_oidc_client, user_from_claims,
        },
        is_business_license_active,
        ldap::utils::ldap_update_user_state,
    },
    events::BidiStreamEvent,
    grpc::{gateway::events::GatewayEvent, proxy::client_mfa::ClientMfaServer},
    version::{IncompatibleComponents, IncompatibleProxyData, is_proxy_version_supported},
};
use defguard_mail::Mail;
use defguard_proto::proxy::{
    AuthCallbackResponse, AuthInfoResponse, CertResponse, CoreError, CoreRequest, CoreResponse,
    CsrRequest, Done, ProxySetupResponse, core_request, core_response, proxy_client::ProxyClient,
    proxy_setup_request,
};
use defguard_version::{
    ComponentInfo, DefguardComponent, client::ClientVersionInterceptor, get_tracing_variables,
};
use openidconnect::{AuthorizationCode, Nonce, Scope, core::CoreAuthenticationFlow, url};
use reqwest::Url;
use semver::Version;
use sqlx::PgPool;
use thiserror::Error;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{self, UnboundedSender},
    },
    task::JoinSet,
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    Code, Streaming,
    transport::{Certificate, ClientTlsConfig, Endpoint},
};

use crate::{enrollment::EnrollmentServer, password_reset::PasswordResetServer};

mod enrollment;
pub(crate) mod password_reset;

#[macro_use]
extern crate tracing;

const TEN_SECS: Duration = Duration::from_secs(10);
const PROXY_AFTER_SETUP_CONNECT_DELAY: Duration = Duration::from_secs(1);
const PROXY_SETUP_RESTART_DELAY: Duration = Duration::from_secs(5);
static VERSION_ZERO: Version = Version::new(0, 0, 0);

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error(transparent)]
    InvalidUriError(#[from] axum::http::uri::InvalidUri),
    #[error("Failed to read CA certificate: {0}")]
    CaCertReadError(std::io::Error),
    #[error(transparent)]
    TonicError(#[from] tonic::transport::Error),
    #[error(transparent)]
    SemverError(#[from] semver::Error),
    #[error(transparent)]
    SqlxError(#[from] sqlx::Error),
    #[error(transparent)]
    TokenError(#[from] TokenError),
    #[error(transparent)]
    CertificateError(#[from] defguard_certs::CertificateError),
    #[error(transparent)]
    UrlParseError(#[from] url::ParseError),
    #[error("Missing proxy configuration: {0}")]
    MissingConfiguration(String),
    #[error("URL error: {0}")]
    UrlError(String),
    #[error("Proxy setup error: {0}")]
    SetupError(#[from] tokio::sync::mpsc::error::SendError<ProxySetupResponse>),
    #[error(transparent)]
    Transport(#[from] tonic::Status),
    #[error("Connection timeout: {0}")]
    ConnectionTimeout(String),
}

/// Maintains routing state for proxy-specific responses by associating
/// correlation tokens with the proxy senders that should receive them.
#[derive(Default)]
struct ProxyRouter {
    response_map: HashMap<String, Vec<UnboundedSender<CoreResponse>>>,
}

impl ProxyRouter {
    /// Records the proxy sender associated with a request that expects a routed response.
    pub(crate) fn register_request(
        &mut self,
        request: &CoreRequest,
        sender: &UnboundedSender<CoreResponse>,
    ) {
        match &request.payload {
            // Mobile-assisted MFA completion responses must go to the proxy that owns the WebSocket
            // so it can send the preshared key.
            // Corresponds to the `core_response::Payload::ClientMfaFinish(response)` response.
            // https://github.com/DefGuard/defguard/issues/1700
            Some(core_request::Payload::ClientMfaTokenValidation(request)) => {
                self.response_map
                    .insert(request.token.clone(), vec![sender.clone()]);
            }
            Some(core_request::Payload::ClientMfaFinish(request)) => {
                if let Some(senders) = self.response_map.get_mut(&request.token) {
                    senders.push(sender.clone());
                }
            }
            _ => {}
        }
    }

    /// Determines whether the given `CoreResponse` must be routed to a specific proxy instance.
    pub(crate) fn route_response(
        &mut self,
        response: &CoreResponse,
    ) -> Option<Vec<UnboundedSender<CoreResponse>>> {
        #[allow(clippy::single_match)]
        match &response.payload {
            // Mobile-assisted MFA completion responses must go to the proxy that owns the WebSocket
            // so it can send the preshared key.
            // Corresponds to the `core_request::Payload::ClientMfaTokenValidation(request)` request.
            // https://github.com/DefGuard/defguard/issues/1700
            Some(core_response::Payload::ClientMfaFinish(response)) => {
                if let Some(ref token) = response.token {
                    return self.response_map.remove(token);
                }
            }
            _ => {}
        }
        None
    }
}

/// Coordinates communication between the Core and multiple proxy instances.
///
/// Responsibilities include:
/// - instantiating and supervising proxy connections,
/// - routing responses to the appropriate proxy based on correlation state,
/// - providing shared infrastructure (database access, outbound channels),
pub struct ProxyOrchestrator {
    pool: PgPool,
    tx: ProxyTxSet,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    router: Arc<RwLock<ProxyRouter>>,
}

impl ProxyOrchestrator {
    pub fn new(
        pool: PgPool,
        tx: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    ) -> Self {
        Self {
            pool,
            tx,
            incompatible_components,
            router: Arc::default(),
        }
    }

    /// Spawns and supervises asynchronous tasks for all configured proxies.
    ///
    /// Each proxy runs in its own task and shares Core-side infrastructure
    /// such as routing state and compatibility tracking.
    pub async fn run(self, url: &Option<String>) -> Result<(), ProxyError> {
        // TODO retrieve proxies from db
        let Some(url) = url else {
            tokio::time::sleep(Duration::MAX).await;
            return Ok(());
        };
        let proxies = vec![Proxy::new(
            self.pool.clone(),
            Url::from_str(url)?,
            self.tx.clone(),
            Arc::clone(&self.router),
        )];
        let mut tasks = JoinSet::<Result<(), ProxyError>>::new();
        for proxy in proxies {
            tasks.spawn(proxy.run(self.tx.clone(), self.incompatible_components.clone()));
        }
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok(Ok(())) => error!("Proxy task returned prematurely"),
                Ok(Err(err)) => error!("Proxy task returned with error: {err}"),
                Err(err) => error!("Proxy task execution failed: {err}"),
            }
        }
        Ok(())
    }
}

/// Shared set of outbound channels that proxy instances use to forward
/// events, notifications, and side effects to Core components.
#[derive(Clone)]
pub struct ProxyTxSet {
    wireguard: Sender<GatewayEvent>,
    mail: UnboundedSender<Mail>,
    bidi_events: UnboundedSender<BidiStreamEvent>,
}

impl ProxyTxSet {
    #[must_use]
    pub const fn new(
        wireguard: Sender<GatewayEvent>,
        mail: UnboundedSender<Mail>,
        bidi_events: UnboundedSender<BidiStreamEvent>,
    ) -> Self {
        Self {
            wireguard,
            mail,
            bidi_events,
        }
    }
}

/// Represents a single Core - Proxy connection.
///
/// A `Proxy` is responsible for establishing and maintaining a gRPC
/// bidirectional stream to one proxy instance, handling incoming requests
/// from that proxy, and forwarding responses back through the same stream.
/// Each `Proxy` runs independently and is supervised by the
/// `ProxyOrchestrator`.
struct Proxy {
    pool: PgPool,
    /// gRPC servers
    services: ProxyServices,
    /// Router shared between proxies and the orchestrator
    router: Arc<RwLock<ProxyRouter>>,
    /// Proxy server gRPC URL
    url: Url,
}

impl Proxy {
    pub fn new(pool: PgPool, url: Url, tx: ProxyTxSet, router: Arc<RwLock<ProxyRouter>>) -> Self {
        // Instantiate gRPC servers.
        let services = ProxyServices::new(pool.clone(), tx);

        Self {
            pool,
            services,
            router,
            url,
        }
    }

    fn endpoint(&self, with_tls: bool) -> Result<Endpoint, ProxyError> {
        let mut url = self.url.clone();

        let scheme = if with_tls { "https" } else { "http" };
        url.set_scheme(scheme).map_err(|()| {
            ProxyError::UrlError(format!("Failed to set {scheme} scheme on URL {url}"))
        })?;
        let endpoint = Endpoint::from_shared(url.to_string())?;
        let endpoint = endpoint
            .http2_keep_alive_interval(TEN_SECS)
            .tcp_keepalive(Some(TEN_SECS))
            .keep_alive_while_idle(true);

        let endpoint = if with_tls {
            let settings = Settings::get_current_settings();
            let Some(ca_cert_der) = settings.ca_cert_der else {
                return Err(ProxyError::MissingConfiguration(
                    "Core CA is not setup, can't create a Proxy endpoint.".to_string(),
                ));
            };

            let cert_pem = der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate)?;
            let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(&cert_pem));

            endpoint.tls_config(tls)?
        } else {
            endpoint
        };

        Ok(endpoint)
    }

    /// Establishes and maintains a gRPC bidirectional stream to the proxy.
    ///
    /// The proxy connection is retried on failure, compatibility is checked
    /// on each successful connection, and incoming messages are handled
    /// until the stream is closed.
    pub(crate) async fn run(
        mut self,
        tx_set: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
    ) -> Result<(), ProxyError> {
        loop {
            // Probe endpoint for HTTPS availability before performing initial setup
            match self.is_https_configured().await {
                Ok(true) => {
                    // HTTPS already present - skip initial setup
                }
                Ok(false) => {
                    // HTTPS not configured - try to perform initial setup
                    if let Err(err) = self.perform_initial_setup().await {
                        error!("Failed to perform initial Proxy setup: {err}");
                    }
                    // Wait a bit before establishing proper connection, reconnecting too fast will often result in an
                    // error since proxy may have not restarted the server yet.
                    tokio::time::sleep(PROXY_AFTER_SETUP_CONNECT_DELAY).await;
                }
                Err(err) => {
                    error!(
                        "Failed to check if Proxy gRPC server is running on {} or {}, retrying in 10s: {err}.",
                        self.endpoint(false)?.uri(),
                        self.endpoint(true)?.uri()
                    );
                    sleep(TEN_SECS).await;
                    continue;
                }
            }

            let endpoint = self.endpoint(true)?;

            debug!("Connecting to proxy at {}", endpoint.uri());
            let interceptor = ClientVersionInterceptor::new(Version::parse(VERSION)?);
            let mut client = ProxyClient::with_interceptor(endpoint.connect_lazy(), interceptor);
            let (tx, rx) = mpsc::unbounded_channel();
            let response = match client.bidi(UnboundedReceiverStream::new(rx)).await {
                Ok(response) => response,
                Err(err) => {
                    match err.code() {
                        Code::FailedPrecondition => {
                            error!(
                                "Failed to connect to proxy @ {}, version check failed, retrying in \
                            10s: {err}",
                                endpoint.uri()
                            );
                            // TODO push event
                        }
                        err => {
                            error!(
                                "Failed to connect to proxy @ {}, retrying in 10s: {err}",
                                endpoint.uri()
                            );
                        }
                    }
                    sleep(TEN_SECS).await;
                    continue;
                }
            };
            let maybe_info = ComponentInfo::from_metadata(response.metadata());

            // Check proxy version and continue if it's not supported.
            let (version, info) = get_tracing_variables(&maybe_info);
            let proxy_is_supported = is_proxy_version_supported(Some(&version));

            let span = tracing::info_span!("proxy_bidi", component = %DefguardComponent::Proxy,
            version = version.to_string(), info);
            let _guard = span.enter();
            if !proxy_is_supported {
                // Store incompatible proxy
                let maybe_version = if version == VERSION_ZERO {
                    None
                } else {
                    Some(version)
                };
                let data = IncompatibleProxyData::new(maybe_version);
                data.insert(&incompatible_components);

                // Sleep before trying to reconnect
                sleep(TEN_SECS).await;
                continue;
            }
            IncompatibleComponents::remove_proxy(&incompatible_components);

            info!("Connected to proxy at {}", endpoint.uri());
            let mut resp_stream = response.into_inner();
            self.message_loop(tx, tx_set.wireguard.clone(), &mut resp_stream)
                .await?;
        }
    }

    /// Probe the endpoint to check if HTTPS is available.
    /// Returns Ok(true) if HTTPS is configured and working.
    /// Returns Ok(false) if there's a protocol/TLS error (HTTPS not configured).
    /// Returns Err for other errors like timeouts or network issues.
    async fn is_https_configured(&self) -> Result<bool, ProxyError> {
        let endpoint = self.endpoint(true)?;
        let interceptor = ClientVersionInterceptor::new(Version::parse(VERSION)?);
        let (tx, rx) = mpsc::unbounded_channel();
        let mut client = ProxyClient::with_interceptor(endpoint.connect_lazy(), interceptor);

        match tokio::time::timeout(
            Duration::from_secs(10),
            client.setup(UnboundedReceiverStream::new(rx)),
        )
        .await
        {
            Ok(Ok(_)) => {
                info!("Proxy endpoint is already using HTTPS, skipping initial setup");
                let _ = tx.send(ProxySetupResponse {
                    payload: Some(defguard_proto::proxy::proxy_setup_response::Payload::Done(
                        Done {},
                    )),
                });
                Ok(true)
            }
            Ok(Err(err)) => {
                let http_endpoint = self.endpoint(false)?;
                let interceptor = ClientVersionInterceptor::new(Version::parse(VERSION)?);
                let (tx, rx) = mpsc::unbounded_channel();
                let mut client =
                    ProxyClient::with_interceptor(http_endpoint.connect_lazy(), interceptor);

                match tokio::time::timeout(
                    Duration::from_secs(10),
                    client.setup(UnboundedReceiverStream::new(rx)),
                )
                .await
                {
                    // HTTP works = HTTPS not configured
                    Ok(Ok(_)) => {
                        info!("Proxy endpoint is available via HTTP, HTTPS not configured");
                        let _ = tx.send(ProxySetupResponse {
                            payload: Some(
                                defguard_proto::proxy::proxy_setup_response::Payload::Done(Done {}),
                            ),
                        });
                        Ok(false)
                    }
                    Ok(Err(_)) => {
                        // gRPC errors should be propagated
                        Err(ProxyError::Transport(err))
                    }
                    Err(_) => Err(ProxyError::ConnectionTimeout(
                        "Timeout while probing HTTP endpoint".to_string(),
                    )),
                }
            }
            Err(_) => Err(ProxyError::ConnectionTimeout(
                "Timeout while probing HTTPS endpoint".to_string(),
            )),
        }
    }

    /// Attempt to perform an initial setup of the target proxy.
    /// If the proxy doesn't have signed gRPC certificates by Core yet,
    /// this step will perform the signing. Otherwise, the step will be skipped
    /// by instantly sending the "Done" message by both parties.
    pub async fn perform_initial_setup(&self) -> Result<(), ProxyError> {
        let endpoint = self.endpoint(false)?;

        let interceptor = ClientVersionInterceptor::new(Version::parse(VERSION)?);
        let mut client = ProxyClient::with_interceptor(endpoint.connect_lazy(), interceptor);
        let Some(hostname) = self.url.host_str() else {
            return Err(ProxyError::UrlError(
                "Proxy URL missing hostname".to_string(),
            ));
        };

        'connection: loop {
            let (tx, rx) = mpsc::unbounded_channel();
            let mut stream = match client.setup(UnboundedReceiverStream::new(rx)).await {
                Ok(response) => response.into_inner(),
                Err(err) => {
                    error!(
                        "Failed to connect to proxy @ {}, retrying in 10s: {}",
                        endpoint.uri(),
                        err
                    );
                    sleep(TEN_SECS).await;
                    continue 'connection;
                }
            };

            tx.send(ProxySetupResponse {
                payload: Some(
                    defguard_proto::proxy::proxy_setup_response::Payload::InitialSetupInfo(
                        defguard_proto::proxy::InitialSetupInfo {
                            cert_hostname: hostname.to_string(),
                        },
                    ),
                ),
            })?;

            loop {
                match stream.message().await {
                    Ok(Some(req)) => match req.payload {
                        Some(proxy_setup_request::Payload::CsrRequest(CsrRequest { csr_der })) => {
                            debug!("Received CSR from proxy during initial setup");
                            match defguard_certs::Csr::from_der(&csr_der) {
                                Ok(csr) => {
                                    let settings = Settings::get_current_settings();

                                    let ca_cert_der = settings.ca_cert_der.ok_or_else(|| {
                                        ProxyError::MissingConfiguration(
                                            "CA certificate DER not found in settings for proxy gRPC bidi stream".to_string(),
                                        )
                                    })?;
                                    let ca_key_pair = settings.ca_key_der.ok_or_else(|| {
                                        ProxyError::MissingConfiguration(
                                            "CA key pairs DER not found in settings for proxy gRPC bidi stream".to_string(),
                                        )
                                    })?;

                                    let ca = defguard_certs::CertificateAuthority::from_cert_der_key_pair(
                                        &ca_cert_der,
                                        &ca_key_pair,
                                    )?;

                                    match ca.sign_csr(&csr) {
                                        Ok(cert) => {
                                            let response = CertResponse {
                                                cert_der: cert.der().to_vec(),
                                            };
                                            tx.send(ProxySetupResponse { payload: Some(
                                                defguard_proto::proxy::proxy_setup_response::Payload::CertResponse(response)
                                            ) })?;
                                            info!(
                                                "Signed CSR received from proxy during initial setup and sent back the certificate"
                                            );
                                        }
                                        Err(err) => {
                                            error!("Failed to sign CSR: {err}");
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!("Failed to parse CSR: {err}");
                                }
                            }
                        }
                        Some(proxy_setup_request::Payload::Done(Done {})) => {
                            info!("Proxy setup completed");
                            tx.send(ProxySetupResponse {
                                payload: Some(
                                    defguard_proto::proxy::proxy_setup_response::Payload::Done(
                                        Done {},
                                    ),
                                ),
                            })?;
                            break 'connection;
                        }
                        _ => {
                            error!("Expected CertRequest from proxy during setup");
                        }
                    },
                    Ok(None) => {
                        error!("Proxy setup stream closed unexpectedly");
                        break;
                    }
                    Err(err) => {
                        error!("Failed to receive CSR request from proxy: {err}");
                        break;
                    }
                }
            }

            sleep(PROXY_SETUP_RESTART_DELAY).await;
        }

        Ok(())
    }

    /// Processes incoming requests from the proxy over an active gRPC stream.
    ///
    /// This loop receives `CoreRequest` messages from the proxy, dispatches
    /// them to the appropriate Core-side handlers, and sends corresponding
    /// `CoreResponse` messages back through the stream. Certain requests may
    /// also register routing state for future responses.
    async fn message_loop(
        &mut self,
        tx: UnboundedSender<CoreResponse>,
        wireguard_tx: Sender<GatewayEvent>,
        resp_stream: &mut Streaming<CoreRequest>,
    ) -> Result<(), ProxyError> {
        let pool = self.pool.clone();
        'message: loop {
            match resp_stream.message().await {
                Ok(None) => {
                    info!("stream was closed by the sender");
                    break 'message;
                }
                Ok(Some(received)) => {
                    debug!("Received message from proxy; ID={}", received.id);
                    self.router
                        .write()
                        .unwrap()
                        .register_request(&received, &tx);
                    let payload = match received.payload {
                        // rpc CodeMfaSetupStart return (CodeMfaSetupStartResponse)
                        Some(core_request::Payload::CodeMfaSetupStart(request)) => {
                            match self
                                .services
                                .enrollment
                                .register_code_mfa_start(request)
                                .await
                            {
                                Ok(response) => Some(
                                    core_response::Payload::CodeMfaSetupStartResponse(response),
                                ),
                                Err(err) => {
                                    error!("Register mfa start error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc CodeMfaSetupFinish return (CodeMfaSetupFinishResponse)
                        Some(core_request::Payload::CodeMfaSetupFinish(request)) => {
                            match self
                                .services
                                .enrollment
                                .register_code_mfa_finish(request)
                                .await
                            {
                                Ok(response) => Some(
                                    core_response::Payload::CodeMfaSetupFinishResponse(response),
                                ),
                                Err(err) => {
                                    error!("Register MFA finish error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ClientMfaTokenValidation return (ClientMfaTokenValidationResponse)
                        Some(core_request::Payload::ClientMfaTokenValidation(request)) => {
                            match self.services.client_mfa.validate_mfa_token(request).await {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::ClientMfaTokenValidation(
                                        response_payload,
                                    ))
                                }
                                Err(err) => {
                                    error!("Client MFA validate token error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc RegisterMobileAuth (RegisterMobileAuthRequest) return (google.protobuf.Empty)
                        Some(core_request::Payload::RegisterMobileAuth(request)) => {
                            match self.services.enrollment.register_mobile_auth(request).await {
                                Ok(()) => Some(core_response::Payload::Empty(())),
                                Err(err) => {
                                    error!("Register mobile auth error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc StartEnrollment (EnrollmentStartRequest) returns (EnrollmentStartResponse)
                        Some(core_request::Payload::EnrollmentStart(request)) => {
                            match self
                                .services
                                .enrollment
                                .start_enrollment(request, received.device_info)
                                .await
                            {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::EnrollmentStart(response_payload))
                                }
                                Err(err) => {
                                    error!("start enrollment error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ActivateUser (ActivateUserRequest) returns (google.protobuf.Empty)
                        Some(core_request::Payload::ActivateUser(request)) => {
                            match self
                                .services
                                .enrollment
                                .activate_user(request, received.device_info)
                                .await
                            {
                                Ok(()) => Some(core_response::Payload::Empty(())),
                                Err(err) => {
                                    error!("activate user error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc CreateDevice (NewDevice) returns (DeviceConfigResponse)
                        Some(core_request::Payload::NewDevice(request)) => {
                            match self
                                .services
                                .enrollment
                                .create_device(request, received.device_info)
                                .await
                            {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::DeviceConfig(response_payload))
                                }
                                Err(err) => {
                                    error!("create device error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc GetNetworkInfo (ExistingDevice) returns (DeviceConfigResponse)
                        Some(core_request::Payload::ExistingDevice(request)) => {
                            match self
                                .services
                                .enrollment
                                .get_network_info(request, received.device_info)
                                .await
                            {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::DeviceConfig(response_payload))
                                }
                                Err(err) => {
                                    error!("get network info error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc RequestPasswordReset (PasswordResetInitializeRequest) returns (google.protobuf.Empty)
                        Some(core_request::Payload::PasswordResetInit(request)) => {
                            match self
                                .services
                                .password_reset
                                .request_password_reset(request, received.device_info)
                                .await
                            {
                                Ok(()) => Some(core_response::Payload::Empty(())),
                                Err(err) => {
                                    error!("password reset init error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc StartPasswordReset (PasswordResetStartRequest) returns (PasswordResetStartResponse)
                        Some(core_request::Payload::PasswordResetStart(request)) => {
                            match self
                                .services
                                .password_reset
                                .start_password_reset(request, received.device_info)
                                .await
                            {
                                Ok(response_payload) => Some(
                                    core_response::Payload::PasswordResetStart(response_payload),
                                ),
                                Err(err) => {
                                    error!("password reset start error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ResetPassword (PasswordResetRequest) returns (google.protobuf.Empty)
                        Some(core_request::Payload::PasswordReset(request)) => {
                            match self
                                .services
                                .password_reset
                                .reset_password(request, received.device_info)
                                .await
                            {
                                Ok(()) => Some(core_response::Payload::Empty(())),
                                Err(err) => {
                                    error!("password reset error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ClientMfaStart (ClientMfaStartRequest) returns (ClientMfaStartResponse)
                        Some(core_request::Payload::ClientMfaStart(request)) => {
                            match self
                                .services
                                .client_mfa
                                .start_client_mfa_login(request)
                                .await
                            {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::ClientMfaStart(response_payload))
                                }
                                Err(err) => {
                                    error!("client MFA start error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc ClientMfaFinish (ClientMfaFinishRequest) returns (ClientMfaFinishResponse)
                        Some(core_request::Payload::ClientMfaFinish(request)) => {
                            match self
                                .services
                                .client_mfa
                                .finish_client_mfa_login(request, received.device_info)
                                .await
                            {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::ClientMfaFinish(response_payload))
                                }
                                Err(err) => {
                                    match err.code() {
                                        Code::FailedPrecondition => {
                                            // User not yet done with OIDC authentication. Don't log it
                                            // as an error.
                                            debug!("Client MFA finish error: {err}");
                                        }
                                        _ => {
                                            // Log other errors as errors.
                                            error!("Client MFA finish error: {err}");
                                        }
                                    }
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        Some(core_request::Payload::ClientMfaOidcAuthenticate(request)) => {
                            match self
                                .services
                                .client_mfa
                                .auth_mfa_session_with_oidc(request, received.device_info)
                                .await
                            {
                                Ok(()) => Some(core_response::Payload::Empty(())),
                                Err(err) => {
                                    error!("client MFA OIDC authenticate error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc LocationInfo (LocationInfoRequest) returns (LocationInfoResponse)
                        Some(core_request::Payload::InstanceInfo(request)) => {
                            match self
                                .services
                                .polling
                                .info(request, received.device_info)
                                .await
                            {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::InstanceInfo(response_payload))
                                }
                                Err(err) => {
                                    if Code::FailedPrecondition == err.code() {
                                        // Ignore the case when we are not enterprise but the client is
                                        // trying to fetch the instance config,
                                        // to avoid spamming the logs with misleading errors.

                                        debug!(
                                            "A client tried to fetch the instance config, but we are \
                                        not enterprise."
                                        );
                                        Some(core_response::Payload::CoreError(err.into()))
                                    } else {
                                        error!("Instance info error {err}");
                                        Some(core_response::Payload::CoreError(err.into()))
                                    }
                                }
                            }
                        }
                        Some(core_request::Payload::AuthInfo(request)) => {
                            if !is_business_license_active() {
                                warn!("Enterprise license required");
                                Some(core_response::Payload::CoreError(CoreError {
                                    status_code: Code::FailedPrecondition as i32,
                                    message: "no valid license".into(),
                                }))
                            } else if let Ok(redirect_url) = Url::parse(&request.redirect_url) {
                                if let Some(provider) = OpenIdProvider::get_current(&pool).await? {
                                    match make_oidc_client(redirect_url, &provider).await {
                                        Ok((_client_id, client)) => {
                                            let mut authorize_url_builder = client
                                                .authorize_url(
                                                    CoreAuthenticationFlow::AuthorizationCode,
                                                    || build_state(request.state),
                                                    Nonce::new_random,
                                                )
                                                .add_scope(Scope::new("email".to_string()))
                                                .add_scope(Scope::new("profile".to_string()));

                                            if SELECT_ACCOUNT_SUPPORTED_PROVIDERS
                                                .iter()
                                                .all(|p| p.eq_ignore_ascii_case(&provider.name))
                                            {
                                                authorize_url_builder = authorize_url_builder
                                                .add_prompt(
                                                openidconnect::core::CoreAuthPrompt::SelectAccount,
                                            );
                                            }
                                            let (url, csrf_token, nonce) =
                                                authorize_url_builder.url();

                                            Some(core_response::Payload::AuthInfo(
                                                AuthInfoResponse {
                                                    url: url.into(),
                                                    csrf_token: csrf_token.secret().to_owned(),
                                                    nonce: nonce.secret().to_owned(),
                                                    button_display_name: provider.display_name,
                                                },
                                            ))
                                        }
                                        Err(err) => {
                                            error!(
                                                "Failed to setup external OIDC provider client: {err}"
                                            );
                                            Some(core_response::Payload::CoreError(CoreError {
                                                status_code: Code::Internal as i32,
                                                message: "failed to build OIDC client".into(),
                                            }))
                                        }
                                    }
                                } else {
                                    error!("Failed to get current OpenID provider");
                                    Some(core_response::Payload::CoreError(CoreError {
                                        status_code: Code::NotFound as i32,
                                        message: "failed to get current OpenID provider".into(),
                                    }))
                                }
                            } else {
                                error!(
                                    "Invalid redirect URL in authentication info request: {}",
                                    request.redirect_url
                                );
                                Some(core_response::Payload::CoreError(CoreError {
                                    status_code: Code::Internal as i32,
                                    message: "invalid redirect URL".into(),
                                }))
                            }
                        }
                        Some(core_request::Payload::AuthCallback(request)) => {
                            match Url::parse(&request.callback_url) {
                                Ok(callback_url) => {
                                    let code = AuthorizationCode::new(request.code);
                                    match user_from_claims(
                                        &pool,
                                        Nonce::new(request.nonce),
                                        code,
                                        callback_url,
                                    )
                                    .await
                                    {
                                        Ok(mut user) => {
                                            clear_unused_enrollment_tokens(&user, &pool).await?;
                                            if let Err(err) = sync_user_groups_if_configured(
                                                &user,
                                                &pool,
                                                &wireguard_tx,
                                            )
                                            .await
                                            {
                                                error!(
                                                    "Failed to sync user groups for user {} with the \
                                                directory while the user was logging in through an \
                                                external provider: {err}",
                                                    user.username,
                                                );
                                            } else {
                                                ldap_update_user_state(&mut user, &pool).await;
                                            }
                                            debug!("Cleared unused tokens for {}.", user.username);
                                            debug!(
                                                "Creating a new desktop activation token for user {} \
                                            as a result of proxy OpenID auth callback.",
                                                user.username
                                            );
                                            let config = server_config();
                                            let desktop_configuration = Token::new(
                                                user.id,
                                                Some(user.id),
                                                Some(user.email),
                                                config.enrollment_token_timeout.as_secs(),
                                                Some(ENROLLMENT_TOKEN_TYPE.to_string()),
                                            );
                                            debug!("Saving a new desktop configuration token...");
                                            desktop_configuration.save(&pool).await?;
                                            debug!(
                                                "Saved desktop configuration token. Responding to \
                                            proxy with the token."
                                            );

                                            Some(core_response::Payload::AuthCallback(
                                                AuthCallbackResponse {
                                                    url: config.enrollment_url.clone().into(),
                                                    token: desktop_configuration.id,
                                                },
                                            ))
                                        }
                                        Err(err) => {
                                            let message = format!("OpenID auth error {err}");
                                            error!(message);
                                            Some(core_response::Payload::CoreError(CoreError {
                                                status_code: Code::Internal as i32,
                                                message,
                                            }))
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!(
                                        "Proxy requested an OpenID authentication info for a callback \
                                    URL ({}) that couldn't be parsed. Details: {err}",
                                        request.callback_url
                                    );
                                    Some(core_response::Payload::CoreError(CoreError {
                                        status_code: Code::Internal as i32,
                                        message: "invalid callback URL".into(),
                                    }))
                                }
                            }
                        }
                        // Reply without payload.
                        None => None,
                    };

                    let req = CoreResponse {
                        id: received.id,
                        payload,
                    };
                    if let Some(txs) = self.router.write().unwrap().route_response(&req) {
                        for tx in txs {
                            let _ = tx.send(req.clone());
                        }
                    } else {
                        let _ = tx.send(req);
                    };
                }
                Err(err) => {
                    error!("Disconnected from proxy at {}: {err}", self.url);
                    debug!("waiting 10s to re-establish the connection");
                    sleep(TEN_SECS).await;
                    break 'message;
                }
            }
        }

        Ok(())
    }
}

/// Groups Core-side service handlers used to process requests originating
/// from a proxy instance.
///
/// Each `ProxyServices` instance is owned by a single `Proxy` and provides
/// the concrete handlers for enrollment, authentication, and polling-related
/// requests received over the gRPC bidirectional stream.
struct ProxyServices {
    enrollment: EnrollmentServer,
    password_reset: PasswordResetServer,
    client_mfa: ClientMfaServer,
    polling: PollingServer,
}

impl ProxyServices {
    pub fn new(pool: PgPool, tx: ProxyTxSet) -> Self {
        let enrollment = EnrollmentServer::new(
            pool.clone(),
            tx.wireguard.clone(),
            tx.mail.clone(),
            tx.bidi_events.clone(),
        );
        let password_reset =
            PasswordResetServer::new(pool.clone(), tx.mail.clone(), tx.bidi_events.clone());
        let client_mfa = ClientMfaServer::new(
            pool.clone(),
            tx.mail.clone(),
            tx.wireguard.clone(),
            tx.bidi_events.clone(),
        );
        let polling = PollingServer::new(pool.clone());

        Self {
            enrollment,
            password_reset,
            client_mfa,
            polling,
        }
    }
}
