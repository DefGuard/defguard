use std::{
    collections::HashMap,
    str::FromStr,
    sync::{Arc, RwLock},
};

use axum_extra::extract::cookie::Key;
use defguard_common::{
    VERSION,
    config::server_config,
    db::{
        Id,
        models::{Settings, proxy::Proxy},
    },
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
        gateway::events::GatewayEvent,
        proxy::client_mfa::{ClientLoginSession, ClientMfaServer},
    },
    version::{IncompatibleComponents, IncompatibleProxyData, is_proxy_version_supported},
};
use defguard_proto::proxy::{
    AuthCallbackResponse, AuthInfoResponse, CoreError, CoreRequest, CoreResponse, InitialInfo,
    core_request, core_response, proxy_client::ProxyClient,
};
use defguard_version::{
    ComponentInfo, DefguardComponent, client::ClientVersionInterceptor, get_tracing_variables,
};
use http::Uri;
use hyper_rustls::HttpsConnectorBuilder;
use openidconnect::{AuthorizationCode, Nonce, Scope, core::CoreAuthenticationFlow};
use reqwest::Url;
use secrecy::ExposeSecret;
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

use crate::{
    ProxyError, ProxyTxSet, TEN_SECS,
    certs::client_config,
    servers::{EnrollmentServer, PasswordResetServer},
};

static VERSION_ZERO: Version = Version::new(0, 0, 0);

type ShutdownReceiver = tokio::sync::oneshot::Receiver<bool>;

/// Represents a single Core - Proxy connection.
///
/// A `ProxyHandler` is responsible for establishing and maintaining a gRPC
/// bidirectional stream to one proxy instance, handling incoming requests
/// from that proxy, and forwarding responses back through the same stream.
/// Each `ProxyHandler` runs independently and is supervised by the
/// `ProxyManager`.
pub(super) struct ProxyHandler {
    pool: PgPool,
    /// gRPC servers
    services: ProxyServices,
    /// Proxy server gRPC URL
    pub(super) url: Url,
    shutdown_signal: Arc<Mutex<Option<ShutdownReceiver>>>,
    proxy_id: Id,
    client: Option<ProxyClient<InterceptedService<Channel, ClientVersionInterceptor>>>,
}

impl ProxyHandler {
    pub(super) fn new(
        pool: PgPool,
        url: Url,
        tx: &ProxyTxSet,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
        shutdown_signal: Arc<Mutex<Option<ShutdownReceiver>>>,
        proxy_id: Id,
    ) -> Self {
        // Instantiate gRPC servers.
        let services = ProxyServices::new(&pool, tx, remote_mfa_responses, sessions);

        Self {
            pool,
            services,
            url,
            shutdown_signal,
            proxy_id,
            client: None,
        }
    }

    pub(super) fn from_proxy(
        proxy: &Proxy<Id>,
        pool: PgPool,
        tx: &ProxyTxSet,
        remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>>,
        sessions: Arc<RwLock<HashMap<String, ClientLoginSession>>>,
        shutdown_signal: Arc<Mutex<Option<ShutdownReceiver>>>,
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
        ))
    }

    async fn mark_connected(&self, version: &Version) -> Result<(), ProxyError> {
        if let Some(mut proxy) = Proxy::find_by_id(&self.pool, self.proxy_id).await? {
            proxy
                .mark_connected(&self.pool, &version.to_string())
                .await?;
        } else {
            warn!("Couldn't find proxy by id, URL: {}", self.url);
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

    fn endpoint(&self) -> Result<Endpoint, ProxyError> {
        let mut url = self.url.clone();

        // Using http here because the connector upgrades to TLS internally.
        url.set_scheme("http").map_err(|()| {
            ProxyError::UrlError(format!("Failed to set http scheme on URL {url}"))
        })?;
        let endpoint = Endpoint::from_shared(url.to_string())?;
        let endpoint = endpoint
            .http2_keep_alive_interval(TEN_SECS)
            .tcp_keepalive(Some(TEN_SECS))
            .keep_alive_while_idle(true);

        Ok(endpoint)
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
        loop {
            let endpoint = self.endpoint()?;
            let settings = Settings::get_current_settings();
            let Some(ca_cert_der) = settings.ca_cert_der else {
                return Err(ProxyError::MissingConfiguration(
                    "Core CA is not setup, can't create a Proxy endpoint.".to_string(),
                ));
            };
            let tls_config = client_config(&ca_cert_der, certs_rx.clone(), self.proxy_id)?;
            let connector = HttpsConnectorBuilder::new()
                .with_tls_config(tls_config)
                .https_only()
                .enable_http2()
                .build();
            let connector = HttpsSchemeConnector::new(connector);

            debug!("Connecting to proxy at {}", endpoint.uri());
            let interceptor = ClientVersionInterceptor::new(Version::parse(VERSION)?);
            let channel = endpoint.connect_with_connector_lazy(connector);
            let mut client = ProxyClient::with_interceptor(channel, interceptor);
            self.client = Some(client.clone());
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
                    self.mark_disconnected().await?;
                    sleep(TEN_SECS).await;
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
                sleep(TEN_SECS).await;
                continue;
            }
            IncompatibleComponents::remove_proxy(&incompatible_components);

            info!("Connected to proxy at {}", endpoint.uri());
            let mut resp_stream = response.into_inner();

            // Derive proxy cookie key from core secret to avoid transmitting it over gRPC.
            let config = server_config();
            let proxy_cookie_key = Key::derive_from(config.secret_key.expose_secret().as_bytes());

            // Send initial info with private cookies key.
            let initial_info = InitialInfo {
                private_cookies_key: proxy_cookie_key.master().to_vec(),
            };
            let _ = tx.send(CoreResponse {
                id: 0,
                payload: Some(core_response::Payload::InitialInfo(initial_info)),
            });

            let shutdown_signal = self.shutdown_signal.lock().await.take();
            if let Some(shutdown_signal) = shutdown_signal {
                select! {
                    res = self.message_loop(tx, tx_set.wireguard.clone(), &mut resp_stream) => {
                        if let Err(err) = res {
                            error!("Proxy message loop ended with error: {err}, reconnecting in {TEN_SECS:?}",);
                        } else {
                            info!("Proxy message loop ended, reconnecting in {TEN_SECS:?}");
                        }
                        self.mark_disconnected().await?;
                        sleep(TEN_SECS).await;
                    }
                    res = shutdown_signal => {
                        match res {
                            Err(err) => {
                                error!("An error occurred when trying to wait for a shutdown signal for Proxy: {err}. Reconnecting to: {}", endpoint.uri());
                            }
                            Ok(purge) => {
                                info!("Shutdown signal received, purge: {purge}, stopping proxy connection to {}", endpoint.uri());
                                if purge {
                                    debug!("Sending purge request to proxy {}", endpoint.uri());
                                    if let Some(client) = self.client.as_mut() {
                                        if let Err(err) = client.purge(Request::new(())).await {
                                            error!("Error sending purge request to proxy {}: {err}", endpoint.uri());
                                        }
                                    }
                                }
                            }
                        }
                        self.mark_disconnected().await?;
                        break;
                    }
                }
            } else {
                self.message_loop(tx, tx_set.wireguard.clone(), &mut resp_stream)
                    .await?;
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
                                            let settings = Settings::get_current_settings();
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
                    debug!("waiting 10s to re-establish the connection");
                    self.mark_disconnected().await?;
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

/// Rewrites the request URI scheme to https for the TLS connector.
///
/// Tonic expects an http URI for its endpoint, but our custom connector performs
/// the TLS handshake and requires https to select the TLS path.
#[derive(Clone, Debug)]
struct HttpsSchemeConnector<C> {
    inner: C,
}

impl<C> HttpsSchemeConnector<C> {
    const fn new(inner: C) -> Self {
        Self { inner }
    }
}

type BoxError = Box<dyn std::error::Error + Send + Sync>;

impl<C> tower_service::Service<Uri> for HttpsSchemeConnector<C>
where
    C: tower_service::Service<Uri, Error = BoxError> + Clone + Send + 'static,
    C::Future: Send,
{
    type Response = C::Response;
    type Error = BoxError;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let mut parts = uri.into_parts();
        parts.scheme = Some(http::uri::Scheme::HTTPS);
        let https_uri = match Uri::from_parts(parts) {
            Ok(uri) => uri,
            Err(err) => {
                return Box::pin(async move { Err(err.into()) });
            }
        };
        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(https_uri).await })
    }
}
