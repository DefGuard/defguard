use std::{
    collections::hash_map::HashMap,
    fs::read_to_string,
    time::{Duration, Instant},
};
#[cfg(any(feature = "wireguard", feature = "worker"))]
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
};

use chrono::{NaiveDateTime, Utc};
use openidconnect::{core::CoreAuthenticationFlow, AuthorizationCode, CsrfToken, Nonce, Scope};
use reqwest::Url;
use serde::Serialize;
#[cfg(feature = "worker")]
use sqlx::PgPool;
use thiserror::Error;
use tokio::{
    sync::{
        broadcast::Sender,
        mpsc::{self, UnboundedSender},
    },
    time::sleep,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_util::sync::CancellationToken;
use tonic::{
    transport::{Certificate, ClientTlsConfig, Endpoint, Identity, Server, ServerTlsConfig},
    Code, Status,
};
use utoipa::ToSchema;
use uuid::Uuid;

#[cfg(feature = "wireguard")]
use self::gateway::{gateway_service_server::GatewayServiceServer, GatewayServer};
use self::{
    auth::{auth_service_server::AuthServiceServer, AuthServer},
    desktop_client_mfa::ClientMfaServer,
    enrollment::EnrollmentServer,
    password_reset::PasswordResetServer,
    proto::core_response,
};
#[cfg(feature = "worker")]
use self::{
    interceptor::JwtInterceptor,
    worker::{worker_service_server::WorkerServiceServer, WorkerServer},
};
use crate::{
    auth::failed_login::FailedLoginMap,
    db::{
        models::enrollment::{Token, ENROLLMENT_TOKEN_TYPE},
        AppEvent, Id, Settings,
    },
    enterprise::{
        db::models::{enterprise_settings::EnterpriseSettings, openid_provider::OpenIdProvider},
        directory_sync::sync_user_groups_if_configured,
        grpc::polling::PollingServer,
        handlers::openid_login::{make_oidc_client, user_from_claims},
        is_enterprise_enabled,
    },
    handlers::mail::{send_gateway_disconnected_email, send_gateway_reconnected_email},
    mail::Mail,
    server_config,
};
#[cfg(feature = "worker")]
use crate::{auth::ClaimsType, db::GatewayEvent};

mod auth;
mod desktop_client_mfa;
pub mod enrollment;
#[cfg(feature = "wireguard")]
pub(crate) mod gateway;
#[cfg(any(feature = "wireguard", feature = "worker"))]
mod interceptor;
pub mod password_reset;
pub(crate) mod utils;
#[cfg(feature = "worker")]
pub mod worker;

pub(crate) mod proto {
    tonic::include_proto!("defguard.proxy");
}

use proto::{
    core_request, proxy_client::ProxyClient, AuthCallbackResponse, AuthInfoResponse, CoreError,
    CoreResponse,
};

// Helper struct used to handle gateway state
// gateways are grouped by network
type GatewayHostname = String;
#[derive(Debug)]
pub struct GatewayMap(HashMap<Id, HashMap<GatewayHostname, GatewayState>>);

#[derive(Error, Debug)]
pub enum GatewayMapError {
    #[error("Gateway {1} for network {0} not found")]
    NotFound(i64, GatewayHostname),
    #[error("Network {0} not found")]
    NetworkNotFound(i64),
    #[error("Gateway with UID {0} not found")]
    UidNotFound(Uuid),
    #[error("Cannot remove. Gateway with UID {0} is still active")]
    RemoveActive(Uuid),
    #[error("Config missing")]
    ConfigError,
    #[error("Failed to get current settings")]
    SettingsError,
}

impl GatewayMap {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    // add a new gateway to map
    // this method is meant to be called when a gateway requests a config
    // as a sort of "registration"
    pub fn add_gateway(
        &mut self,
        network_id: Id,
        network_name: &str,
        hostname: String,
        name: Option<String>,
        mail_tx: UnboundedSender<Mail>,
    ) {
        info!("Adding gateway {hostname} with to gateway map for network {network_id}",);
        let gateway_state = GatewayState::new(network_id, network_name, &hostname, name, mail_tx);

        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            network_gateway_map.entry(hostname).or_insert(gateway_state);
        } else {
            // no map for a given network exists yet
            let mut network_gateway_map = HashMap::new();
            network_gateway_map.insert(hostname, gateway_state);
            self.0.insert(network_id, network_gateway_map);
        }
    }

    // remove gateway from map
    pub fn remove_gateway(&mut self, network_id: Id, uid: Uuid) -> Result<(), GatewayMapError> {
        debug!("Removing gateway from network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            // find gateway by uuid
            let hostname = match network_gateway_map
                .iter()
                .find(|(_address, state)| state.uid == uid)
            {
                None => {
                    error!("Failed to find gateway with UID {uid}");
                    return Err(GatewayMapError::UidNotFound(uid));
                }
                Some((hostname, state)) => {
                    if state.connected {
                        error!("Cannot remove. Gateway with UID {uid} is still active");
                        return Err(GatewayMapError::RemoveActive(uid));
                    }
                    hostname.clone()
                }
            };
            // remove matching gateway
            network_gateway_map.remove(&hostname)
        } else {
            // no map for a given network exists yet
            error!("Network {network_id} not found in gateway map");
            return Err(GatewayMapError::NetworkNotFound(network_id));
        };
        info!("Gateway with UID {uid} removed from network {network_id}");
        Ok(())
    }

    // change gateway status to connected
    // we assume that the gateway is already present in hashmap
    pub fn connect_gateway(
        &mut self,
        network_id: Id,
        hostname: &str,
        pool: &PgPool,
    ) -> Result<(), GatewayMapError> {
        debug!("Connecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(hostname) {
                // check if a gateway is reconnecting to avoid sending notifications on initial
                // connection
                let is_reconnecting = state.disconnected_at.is_some();
                state.connected = true;
                state.disconnected_at = None;
                state.connected_at = Some(Utc::now().naive_utc());
                state.cancel_pending_disconnect_notification();
                if is_reconnecting {
                    state.handle_reconnect_notification(pool);
                }
                debug!(
                    "Gateway {hostname} found in gateway map, current state: {:?}",
                    state
                );
            } else {
                error!("Gateway {hostname} not found in gateway map for network {network_id}");
                return Err(GatewayMapError::NotFound(network_id, hostname.into()));
            }
        } else {
            // no map for a given network exists yet
            error!("Network {network_id} not found in gateway map");
            return Err(GatewayMapError::NetworkNotFound(network_id));
        };
        info!("Gateway {hostname} connected in network {network_id}");
        Ok(())
    }

    // change gateway status to disconnected
    pub fn disconnect_gateway(
        &mut self,
        network_id: Id,
        hostname: String,
        pool: &PgPool,
    ) -> Result<(), GatewayMapError> {
        debug!("Disconnecting gateway {hostname} in network {network_id}");
        if let Some(network_gateway_map) = self.0.get_mut(&network_id) {
            if let Some(state) = network_gateway_map.get_mut(&hostname) {
                state.connected = false;
                state.disconnected_at = Some(Utc::now().naive_utc());
                state.handle_disconnect_notification(pool);
                debug!("Gateway {hostname} found in gateway map, current state: {state:?}");
                info!("Gateway {hostname} disconnected in network {network_id}");
                return Ok(());
            };
        };
        let err = GatewayMapError::NotFound(network_id, hostname);
        error!("Gateway disconnect failed: {err}");
        Err(err)
    }

    // return `true` if at least one gateway in a given network is connected
    #[must_use]
    pub fn connected(&self, network_id: Id) -> bool {
        match self.0.get(&network_id) {
            Some(network_gateway_map) => network_gateway_map
                .values()
                .any(|gateway| gateway.connected),
            None => false,
        }
    }

    // return a list af aff statuses af all gateways in a given network
    #[must_use]
    pub fn get_network_gateway_status(&self, network_id: Id) -> Vec<GatewayState> {
        match self.0.get(&network_id) {
            Some(network_gateway_map) => network_gateway_map.clone().into_values().collect(),
            None => Vec::new(),
        }
    }

    // return gateway name
    #[must_use]
    pub fn get_network_gateway_name(&self, network_id: Id, hostname: &str) -> Option<String> {
        match self.0.get(&network_id) {
            Some(network_gateway_map) => {
                if let Some(state) = network_gateway_map.get(hostname) {
                    state.name.clone()
                } else {
                    None
                }
            }
            None => None,
        }
    }
}

impl Default for GatewayMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Clone, Debug, ToSchema)]
pub struct GatewayState {
    pub uid: Uuid,
    pub connected: bool,
    pub network_id: Id,
    pub network_name: String,
    pub name: Option<String>,
    pub hostname: String,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    #[serde(skip)]
    pub mail_tx: UnboundedSender<Mail>,
    #[serde(skip)]
    pub pending_notification_cancel_token: Option<CancellationToken>,
}

impl GatewayState {
    #[must_use]
    pub fn new<S: Into<String>>(
        network_id: Id,
        network_name: S,
        hostname: S,
        name: Option<String>,
        mail_tx: UnboundedSender<Mail>,
    ) -> Self {
        Self {
            uid: Uuid::new_v4(),
            connected: false,
            network_id,
            network_name: network_name.into(),
            name,
            hostname: hostname.into(),
            connected_at: None,
            disconnected_at: None,
            mail_tx,
            pending_notification_cancel_token: None,
        }
    }

    /// Checks if gateway disconnect notification should be sent.
    fn handle_disconnect_notification(&mut self, pool: &PgPool) {
        debug!("Checking if gateway disconnect notification needs to be sent");
        let settings = Settings::get_current_settings();
        if settings.gateway_disconnect_notifications_enabled {
            let delay = Duration::from_secs(
                60 * settings.gateway_disconnect_notifications_inactivity_threshold as u64,
            );
            self.send_disconnect_notification(pool, delay);
        };
    }

    /// Send gateway disconnected notification
    /// Sends notification only if last notification time is bigger than specified in config
    fn send_disconnect_notification(&mut self, pool: &PgPool, delay: Duration) {
        // Clone here because self doesn't live long enough
        let name = self.name.clone();
        let mail_tx = self.mail_tx.clone();
        let pool = pool.clone();
        let hostname = self.hostname.clone();
        let network_name = self.network_name.clone();

        debug!(
            "Scheduling gateway disconnect email notification for {hostname} to be sent in \
            {delay:?}"
        );
        // use cancellation token to abort sending if gateway reconnects during the delay
        // we should never need to cancel a previous token since that would've been done on reconnect
        assert!(self.pending_notification_cancel_token.is_none());
        let cancellation_token = CancellationToken::new();
        self.pending_notification_cancel_token = Some(cancellation_token.clone());

        // notification is not supposed to be sent immediately, so we instead schedule a
        // background task with a configured delay
        tokio::spawn(async move {
            tokio::select! {
                () = async {
                    sleep(delay).await;
                    debug!("Gateway disconnect notification delay has passed. \
                        Trying to send email...");
                    if let Err(e) = send_gateway_disconnected_email(name, network_name, &hostname,
                        &mail_tx, &pool)
                    .await
                    {
                        error!("Failed to send gateway disconnect notification: {e}");
                    } else {
                        info!("Gateway {hostname} disconnected. Email notification sent",);
                    }
                } => {
                    debug!("Scheduled gateway disconnect notification for {hostname} has been \
                        sent");
                },
                () = cancellation_token.cancelled() => {
                    info!("Scheduled gateway disconnect notification for {hostname} cancelled");
                }
            }
        });
    }

    /// Checks if gateway disconnect notification should be sent.
    fn handle_reconnect_notification(&mut self, pool: &PgPool) {
        debug!("Checking if gateway reconnect notification needs to be sent");
        let settings = Settings::get_current_settings();
        if settings.gateway_disconnect_notifications_reconnect_notification_enabled {
            self.send_reconnect_notification(pool);
        };
    }

    /// Send gateway disconnected notification
    /// Sends notification only if last notification time is bigger than specified in config
    fn send_reconnect_notification(&mut self, pool: &PgPool) {
        debug!("Sending gateway reconnect email notification");
        // Clone here because self doesn't live long enough
        let name = self.name.clone();
        let mail_tx = self.mail_tx.clone();
        let pool = pool.clone();
        let hostname = self.hostname.clone();
        let network_name = self.network_name.clone();
        tokio::spawn(async move {
            if let Err(e) =
                send_gateway_reconnected_email(name, network_name, &hostname, &mail_tx, &pool).await
            {
                error!("Failed to send gateway reconnect notification: {e}");
            } else {
                info!("Gateway {hostname} reconnected. Email notification sent",);
            }
        });
    }

    /// Cancels disconnect notification if one is scheduled to be sent
    fn cancel_pending_disconnect_notification(&mut self) {
        debug!(
            "Checking if there's a gateway disconnect notification for {} pending which needs \
            to be cancelled",
            self.hostname
        );
        if let Some(token) = &self.pending_notification_cancel_token {
            debug!(
                "Cancelling pending gateway disconnect notification for {}",
                self.hostname
            );
            token.cancel();
            self.pending_notification_cancel_token = None;
        }
    }
}

const TEN_SECS: Duration = Duration::from_secs(10);

impl From<Status> for CoreError {
    fn from(status: Status) -> Self {
        Self {
            status_code: status.code().into(),
            message: status.message().into(),
        }
    }
}

/// Bi-directional gRPC stream for comminication with Defguard proxy.
pub async fn run_grpc_bidi_stream(
    pool: PgPool,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
) -> Result<(), anyhow::Error> {
    let config = server_config();

    // TODO: merge the two
    let enrollment_server =
        EnrollmentServer::new(pool.clone(), wireguard_tx.clone(), mail_tx.clone());
    let password_reset_server = PasswordResetServer::new(pool.clone(), mail_tx.clone());
    let mut client_mfa_server = ClientMfaServer::new(pool.clone(), mail_tx, wireguard_tx.clone());
    let polling_server = PollingServer::new(pool.clone());

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
        let mut client = ProxyClient::new(endpoint.connect_lazy());
        let (tx, rx) = mpsc::unbounded_channel();
        let Ok(response) = client.bidi(UnboundedReceiverStream::new(rx)).await else {
            error!(
                "Failed to connect to proxy @ {}, retrying in 10s",
                endpoint.uri()
            );
            sleep(TEN_SECS).await;
            continue;
        };
        info!("Connected to proxy at {}", endpoint.uri());
        let mut resp_stream = response.into_inner();
        'message: loop {
            match resp_stream.message().await {
                Ok(None) => {
                    info!("stream was closed by the sender");
                    break 'message;
                }
                Ok(Some(received)) => {
                    info!("Received message from proxy.");
                    debug!("Received the following message from proxy: {received:?}");
                    let payload = match received.payload {
                        // rpc StartEnrollment (EnrollmentStartRequest) returns (EnrollmentStartResponse)
                        Some(core_request::Payload::EnrollmentStart(request)) => {
                            match enrollment_server.start_enrollment(request).await {
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
                            match enrollment_server
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
                            match enrollment_server
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
                            match enrollment_server.get_network_info(request).await {
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
                            match password_reset_server
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
                            match password_reset_server.start_password_reset(request).await {
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
                            match password_reset_server
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
                            match client_mfa_server.start_client_mfa_login(request).await {
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
                            match client_mfa_server.finish_client_mfa_login(request).await {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::ClientMfaFinish(response_payload))
                                }
                                Err(err) => {
                                    error!("client MFA start error {err}");
                                    Some(core_response::Payload::CoreError(err.into()))
                                }
                            }
                        }
                        // rpc LocationInfo (LocationInfoRequest) returns (LocationInfoResponse)
                        Some(core_request::Payload::InstanceInfo(request)) => {
                            match polling_server.info(request).await {
                                Ok(response_payload) => {
                                    Some(core_response::Payload::InstanceInfo(response_payload))
                                }
                                Err(err) => {
                                    if Code::FailedPrecondition == err.code() {
                                        // Ignore the case when we are not enterprise but the client is trying to fetch the instance config,
                                        // to avoid spamming the logs with misleading errors.

                                        debug!("A client tried to fetch the instance config, but we are not enterprise.");
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
                                        let (url, csrf_token, nonce) = client
                                            .authorize_url(
                                                CoreAuthenticationFlow::AuthorizationCode,
                                                CsrfToken::new_random,
                                                Nonce::new_random,
                                            )
                                            .add_scope(Scope::new("email".to_string()))
                                            .add_scope(Scope::new("profile".to_string()))
                                            .url();
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
                                        status_code: Code::Internal as i32,
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
                                        Ok(user) => {
                                            user.clear_unused_enrollment_tokens(&pool).await?;
                                            if let Err(err) = sync_user_groups_if_configured(
                                                &user,
                                                &pool,
                                                &wireguard_tx,
                                            )
                                            .await
                                            {
                                                error!(
                                                    "Failed to sync user groups for user {} with the directory while the user was logging in through an external provider: {err:?}",
                                                   user.username,
                                                );
                                            }
                                            debug!("Cleared unused tokens for {}.", user.username);
                                            debug!(
                                        "Creating a new desktop activation token for user {} as a result of proxy OpenID auth callback.",
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
                                            debug!("Saved desktop configuration token. Responding to proxy with the token.");

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
                                    error!("Proxy requested an OpenID authentication info for a callback URL ({}) that couldn't be parsed. Details: {err}", request.callback_url);
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
                    tx.send(req).unwrap();
                }
                Err(err) => {
                    error!("Disconnected from proxy at {}", endpoint.uri());
                    error!("stream error: {err}");
                    debug!("waiting 10s to re-establish the connection");
                    sleep(TEN_SECS).await;
                    break 'message;
                }
            }
        }
    }
}

/// Runs gRPC server with core services.
pub async fn run_grpc_server(
    worker_state: Arc<Mutex<WorkerState>>,
    pool: PgPool,
    gateway_state: Arc<Mutex<GatewayMap>>,
    wireguard_tx: Sender<GatewayEvent>,
    mail_tx: UnboundedSender<Mail>,
    grpc_cert: Option<String>,
    grpc_key: Option<String>,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
) -> Result<(), anyhow::Error> {
    // Build gRPC services
    let auth_service = AuthServiceServer::new(AuthServer::new(pool.clone(), failed_logins));
    #[cfg(feature = "worker")]
    let worker_service = WorkerServiceServer::with_interceptor(
        WorkerServer::new(pool.clone(), worker_state),
        JwtInterceptor::new(ClaimsType::YubiBridge),
    );
    #[cfg(feature = "wireguard")]
    let gateway_service = GatewayServiceServer::with_interceptor(
        GatewayServer::new(pool, gateway_state, wireguard_tx, mail_tx),
        JwtInterceptor::new(ClaimsType::Gateway),
    );

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<AuthServiceServer<AuthServer>>()
        .await;

    // Run gRPC server
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), server_config().grpc_port);
    debug!("Starting gRPC services");
    let builder = if let (Some(cert), Some(key)) = (grpc_cert, grpc_key) {
        let identity = Identity::from_pem(cert, key);
        Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
    } else {
        Server::builder()
    };
    let router = builder
        .http2_keepalive_interval(Some(TEN_SECS))
        .tcp_keepalive(Some(TEN_SECS))
        .add_service(health_service)
        .add_service(auth_service);
    #[cfg(feature = "wireguard")]
    let router = router.add_service(gateway_service);
    #[cfg(feature = "worker")]
    let router = router.add_service(worker_service);
    router.serve(addr).await?;
    info!("gRPC server started on {addr}");
    Ok(())
}

#[cfg(feature = "worker")]
pub struct Job {
    id: u32,
    first_name: String,
    last_name: String,
    email: String,
    username: String,
}

#[cfg(feature = "worker")]
#[derive(Serialize)]
pub struct JobResponse {
    pub success: bool,
    pub serial: String,
    pub error: String,
    #[serde(skip)]
    pub username: String,
}

#[cfg(feature = "worker")]
pub struct WorkerInfo {
    last_seen: Instant,
    ip: IpAddr,
    jobs: Vec<Job>,
}

#[cfg(feature = "worker")]
pub struct WorkerState {
    current_job_id: u32,
    workers: HashMap<String, WorkerInfo>,
    job_status: HashMap<u32, JobResponse>,
    webhook_tx: UnboundedSender<AppEvent>,
}

#[cfg(feature = "worker")]
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
}

impl InstanceInfo {
    pub fn new<S: Into<String>>(
        settings: Settings,
        username: S,
        enterprise_settings: &EnterpriseSettings,
    ) -> Self {
        let config = server_config();
        InstanceInfo {
            id: settings.uuid,
            name: settings.instance_name,
            url: config.url.clone(),
            proxy_url: config.enrollment_url.clone(),
            username: username.into(),
            disable_all_traffic: enterprise_settings.disable_all_traffic,
            enterprise_enabled: is_enterprise_enabled(),
        }
    }
}

impl From<InstanceInfo> for proto::InstanceInfo {
    fn from(instance: InstanceInfo) -> Self {
        Self {
            name: instance.name,
            id: instance.id.to_string(),
            url: instance.url.to_string(),
            proxy_url: instance.proxy_url.to_string(),
            username: instance.username,
            disable_all_traffic: instance.disable_all_traffic,
            enterprise_enabled: instance.enterprise_enabled,
        }
    }
}
