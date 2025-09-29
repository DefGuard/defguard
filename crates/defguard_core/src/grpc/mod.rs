use std::{
    collections::hash_map::HashMap,
    fs::read_to_string,
    time::{Duration, Instant},
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex, RwLock},
};

use axum::http::Uri;
use defguard_common::{
    VERSION,
    auth::claims::ClaimsType,
    db::{Id, models::Settings},
};
use defguard_mail::Mail;
use defguard_version::server::DefguardVersionLayer;
use defguard_version::{
    ComponentInfo, DefguardComponent, Version, client::ClientVersionInterceptor,
    get_tracing_variables,
};
use openidconnect::{AuthorizationCode, Nonce, Scope, core::CoreAuthenticationFlow};
use reqwest::Url;
use serde::Serialize;
use sqlx::PgPool;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{
    Code, Streaming,
    transport::{
        Certificate, ClientTlsConfig, Endpoint, Identity, Server, ServerTlsConfig, server::Router,
    },
};
use tower::ServiceBuilder;

use self::gateway::GatewayServer;
use self::{
    auth::AuthServer, client_mfa::ClientMfaServer, enrollment::EnrollmentServer,
    password_reset::PasswordResetServer,
};
use self::{interceptor::JwtInterceptor, worker::WorkerServer};
use crate::db::GatewayEvent;
pub use crate::version::MIN_GATEWAY_VERSION;
use crate::{
    auth::failed_login::FailedLoginMap,
    db::{
        AppEvent,
        models::enrollment::{ENROLLMENT_TOKEN_TYPE, Token},
    },
    enterprise::{
        db::models::{enterprise_settings::EnterpriseSettings, openid_provider::OpenIdProvider},
        directory_sync::sync_user_groups_if_configured,
        grpc::polling::PollingServer,
        handlers::openid_login::{
            SELECT_ACCOUNT_SUPPORTED_PROVIDERS, build_state, make_oidc_client, user_from_claims,
        },
        is_enterprise_enabled,
        ldap::utils::ldap_update_user_state,
    },
    events::{BidiStreamEvent, GrpcEvent},
    grpc::gateway::{client_state::ClientMap, map::GatewayMap},
    server_config,
    version::{IncompatibleComponents, IncompatibleProxyData, is_proxy_version_supported},
};

static VERSION_ZERO: Version = Version::new(0, 0, 0);

mod auth;
pub(crate) mod client_mfa;
pub mod enrollment;
pub mod gateway;
mod interceptor;
pub mod password_reset;
pub(crate) mod utils;
pub mod worker;

pub mod proto {
    pub mod enterprise {
        pub mod license {
            tonic::include_proto!("enterprise.license");
        }
    }
}

use defguard_proto::{
    auth::auth_service_server::AuthServiceServer,
    gateway::gateway_service_server::GatewayServiceServer,
    proxy::{
        AuthCallbackResponse, AuthInfoResponse, CoreError, CoreRequest, CoreResponse, core_request,
        core_response, proxy_client::ProxyClient,
    },
    worker::worker_service_server::WorkerServiceServer,
};

// gRPC header for passing auth token from clients
pub static AUTHORIZATION_HEADER: &str = "authorization";

// gRPC header for passing hostname from clients
pub static HOSTNAME_HEADER: &str = "hostname";

const TEN_SECS: Duration = Duration::from_secs(10);

struct ProxyMessageLoopContext<'a> {
    pool: PgPool,
    tx: UnboundedSender<CoreResponse>,
    wireguard_tx: Sender<GatewayEvent>,
    resp_stream: &'a mut Streaming<CoreRequest>,
    enrollment_server: &'a mut EnrollmentServer,
    password_reset_server: &'a mut PasswordResetServer,
    client_mfa_server: &'a mut ClientMfaServer,
    polling_server: &'a mut PollingServer,
    endpoint_uri: &'a Uri,
}

#[instrument(skip_all)]
async fn handle_proxy_message_loop(
    context: ProxyMessageLoopContext<'_>,
) -> Result<(), anyhow::Error> {
    let pool = context.pool.clone();
    'message: loop {
        match context.resp_stream.message().await {
            Ok(None) => {
                info!("stream was closed by the sender");
                break 'message;
            }
            Ok(Some(received)) => {
                debug!("Received message from proxy; ID={}", received.id);
                let payload = match received.payload {
                    // rpc CodeMfaSetupStart return (CodeMfaSetupStartResponse)
                    Some(core_request::Payload::CodeMfaSetupStart(request)) => {
                        match context
                            .enrollment_server
                            .register_code_mfa_start(request)
                            .await
                        {
                            Ok(response) => {
                                Some(core_response::Payload::CodeMfaSetupStartResponse(response))
                            }
                            Err(err) => {
                                error!("Register mfa start error {err}");
                                Some(core_response::Payload::CoreError(err.into()))
                            }
                        }
                    }
                    // rpc CodeMfaSetupFinish return (CodeMfaSetupFinishResponse)
                    Some(core_request::Payload::CodeMfaSetupFinish(request)) => {
                        match context
                            .enrollment_server
                            .register_code_mfa_finish(request)
                            .await
                        {
                            Ok(response) => {
                                Some(core_response::Payload::CodeMfaSetupFinishResponse(response))
                            }
                            Err(err) => {
                                error!("Register MFA finish error {err}");
                                Some(core_response::Payload::CoreError(err.into()))
                            }
                        }
                    }
                    // rpc ClientMfaTokenValidation return (ClientMfaTokenValidationResponse)
                    Some(core_request::Payload::ClientMfaTokenValidation(request)) => {
                        match context.client_mfa_server.validate_mfa_token(request).await {
                            Ok(response_payload) => Some(
                                core_response::Payload::ClientMfaTokenValidation(response_payload),
                            ),
                            Err(err) => {
                                error!("Client MFA validate token error {err}");
                                Some(core_response::Payload::CoreError(err.into()))
                            }
                        }
                    }
                    // rpc RegisterMobileAuth (RegisterMobileAuthRequest) return (google.protobuf.Empty)
                    Some(core_request::Payload::RegisterMobileAuth(request)) => {
                        match context
                            .enrollment_server
                            .register_mobile_auth(request)
                            .await
                        {
                            Ok(()) => Some(core_response::Payload::Empty(())),
                            Err(err) => {
                                error!("Register mobile auth error {err}");
                                Some(core_response::Payload::CoreError(err.into()))
                            }
                        }
                    }
                    // rpc StartEnrollment (EnrollmentStartRequest) returns (EnrollmentStartResponse)
                    Some(core_request::Payload::EnrollmentStart(request)) => {
                        match context
                            .enrollment_server
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
                        match context
                            .enrollment_server
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
                        match context
                            .enrollment_server
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
                        match context.enrollment_server.get_network_info(request).await {
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
                        match context
                            .password_reset_server
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
                        match context
                            .password_reset_server
                            .start_password_reset(request, received.device_info)
                            .await
                        {
                            Ok(response_payload) => {
                                Some(core_response::Payload::PasswordResetStart(response_payload))
                            }
                            Err(err) => {
                                error!("password reset start error {err}");
                                Some(core_response::Payload::CoreError(err.into()))
                            }
                        }
                    }
                    // rpc ResetPassword (PasswordResetRequest) returns (google.protobuf.Empty)
                    Some(core_request::Payload::PasswordReset(request)) => {
                        match context
                            .password_reset_server
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
                        match context
                            .client_mfa_server
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
                        match context
                            .client_mfa_server
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
                        match context
                            .client_mfa_server
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
                        match context.polling_server.info(request).await {
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
                                if let Ok((_client_id, client)) =
                                    make_oidc_client(redirect_url, &provider).await
                                {
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
                                        authorize_url_builder = authorize_url_builder.add_prompt(
                                            openidconnect::core::CoreAuthPrompt::SelectAccount,
                                        );
                                    }
                                    let (url, csrf_token, nonce) = authorize_url_builder.url();

                                    Some(core_response::Payload::AuthInfo(AuthInfoResponse {
                                        url: url.into(),
                                        csrf_token: csrf_token.secret().to_owned(),
                                        nonce: nonce.secret().to_owned(),
                                        button_display_name: provider.display_name,
                                    }))
                                } else {
                                    Some(core_response::Payload::CoreError(CoreError {
                                        status_code: Code::Internal as i32,
                                        message: "failed to build OIDC client".into(),
                                    }))
                                }
                            } else {
                                error!("Failed to get current OpenID provider");
                                Some(core_response::Payload::CoreError(CoreError {
                                    status_code: Code::NotFound as i32,
                                    message: "failed to get current OpenID provider".into(),
                                }))
                            }
                        } else {
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
                                        user.clear_unused_enrollment_tokens(&pool).await?;
                                        if let Err(err) = sync_user_groups_if_configured(
                                            &user,
                                            &pool,
                                            &context.wireguard_tx,
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
                context.tx.send(req).unwrap();
            }
            Err(err) => {
                error!("Disconnected from proxy at {}: {err}", context.endpoint_uri);
                debug!("waiting 10s to re-establish the connection");
                sleep(TEN_SECS).await;
                break 'message;
            }
        }
    }

    Ok(())
}

/// Bi-directional gRPC stream for communication with Defguard Proxy.
#[instrument(skip_all)]
pub async fn run_grpc_bidi_stream(
    pool: PgPool,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
) -> Result<(), anyhow::Error> {
    let config = server_config();

    // TODO: merge the two
    let mut enrollment_server = EnrollmentServer::new(
        pool.clone(),
        wireguard_tx.clone(),
        mail_tx.clone(),
        bidi_event_tx.clone(),
    );
    let mut password_reset_server =
        PasswordResetServer::new(pool.clone(), mail_tx.clone(), bidi_event_tx.clone());
    let mut client_mfa_server =
        ClientMfaServer::new(pool.clone(), mail_tx, wireguard_tx.clone(), bidi_event_tx);
    let mut polling_server = PollingServer::new(pool.clone());

    let endpoint = Endpoint::from_shared(config.proxy_url.as_deref().unwrap())?;
    let endpoint = endpoint
        .http2_keep_alive_interval(TEN_SECS)
        .tcp_keepalive(Some(TEN_SECS))
        .keep_alive_while_idle(true);
    let endpoint = if let Some(ca) = &config.proxy_grpc_ca {
        let ca = read_to_string(ca)?;
        let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(ca));
        endpoint.tls_config(tls)?
    } else {
        endpoint.tls_config(ClientTlsConfig::new().with_enabled_roots())?
    };

    loop {
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
        handle_proxy_message_loop(ProxyMessageLoopContext {
            pool: pool.clone(),
            tx,
            wireguard_tx: wireguard_tx.clone(),
            resp_stream: &mut resp_stream,
            enrollment_server: &mut enrollment_server,
            password_reset_server: &mut password_reset_server,
            client_mfa_server: &mut client_mfa_server,
            polling_server: &mut polling_server,
            endpoint_uri: endpoint.uri(),
        })
        .await?;
    }
}

/// Runs gRPC server with core services.
#[instrument(skip_all)]
pub async fn run_grpc_server(
    worker_state: Arc<Mutex<WorkerState>>,
    pool: PgPool,
    gateway_state: Arc<Mutex<GatewayMap>>,
    client_state: Arc<Mutex<ClientMap>>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    grpc_cert: Option<String>,
    grpc_key: Option<String>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
    grpc_event_tx: UnboundedSender<GrpcEvent>,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
) -> Result<(), anyhow::Error> {
    // Build gRPC services
    let server = if let (Some(cert), Some(key)) = (grpc_cert, grpc_key) {
        let identity = Identity::from_pem(cert, key);
        Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
    } else {
        Server::builder()
    };

    let router = build_grpc_service_router(
        server,
        pool,
        worker_state,
        gateway_state,
        client_state,
        wireguard_tx,
        mail_tx,
        failed_logins,
        grpc_event_tx,
        incompatible_components,
    )
    .await?;

    // Run gRPC server
    let addr = SocketAddr::new(
        server_config()
            .grpc_bind_address
            .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
        server_config().grpc_port,
    );
    debug!("Starting gRPC services");
    router.serve(addr).await?;
    info!("gRPC server started on {addr}");
    Ok(())
}

pub async fn build_grpc_service_router(
    server: Server,
    pool: PgPool,
    worker_state: Arc<Mutex<WorkerState>>,
    gateway_state: Arc<Mutex<GatewayMap>>,
    client_state: Arc<Mutex<ClientMap>>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
    grpc_event_tx: UnboundedSender<GrpcEvent>,
    incompatible_components: Arc<RwLock<IncompatibleComponents>>,
) -> Result<Router, anyhow::Error> {
    let auth_service = AuthServiceServer::new(AuthServer::new(pool.clone(), failed_logins));

    let worker_service = WorkerServiceServer::with_interceptor(
        WorkerServer::new(pool.clone(), worker_state),
        JwtInterceptor::new(ClaimsType::YubiBridge),
    );

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<AuthServiceServer<AuthServer>>()
        .await;

    let router = server
        .http2_keepalive_interval(Some(TEN_SECS))
        .tcp_keepalive(Some(TEN_SECS))
        .add_service(health_service)
        .add_service(auth_service);

    let router = {
        use crate::version::GatewayVersionInterceptor;

        let gateway_service = GatewayServiceServer::new(GatewayServer::new(
            pool,
            gateway_state,
            client_state,
            wireguard_tx,
            mail_tx,
            grpc_event_tx,
        ));

        let own_version = Version::parse(VERSION)?;
        router.add_service(
            ServiceBuilder::new()
                .layer(tonic::service::InterceptorLayer::new(JwtInterceptor::new(
                    ClaimsType::Gateway,
                )))
                .layer(tonic::service::InterceptorLayer::new(
                    GatewayVersionInterceptor::new(MIN_GATEWAY_VERSION, incompatible_components),
                ))
                .layer(DefguardVersionLayer::new(own_version))
                .service(gateway_service),
        )
    };

    let router = router.add_service(worker_service);

    Ok(router)
}

pub struct Job {
    id: u32,
    first_name: String,
    last_name: String,
    email: String,
    username: String,
}

#[derive(Serialize)]
pub struct JobResponse {
    pub success: bool,
    pub serial: String,
    pub error: String,
    #[serde(skip)]
    pub username: String,
}

pub struct WorkerInfo {
    last_seen: Instant,
    ip: IpAddr,
    jobs: Vec<Job>,
}

pub struct WorkerState {
    current_job_id: u32,
    workers: HashMap<String, WorkerInfo>,
    job_status: HashMap<u32, JobResponse>,
    webhook_tx: UnboundedSender<AppEvent>,
}

#[derive(Deserialize, Serialize)]
pub struct WorkerDetail {
    id: String,
    ip: IpAddr,
    connected: bool,
}

#[derive(Debug)]
pub struct InstanceInfo {
    id: uuid::Uuid,
    name: String,
    url: Url,
    proxy_url: Url,
    username: String,
    disable_all_traffic: bool,
    enterprise_enabled: bool,
    openid_display_name: Option<String>,
}

impl InstanceInfo {
    pub fn new<S: Into<String>>(
        settings: Settings,
        username: S,
        enterprise_settings: &EnterpriseSettings,
        openid_provider: Option<OpenIdProvider<Id>>,
    ) -> Self {
        let config = server_config();
        let openid_display_name = openid_provider
            .as_ref()
            .map(|provider| provider.display_name.clone())
            .unwrap_or_default();
        InstanceInfo {
            id: settings.uuid,
            name: settings.instance_name,
            url: config.url.clone(),
            proxy_url: config.enrollment_url.clone(),
            username: username.into(),
            disable_all_traffic: enterprise_settings.disable_all_traffic,
            enterprise_enabled: is_enterprise_enabled(),
            openid_display_name,
        }
    }
}

impl From<InstanceInfo> for defguard_proto::proxy::InstanceInfo {
    fn from(instance: InstanceInfo) -> Self {
        Self {
            name: instance.name,
            id: instance.id.to_string(),
            url: instance.url.to_string(),
            proxy_url: instance.proxy_url.to_string(),
            username: instance.username,
            disable_all_traffic: instance.disable_all_traffic,
            enterprise_enabled: instance.enterprise_enabled,
            openid_display_name: instance.openid_display_name,
        }
    }
}
