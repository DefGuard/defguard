use std::{
    collections::HashMap,
    fs::read_to_string,
    sync::{Arc, RwLock},
    time::Duration,
};

use axum::http::Uri;
use openidconnect::{AuthorizationCode, Nonce, Scope, core::CoreAuthenticationFlow};
use reqwest::Url;
use semver::Version;
use sqlx::PgPool;
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

use defguard_common::{VERSION, config::server_config, db::Id};
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
        is_enterprise_enabled,
        ldap::utils::ldap_update_user_state,
    },
    events::BidiStreamEvent,
    grpc::{gateway::events::GatewayEvent, proxy::client_mfa::ClientMfaServer},
    version::{IncompatibleComponents, IncompatibleProxyData, is_proxy_version_supported},
};
use defguard_mail::Mail;
use defguard_proto::proxy::{
    AuthCallbackResponse, AuthInfoResponse, CoreError, CoreRequest, CoreResponse, core_request,
    core_response, proxy_client::ProxyClient,
};
use defguard_version::{
    ComponentInfo, DefguardComponent, client::ClientVersionInterceptor, get_tracing_variables,
};

use crate::{enrollment::EnrollmentServer, password_reset::PasswordResetServer};

mod enrollment;
pub(crate) mod password_reset;

#[macro_use]
extern crate tracing;

const TEN_SECS: Duration = Duration::from_secs(10);
static VERSION_ZERO: Version = Version::new(0, 0, 0);

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
                error!("### Registering ClientMfaTokenValidation request");
                self.response_map
                    .insert(request.token.clone(), vec![sender.clone()]);
            }
            Some(core_request::Payload::ClientMfaFinish(request)) => {
                error!("### Registering ClientMfaFinish request");
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
        match &response.payload {
            // Mobile-assisted MFA completion responses must go to the proxy that owns the WebSocket
            // so it can send the preshared key.
            // Corresponds to the `core_request::Payload::ClientMfaTokenValidation(request)` request.
            // https://github.com/DefGuard/defguard/issues/1700
            Some(core_response::Payload::ClientMfaFinish(response)) => {
                if let Some(ref token) = response.token {
                    error!("### Routing ClientMfaFinish response");
                    return self.response_map.remove(token);
                }
            }
            _ => {}
        }
        None
    }
}

/// TODO(jck) rustdoc, list orchestrator's responsibilities
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
            router: Default::default(),
        }
    }

    /// TODO(jck) Retrieves proxies from the db and runs them
    // TODO(jck) consider new error type
    pub async fn run(self) -> Result<(), anyhow::Error> {
        // TODO(jck) retrieve proxies from db
        let proxies = vec![
            Proxy::new(
                1,
                self.pool.clone(),
                Uri::from_static("http://localhost:50051"),
                self.tx.clone(),
            )?,
            Proxy::new(
                2,
                self.pool.clone(),
                Uri::from_static("http://localhost:50052"),
                self.tx.clone(),
            )?,
        ];
        let mut tasks = JoinSet::<Result<(), anyhow::Error>>::new();
        for proxy in proxies {
            tasks.spawn(proxy.run(
                self.tx.clone(),
                self.incompatible_components.clone(),
                self.router.clone(),
            ));
        }
        // TODO(jck) handle empty proxies vec somewhere earlier
        while let Some(result) = tasks.join_next().await {
            match result {
                // TODO(jck) add proxy id/name to the error log
                Ok(Ok(())) => error!("Proxy task returned prematurely"),
                Ok(Err(err)) => error!("Proxy task returned with error: {err}"),
                Err(err) => error!("Proxy task execution failed: {err}"),
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct ProxyTxSet {
    wireguard: Sender<GatewayEvent>,
    mail: UnboundedSender<Mail>,
    bidi_events: UnboundedSender<BidiStreamEvent>,
}

impl ProxyTxSet {
    pub fn new(
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

/// Groups all proxy GRPC servers
struct Proxy {
    id: Id,
    pool: PgPool,
    /// Proxy server gRPC URI
    endpoint: Endpoint,
    /// gRPC servers
    servers: ProxyServerSet,
}

impl Proxy {
    // TODO(jck) more specific error
    pub fn new(id: Id, pool: PgPool, uri: Uri, tx: ProxyTxSet) -> Result<Self, anyhow::Error> {
        let endpoint = Endpoint::from(uri);

        // Set endpoint keep-alive to avoid connectivity issues in proxied deployments.
        let endpoint = endpoint
            .http2_keep_alive_interval(TEN_SECS)
            .tcp_keepalive(Some(TEN_SECS))
            .keep_alive_while_idle(true);

        // Setup certs.
        let config = server_config();
        let endpoint = if let Some(ca) = &config.proxy_grpc_ca {
            let ca = read_to_string(ca)?;
            let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
            endpoint.tls_config(tls)?
        } else {
            endpoint.tls_config(ClientTlsConfig::new().with_enabled_roots())?
        };

        // Instantiate gRPC servers.
        let servers = ProxyServerSet::new(pool.clone(), tx);

        Ok(Self {
            id,
            pool,
            endpoint,
            servers,
        })
    }

    pub(crate) async fn run(
        mut self,
        tx_set: ProxyTxSet,
        incompatible_components: Arc<RwLock<IncompatibleComponents>>,
        router: Arc<RwLock<ProxyRouter>>,
    ) -> Result<(), anyhow::Error> {
        loop {
            debug!("Connecting to proxy at {}", self.endpoint.uri());
            let interceptor = ClientVersionInterceptor::new(Version::parse(VERSION)?);
            let mut client =
                ProxyClient::with_interceptor(self.endpoint.connect_lazy(), interceptor);
            let (tx, rx) = mpsc::unbounded_channel();
            let response = match client.bidi(UnboundedReceiverStream::new(rx)).await {
                Ok(response) => response,
                Err(err) => {
                    match err.code() {
                        Code::FailedPrecondition => {
                            error!(
                                "Failed to connect to proxy @ {}, version check failed, retrying in \
                            10s: {err}",
                                self.endpoint.uri()
                            );
                            // TODO push event
                        }
                        err => {
                            error!(
                                "Failed to connect to proxy @ {}, retrying in 10s: {err}",
                                self.endpoint.uri()
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

            info!("Connected to proxy at {}", self.endpoint.uri());
            let mut resp_stream = response.into_inner();
            // TODO(jck) store router in the Proxy struct
            self.message_loop(tx, tx_set.wireguard.clone(), &mut resp_stream, &router)
                .await?;
        }
    }

    async fn message_loop(
        &mut self,
        tx: UnboundedSender<CoreResponse>,
        wireguard_tx: Sender<GatewayEvent>,
        resp_stream: &mut Streaming<CoreRequest>,
        router: &Arc<RwLock<ProxyRouter>>,
    ) -> Result<(), anyhow::Error> {
        let pool = self.pool.clone();
        'message: loop {
            match resp_stream.message().await {
                Ok(None) => {
                    info!("stream was closed by the sender");
                    break 'message;
                }
                Ok(Some(received)) => {
                    debug!("Received message from proxy; ID={}", received.id);
                    router.write().unwrap().register_request(&received, &tx);
                    let payload = match received.payload {
                        // rpc CodeMfaSetupStart return (CodeMfaSetupStartResponse)
                        Some(core_request::Payload::CodeMfaSetupStart(request)) => {
                            match self
                                .servers
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
                                .servers
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
                            match self.servers.client_mfa.validate_mfa_token(request).await {
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
                            match self.servers.enrollment.register_mobile_auth(request).await {
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
                                .servers
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
                                .servers
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
                                .servers
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
                                .servers
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
                                .servers
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
                                .servers
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
                                .servers
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
                                .servers
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
                                .servers
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
                                .servers
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
                                .servers
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
                            if !is_enterprise_enabled() {
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
                    if let Some(txs) = router.write().unwrap().route_response(&req) {
                        for tx in txs {
                            let _ = tx.send(req.clone());
                        }
                    } else {
                        let _ = tx.send(req);
                    };
                }
                Err(err) => {
                    error!("Disconnected from proxy at {}: {err}", self.endpoint.uri());
                    debug!("waiting 10s to re-establish the connection");
                    sleep(TEN_SECS).await;
                    break 'message;
                }
            }
        }

        Ok(())
    }
}

struct ProxyServerSet {
    enrollment: EnrollmentServer,
    password_reset: PasswordResetServer,
    client_mfa: ClientMfaServer,
    polling: PollingServer,
}

impl ProxyServerSet {
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
