#[cfg(test)]
use std::path::PathBuf;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
    time::Duration,
};

use axum_extra::extract::cookie::Key;
use chrono::NaiveDateTime;
use defguard_common::{
    VERSION,
    db::{
        Id,
        models::{Certificates, Settings, proxy::Proxy},
    },
    types::AuthFlowType,
};
use defguard_core::{
    db::models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token},
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
    grpc::{
        GatewayEvent,
        proxy::client_mfa::{ClientLoginSession, ClientMfaServer},
    },
    version::{IncompatibleComponents, IncompatibleProxyData, is_proxy_version_supported},
};
use defguard_grpc_tls::{certs as tls_certs, connector::HttpsSchemeConnector};
use defguard_proto::{
    client_types::AuthFlowType as ProtoAuthFlowType,
    proxy::{
        AuthCallbackResponse, AuthInfoResponse, CoreError, CoreRequest, CoreResponse, HttpsCerts,
        InitialInfo, core_request, core_response, proxy_client::ProxyClient,
    },
};
use defguard_version::{
    ComponentInfo, DefguardComponent, client::ClientVersionInterceptor, get_tracing_variables,
};
use hyper_rustls::HttpsConnectorBuilder;
use openidconnect::{AuthorizationCode, Nonce, Scope, core::CoreAuthenticationFlow};
use reqwest::Url;
use semver::Version;
use sqlx::PgPool;
use tokio::{
    select,
    sync::{
        Mutex,
        broadcast::Sender,
        mpsc::{self, UnboundedSender},
        oneshot, watch,
    },
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    Code, Request, Streaming,
    service::interceptor::InterceptedService,
    transport::{Channel, Endpoint},
};

#[cfg(test)]
use crate::ProxyManagerTestSupport;
use crate::{
    HandlerTxMap, ProxyError, ProxyTxSet, TEN_SECS,
    servers::{EnrollmentServer, PasswordResetServer},
};

const VERSION_ZERO: Version = Version::new(0, 0, 0);

type ShutdownReceiver = tokio::sync::oneshot::Receiver<bool>;

#[cfg(test)]
#[derive(Default)]
struct ProxyTestTransport {
    socket_path: Option<PathBuf>,
}

#[cfg(test)]
impl ProxyTestTransport {
    fn with_socket_path(socket_path: PathBuf) -> Self {
        Self {
            socket_path: Some(socket_path),
        }
    }

    fn socket_path(&self) -> Option<&PathBuf> {
        self.socket_path.as_ref()
    }
}

/// Represents a single Core - Proxy connection.
///
/// A `ProxyHandler` is responsible for establishing and maintaining a gRPC
/// bidirectional stream to one proxy instance, handling incoming requests
/// from that proxy, and forwarding responses back through the same stream.
pub(super) struct ProxyHandler {
    pool: PgPool,
    /// gRPC servers
    services: ProxyServices,
    /// Proxy server gRPC URL
    pub(super) url: Url,
    shutdown_signal: Arc<Mutex<ShutdownReceiver>>,
    proxy_id: Id,
    proxy_cookie_key: Key,
    client: Option<ProxyClient<InterceptedService<Channel, ClientVersionInterceptor>>>,
    /// Shared map used to register this handler's active stream sender so the manager
    /// can push messages to a specific proxy.
    handler_tx_map: HandlerTxMap,
    #[cfg(test)]
    test_transport: ProxyTestTransport,
    #[cfg(test)]
    test_support: Option<ProxyManagerTestSupport>,
}

impl ProxyHandler {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        pool: PgPool,
        url: Url,
        tx: &ProxyTxSet,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
        shutdown_signal: Arc<Mutex<ShutdownReceiver>>,
        proxy_id: Id,
        proxy_cookie_key: Key,
        handler_tx_map: HandlerTxMap,
    ) -> Self {
        // Instantiate gRPC servers.
        let services = ProxyServices::new(&pool, tx, remote_mfa_responses, sessions);

        Self {
            pool,
            services,
            url,
            shutdown_signal,
            proxy_id,
            proxy_cookie_key,
            client: None,
            handler_tx_map,
            #[cfg(test)]
            test_transport: ProxyTestTransport::default(),
            #[cfg(test)]
            test_support: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn from_proxy(
        proxy: &Proxy<Id>,
        pool: PgPool,
        tx: &ProxyTxSet,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
        shutdown_signal: Arc<Mutex<ShutdownReceiver>>,
        proxy_cookie_key: Key,
        handler_tx_map: HandlerTxMap,
    ) -> Result<Self, ProxyError> {
        let url = Url::from_str(&format!("http://{}:{}", proxy.address, proxy.port))?;
        let proxy_id = proxy.id;
        Ok(Self::new(
            pool,
            url,
            tx,
            remote_mfa_responses,
            sessions,
            shutdown_signal,
            proxy_id,
            proxy_cookie_key,
            handler_tx_map,
        ))
    }

    async fn mark_connected(&self, version: &Version) -> Result<(), ProxyError> {
        if let Some(mut proxy) = Proxy::find_by_id(&self.pool, self.proxy_id).await? {
            proxy
                .mark_connected(&self.pool, version.to_string())
                .await?;
        } else {
            warn!("Couldn't find Proxy by ID for URL: {}", self.url);
        }

        Ok(())
    }

    async fn mark_disconnected(&self) -> Result<(), ProxyError> {
        let Some(mut proxy) = Proxy::find_by_id(&self.pool, self.proxy_id).await? else {
            warn!("Couldn't find proxy by id, URL: {}", self.url);
            return Ok(());
        };

        // Make sure we don't continuously update disconnected time in connection loop
        let should_mark = match (proxy.connected_at, proxy.disconnected_at) {
            (Some(connected), Some(disconnected)) => disconnected < connected,
            (Some(_), None) => true,
            _ => false,
        };

        if should_mark {
            proxy.mark_disconnected(&self.pool).await?;
        }

        Ok(())
    }

    fn retry_delay(&self) -> Duration {
        #[cfg(test)]
        {
            return self.handler_retry_delay();
        }
        #[cfg_attr(test, allow(unreachable_code))]
        TEN_SECS
    }

    fn endpoint(&self) -> Result<Endpoint, ProxyError> {
        let mut url = self.url.clone();

        // Using HTTP here because the connector upgrades to TLS internally.
        url.set_scheme("http").map_err(|()| {
            ProxyError::UrlError(format!("Failed to set HTTP scheme on URL {url}"))
        })?;
        let endpoint = Endpoint::from_shared(url.to_string())?;
        let endpoint = endpoint
            .http2_keep_alive_interval(TEN_SECS)
            .tcp_keepalive(Some(TEN_SECS))
            .keep_alive_while_idle(true);

        Ok(endpoint)
    }

    async fn connect_tls_channel(
        &self,
        endpoint: &Endpoint,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<Channel, ProxyError> {
        let certs = Certificates::get(&self.pool)
            .await
            .map_err(ProxyError::SqlxError)?
            .ok_or_else(|| {
                ProxyError::MissingConfiguration(
                    "Core CA is not setup, can't create a Proxy endpoint.".to_string(),
                )
            })?;
        let ca_cert_der = certs.ca_cert_der.ok_or_else(|| {
            ProxyError::MissingConfiguration(
                "Core CA is not setup, can't create a Proxy endpoint.".to_string(),
            )
        })?;

        // Load the Proxy model to retrieve the per-component Core client cert.
        let proxy = Proxy::find_by_id(&self.pool, self.proxy_id)
            .await
            .map_err(ProxyError::SqlxError)?
            .ok_or_else(|| {
                ProxyError::MissingConfiguration(format!(
                    "Proxy id={} not found in DB, can't load Core client certificate",
                    self.proxy_id
                ))
            })?;
        let core_client_cert_der = proxy.core_client_cert_der.ok_or_else(|| {
            ProxyError::MissingConfiguration(format!(
                "Core client certificate not provisioned for proxy id={}",
                self.proxy_id
            ))
        })?;
        let core_client_cert_key_der = proxy.core_client_cert_key_der.ok_or_else(|| {
            ProxyError::MissingConfiguration(format!(
                "Core client certificate key not provisioned for proxy id={}",
                self.proxy_id
            ))
        })?;

        let tls_config = tls_certs::client_config(
            &ca_cert_der,
            certs_rx,
            self.proxy_id,
            &core_client_cert_der,
            &core_client_cert_key_der,
        )
        .map_err(|err| ProxyError::TlsConfigError(err.to_string()))?;
        let connector = HttpsConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_only()
            .enable_http2()
            .build();
        let connector = HttpsSchemeConnector::new(connector);
        Ok(endpoint.connect_with_connector_lazy(connector))
    }

    #[cfg(not(test))]
    async fn connect_channel(
        &self,
        endpoint: &Endpoint,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<Channel, ProxyError> {
        self.connect_tls_channel(endpoint, certs_rx).await
    }

    /// Establishes and maintains a gRPC bidirectional stream to the proxy.
    ///
    /// The proxy connection is retried on failure, compatibility is checked
    /// on each successful connection, and incoming messages are handled
    /// until the stream is closed.
    pub(super) async fn run(
        mut self,
        tx_set: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<(), ProxyError> {
        let parsed_version = Version::parse(VERSION)?;
        loop {
            let endpoint = self.endpoint()?;

            let channel = match self.connect_channel(&endpoint, certs_rx.clone()).await {
                Ok(ch) => ch,
                Err(err) => {
                    error!(
                        "Failed to create proxy channel for {}: {err}, retrying in {:?}",
                        endpoint.uri(),
                        self.retry_delay()
                    );
                    self.mark_disconnected().await?;
                    sleep(self.retry_delay()).await;
                    continue;
                }
            };

            debug!("Connecting to proxy at {}", endpoint.uri());
            let interceptor = ClientVersionInterceptor::new(parsed_version.clone());
            let mut client = ProxyClient::with_interceptor(channel, interceptor);
            self.client = Some(client.clone());
            let (tx, rx) = mpsc::unbounded_channel();

            // Register this handler's sender so the manager can push messages to this proxy.
            if let Ok(mut map) = self.handler_tx_map.write() {
                map.insert(self.proxy_id, tx.clone());
            }

            let response = match client.bidi(UnboundedReceiverStream::new(rx)).await {
                Ok(response) => response,
                Err(err) => {
                    match err.code() {
                        Code::FailedPrecondition => {
                            error!(
                                "Failed to connect to proxy @ {}, version check failed, retrying in \
                            {:?}: {err}",
                                endpoint.uri(),
                                self.retry_delay()
                            );
                            // TODO push event
                        }
                        err => {
                            error!(
                                "Failed to connect to proxy @ {}, retrying in {:?}: {err}",
                                endpoint.uri(),
                                self.retry_delay()
                            );
                        }
                    }
                    // Deregister tx on connection failure.
                    if let Ok(mut map) = self.handler_tx_map.write() {
                        map.remove(&self.proxy_id);
                    }
                    self.mark_disconnected().await?;
                    sleep(self.retry_delay()).await;
                    continue;
                }
            };
            let maybe_info = ComponentInfo::from_metadata(response.metadata());

            // Check proxy version and continue if it's not supported.
            let (version, info) = get_tracing_variables(&maybe_info);
            let proxy_is_supported = is_proxy_version_supported(Some(&version));
            self.mark_connected(&version).await?;

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
                sleep(self.retry_delay()).await;
                continue;
            }
            IncompatibleComponents::remove_proxy(&incompatible_components);

            info!("Connected to proxy at {}", endpoint.uri());
            let mut resp_stream = response.into_inner();

            // Send initial info with private cookies key.
            let initial_info = InitialInfo {
                private_cookies_key: self.proxy_cookie_key.master().to_vec(),
            };
            let _ = tx.send(CoreResponse {
                id: 0,
                payload: Some(core_response::Payload::InitialInfo(initial_info)),
            });

            // If a certificate has already been provisioned, push it to the newly-connected
            // proxy immediately so it can start serving HTTPS without a manual trigger.
            // The active source determines which cert/key pair to send.
            match Certificates::get(&self.pool).await {
                Ok(Some(certs)) => {
                    if let Some((cert_pem, key_pem)) = certs.proxy_http_cert_pair() {
                        info!(
                            "Sending stored {:?} certificate to proxy {} on connect",
                            certs.proxy_http_cert_source, self.proxy_id
                        );
                        let _ = tx.send(CoreResponse {
                            id: 0,
                            payload: Some(core_response::Payload::HttpsCerts(HttpsCerts {
                                cert_pem: cert_pem.to_string(),
                                key_pem: key_pem.to_string(),
                            })),
                        });
                    }
                }
                Ok(None) => {
                    warn!("Certificates row not found; skipping cert push on connect");
                }
                Err(err) => {
                    error!("Failed to load certificates for cert push on connect: {err}");
                }
            }

            let shutdown_signal = Arc::clone(&self.shutdown_signal);
            select! {
                res = self.message_loop(tx, tx_set.wireguard.clone(), &mut resp_stream) => {
                    if let Err(err) = res {
                        error!("Proxy message loop ended with error: {err}, reconnecting in {:?}", self.retry_delay());
                    } else {
                        info!("Proxy message loop ended, reconnecting in {:?}", self.retry_delay());
                    }
                    if let Ok(mut map) = self.handler_tx_map.write() {
                        map.remove(&self.proxy_id);
                    }
                    self.mark_disconnected().await?;
                    sleep(self.retry_delay()).await;
                }
                res = &mut *shutdown_signal.lock().await => {
                    match res {
                        Err(err) => {
                            error!("An error occurred when trying to wait for a shutdown signal for Proxy: {err}. Reconnecting to: {}", endpoint.uri());
                        }
                        Ok(purge) => {
                            info!("Shutdown signal received, purge: {purge}, stopping proxy connection to {}", endpoint.uri());
                            if purge {
                                if let Some(client) = self.client.as_mut() {
                                    debug!("Sending purge request to proxy {}", endpoint.uri());
                                    if let Err(err) = client.purge(Request::new(())).await {
                                        error!("Error sending purge request to proxy {}: {err}", endpoint.uri());
                                    } else {
                                        info!("Sent purge request to proxy {}", endpoint.uri());
                                    }
                                }
                            }
                        }
                    }
                    if let Ok(mut map) = self.handler_tx_map.write() {
                        map.remove(&self.proxy_id);
                    }
                    self.mark_disconnected().await?;
                    break;
                }
            }
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
                        // rpc ClientRemoteMfaFinish (ClientRemoteMfaFinishRequest) returns (ClientRemoteMfaFinishResponse)
                        Some(core_request::Payload::AwaitRemoteMfaFinish(request)) => {
                            match self
                                .services
                                .client_mfa
                                .await_remote_mfa_login(request, tx.clone(), received.id)
                                .await
                            {
                                Ok(()) => None,
                                Err(err) => {
                                    error!("Client remote MFA finish error: {err}");
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
                            if is_business_license_active() {
                                let redirect_url = match request.auth_flow_type() {
                                    ProtoAuthFlowType::Enrollment => {
                                        let settings = Settings::get_current_settings();
                                        settings.edge_callback_url(AuthFlowType::Enrollment)
                                    }
                                    ProtoAuthFlowType::Mfa => {
                                        let settings = Settings::get_current_settings();
                                        settings.edge_callback_url(AuthFlowType::Mfa)
                                    }
                                    // fall back for legacy pre-2.0 clients
                                    ProtoAuthFlowType::Unspecified =>
                                    {
                                        #[allow(deprecated)]
                                        Url::parse(&request.redirect_url)
                                    }
                                };

                                if let Ok(redirect_url) = redirect_url {
                                    if let Some(provider) =
                                        OpenIdProvider::get_current(&pool).await?
                                    {
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
                                    error!("Invalid redirect URL in authentication info request");
                                    Some(core_response::Payload::CoreError(CoreError {
                                        status_code: Code::Internal as i32,
                                        message: "invalid redirect URL".into(),
                                    }))
                                }
                            } else {
                                warn!("Enterprise license required");
                                Some(core_response::Payload::CoreError(CoreError {
                                    status_code: Code::FailedPrecondition as i32,
                                    message: "no valid license".into(),
                                }))
                            }
                        }
                        Some(core_request::Payload::AuthCallback(request)) => {
                            match Settings::get_current_settings()
                                .edge_callback_url(AuthFlowType::Enrollment)
                            {
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
                                            let settings = Settings::get_current_settings();
                                            let desktop_configuration = Token::new(
                                                user.id,
                                                Some(user.id),
                                                Some(user.email),
                                                settings.enrollment_token_timeout().as_secs(),
                                                Some(ENROLLMENT_TOKEN_TYPE.to_string()),
                                            );
                                            debug!("Saving a new desktop configuration token...");
                                            desktop_configuration.save(&pool).await?;
                                            debug!(
                                                "Saved desktop configuration token. Responding to \
                                            proxy with the token."
                                            );
                                            let public_proxy_url = settings.proxy_public_url()?;

                                            Some(core_response::Payload::AuthCallback(
                                                AuthCallbackResponse {
                                                    url: public_proxy_url.into(),
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
                                    URL that couldn't be built. Details: {err}"
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
                        // rpc AcmeCertificate: proxy completed ACME issuance.
                        Some(core_request::Payload::AcmeCertificate(cert)) => {
                            info!("Received AcmeCertificate from proxy, saving and broadcasting");
                            // Parse the cert expiry from the PEM so we can store it.
                            let acme_cert_expiry = parse_cert_expiry(&cert.cert_pem);
                            // Load current certificates row, patch ACME fields, and save.
                            match Certificates::get_or_default(&pool).await {
                                Ok(mut certs) => {
                                    certs.proxy_http_cert_pem = Some(cert.cert_pem.clone());
                                    certs.proxy_http_cert_key_pem = Some(cert.key_pem.clone());
                                    certs.acme_account_credentials =
                                        Some(cert.account_credentials_json.clone());
                                    certs.proxy_http_cert_expiry = acme_cert_expiry;
                                    certs.proxy_http_cert_source =
                                        defguard_common::db::models::ProxyCertSource::LetsEncrypt;
                                    if let Err(err) = certs.save(&pool).await {
                                        error!(
                                            "Failed to save ACME certificate to certificates: {err}"
                                        );
                                    } else {
                                        info!("ACME certificate saved to certificates");
                                        // Broadcast HttpsCerts to ALL connected proxies.
                                        let https_certs = CoreResponse {
                                            id: 0,
                                            payload: Some(core_response::Payload::HttpsCerts(
                                                HttpsCerts {
                                                    cert_pem: cert.cert_pem,
                                                    key_pem: cert.key_pem,
                                                },
                                            )),
                                        };
                                        if let Ok(map) = self.handler_tx_map.read() {
                                            for (pid, handler_tx) in map.iter() {
                                                debug!("Broadcasting HttpsCerts to proxy {pid}");
                                                let _ = handler_tx.send(https_certs.clone());
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!("Failed to load certificates for ACME save: {err}");
                                }
                            }
                            None
                        }
                    };

                    if let Some(payload) = payload {
                        let req = CoreResponse {
                            id: received.id,
                            payload: Some(payload),
                        };
                        let _ = tx.send(req);
                    }
                }
                Err(err) => {
                    error!("Disconnected from proxy at {}: {err}", self.url);
                    self.mark_disconnected().await?;
                    break 'message;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
impl ProxyHandler {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_test_socket(
        pool: PgPool,
        url: Url,
        tx: &ProxyTxSet,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
        shutdown_signal: Arc<Mutex<ShutdownReceiver>>,
        proxy_id: Id,
        proxy_cookie_key: Key,
        socket_path: PathBuf,
    ) -> Self {
        let handler_tx_map: HandlerTxMap = Arc::new(RwLock::new(HashMap::new()));
        let mut handler = Self::new(
            pool,
            url,
            tx,
            remote_mfa_responses,
            sessions,
            shutdown_signal,
            proxy_id,
            proxy_cookie_key,
            handler_tx_map,
        );
        handler.test_transport = ProxyTestTransport::with_socket_path(socket_path);
        handler
    }

    pub(crate) fn attach_test_support(&mut self, test_support: ProxyManagerTestSupport) {
        self.test_support = Some(test_support);
    }

    /// Override the transport to connect via a Unix socket instead of TLS.
    ///
    /// Used in manager-level tests where the handler must share the manager's
    /// `handler_tx_map` (constructed via `from_proxy`) but still reach a mock
    /// proxy over a Unix socket.
    pub(crate) fn set_test_socket_path(&mut self, socket_path: PathBuf) {
        self.test_transport = ProxyTestTransport::with_socket_path(socket_path);
    }

    fn handler_retry_delay(&self) -> Duration {
        self.test_support
            .as_ref()
            .map_or(TEN_SECS, ProxyManagerTestSupport::handler_reconnect_delay)
    }

    async fn connect_channel(
        &self,
        endpoint: &Endpoint,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<Channel, ProxyError> {
        if let Some(socket_path) = self.test_transport.socket_path().cloned() {
            return Ok(endpoint.connect_with_connector_lazy(tower::service_fn(
                move |_: tonic::transport::Uri| {
                    let socket_path = socket_path.clone();
                    async move {
                        Ok::<_, std::io::Error>(hyper_util::rt::TokioIo::new(
                            tokio::net::UnixStream::connect(socket_path).await?,
                        ))
                    }
                },
            )));
        }

        self.connect_tls_channel(endpoint, certs_rx).await
    }

    /// Single-iteration version of `run()` for use in tests.
    ///
    /// Attempts one connection to the proxy, processes the bidirectional
    /// stream until it closes or an error occurs, then returns. Does not
    /// retry or loop.
    pub(crate) async fn run_once(
        mut self,
        tx_set: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    ) -> Result<(), ProxyError> {
        let parsed_version = Version::parse(VERSION)?;
        let endpoint = self.endpoint()?;
        let channel = self.connect_channel(&endpoint, certs_rx).await?;

        debug!(
            "Connecting to proxy at {} (test, single iteration)",
            endpoint.uri()
        );
        let interceptor = ClientVersionInterceptor::new(parsed_version);
        let mut client = ProxyClient::with_interceptor(channel, interceptor);
        self.client = Some(client.clone());
        let (tx, rx) = mpsc::unbounded_channel();
        let response = match client.bidi(UnboundedReceiverStream::new(rx)).await {
            Ok(response) => response,
            Err(err) => {
                self.mark_disconnected().await?;
                return Err(err.into());
            }
        };
        let maybe_info = ComponentInfo::from_metadata(response.metadata());

        let (version, info) = get_tracing_variables(&maybe_info);
        let proxy_is_supported = is_proxy_version_supported(Some(&version));
        self.mark_connected(&version).await?;

        let span = tracing::info_span!("proxy_bidi", component = %DefguardComponent::Proxy,
            version = version.to_string(), info);
        let _guard = span.enter();
        if !proxy_is_supported {
            let maybe_version = if version == VERSION_ZERO {
                None
            } else {
                Some(version)
            };
            let data = IncompatibleProxyData::new(maybe_version);
            data.insert(&incompatible_components);
            self.mark_disconnected().await?;
            return Ok(());
        }
        IncompatibleComponents::remove_proxy(&incompatible_components);

        info!("Connected to proxy at {} (test)", endpoint.uri());
        let mut resp_stream = response.into_inner();

        let initial_info = InitialInfo {
            private_cookies_key: self.proxy_cookie_key.master().to_vec(),
        };
        let _ = tx.send(CoreResponse {
            id: 0,
            payload: Some(core_response::Payload::InitialInfo(initial_info)),
        });

        let result = self
            .message_loop(tx, tx_set.wireguard.clone(), &mut resp_stream)
            .await;
        self.mark_disconnected().await?;
        result
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
    pub fn new(
        pool: &PgPool,
        tx: &ProxyTxSet,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
    ) -> Self {
        let enrollment =
            EnrollmentServer::new(pool.clone(), tx.wireguard.clone(), tx.bidi_events.clone());
        let password_reset = PasswordResetServer::new(pool.clone(), tx.bidi_events.clone());
        let client_mfa = ClientMfaServer::new(
            pool.clone(),
            tx.wireguard.clone(),
            tx.bidi_events.clone(),
            remote_mfa_responses,
            sessions,
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

/// Parse the `not_after` expiry timestamp from a PEM-encoded certificate.
///
/// Returns `None` if the PEM is unparseable (e.g. empty or malformed), so the
/// caller can still proceed without a stored expiry.
fn parse_cert_expiry(cert_pem: &str) -> Option<NaiveDateTime> {
    use defguard_certs::{CertificateInfo, parse_pem_certificate};

    let der = parse_pem_certificate(cert_pem)
        .map_err(|e| warn!("Failed to parse ACME cert PEM for expiry: {e}"))
        .ok()?;
    CertificateInfo::from_der(&der)
        .map(|info| info.not_after)
        .map_err(|e| warn!("Failed to extract expiry from ACME cert: {e}"))
        .ok()
}
