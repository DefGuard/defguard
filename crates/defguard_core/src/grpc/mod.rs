use std::{
    collections::hash_map::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use defguard_common::{
    auth::claims::ClaimsType,
    config::server_config,
    db::{
        Id,
        models::{
            Settings, User, WireguardNetwork, vpn_client_session::VpnClientMfaMethod,
            wireguard::ServiceLocationMode,
        },
    },
    types::UrlParseError,
};
use reqwest::Url;
use serde::Serialize;
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    db::AppEvent,
    enterprise::{
        LicenseFeature,
        db::models::{
            enterprise_settings::{
                ClientTrafficPolicy, EnterpriseSettings, resolve_client_traffic_policy,
            },
            group_client_traffic_policy::GroupClientTrafficPolicy,
            openid_provider::OpenIdProvider,
        },
        has_enterprise_access, is_business_license_active,
    },
    grpc::{interceptor::JwtInterceptor, worker::WorkerServer},
};

pub mod client_version;
pub mod interceptor;
pub mod proxy;
pub mod utils;
pub mod worker;

pub mod proto {
    pub mod enterprise {
        pub mod license {
            tonic::include_proto!("enterprise.license");
        }
    }
}

use defguard_proto::{
    client_types::{MfaMethod, MfaUserState},
    worker::worker_service_server::WorkerServiceServer,
};
use tonic::transport::{Identity, Server, ServerTlsConfig, server::Router};

// gRPC header for passing auth token from clients
pub static AUTHORIZATION_HEADER: &str = "authorization";

// gRPC header for passing hostname from clients
pub static HOSTNAME_HEADER: &str = "hostname";
const TEN_SECS: Duration = Duration::from_secs(10);

/// Runs gRPC server with core services.
#[instrument(skip_all)]
pub async fn run_grpc_server(
    worker_state: Arc<Mutex<WorkerState>>,
    pool: PgPool,
    grpc_cert: Option<String>,
    grpc_key: Option<String>,
) -> Result<(), anyhow::Error> {
    // Build gRPC services
    let server = if let (Some(cert), Some(key)) = (grpc_cert, grpc_key) {
        let identity = Identity::from_pem(cert, key);
        Server::builder().tls_config(ServerTlsConfig::new().identity(identity))?
    } else {
        Server::builder()
    };

    let router = build_grpc_service_router(server, pool, worker_state).await?;

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
) -> Result<Router, anyhow::Error> {
    let worker_service = WorkerServiceServer::with_interceptor(
        WorkerServer::new(pool.clone(), worker_state),
        JwtInterceptor::new(ClaimsType::YubiBridge),
    );

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<WorkerServiceServer<WorkerServer>>()
        .await;
    health_reporter
        .set_serving::<WorkerServiceServer<WorkerServer>>()
        .await;

    let router = server
        .http2_keepalive_interval(Some(TEN_SECS))
        .tcp_keepalive(Some(TEN_SECS))
        .add_service(health_service)
        .add_service(worker_service);

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
    client_traffic_policy: ClientTrafficPolicy,
    enterprise_enabled: bool,
    openid_display_name: Option<String>,
    disable_tunnels: bool,
    mfa_user_state: Vec<VpnClientMfaMethod>,
}

#[derive(Debug, thiserror::Error)]
/// Errors that can occur while building client instance information.
pub enum InstanceInfoBuildError {
    #[error("failed to load enterprise settings: {0}")]
    Database(#[from] sqlx::Error),
    #[error("failed to parse instance URL: {0}")]
    UrlParse(#[from] UrlParseError),
}

impl InstanceInfo {
    /// Builds client instance information with the effective user traffic policy.
    pub async fn build(
        pool: &PgPool,
        settings: &Settings,
        user: &User<Id>,
        openid_provider: Option<OpenIdProvider<Id>>,
        device_id: Option<Id>,
    ) -> Result<Self, InstanceInfoBuildError> {
        let enterprise_settings = EnterpriseSettings::get(pool).await?;
        let smtp_configured = settings.smtp_configured();
        let oidc_configured = is_business_license_active() && openid_provider.is_some();
        let mut mfa_user_state = Vec::with_capacity(5);
        for method in [
            VpnClientMfaMethod::Totp,
            VpnClientMfaMethod::Email,
            VpnClientMfaMethod::Oidc,
            VpnClientMfaMethod::Biometric,
            VpnClientMfaMethod::MobileApprove,
        ] {
            if method
                .is_configured(pool, user, device_id, smtp_configured, oidc_configured)
                .await?
            {
                mfa_user_state.push(method);
            }
        }
        let client_traffic_policy = if is_business_license_active() {
            let group_policies = GroupClientTrafficPolicy::find_by_user_id(pool, user.id)
                .await?
                .into_iter()
                .map(|policy| policy.client_traffic_policy);
            resolve_client_traffic_policy(enterprise_settings.client_traffic_policy, group_policies)
        } else {
            enterprise_settings.client_traffic_policy
        };
        let openid_display_name = openid_provider
            .as_ref()
            .map(|provider| provider.display_name.clone())
            .unwrap_or_default();
        let url = Settings::url()?;
        let proxy_url = settings.proxy_public_url()?;
        Ok(Self {
            id: settings.uuid,
            name: settings.instance_name.clone(),
            url,
            proxy_url,
            username: user.username.clone(),
            client_traffic_policy,
            enterprise_enabled: is_business_license_active(),
            openid_display_name,
            disable_tunnels: enterprise_settings.disable_tunnels,
            mfa_user_state,
        })
    }
}

impl From<InstanceInfo> for defguard_proto::client_types::InstanceInfo {
    fn from(instance: InstanceInfo) -> Self {
        Self {
            name: instance.name,
            id: instance.id.to_string(),
            url: instance.url.to_string(),
            proxy_url: instance.proxy_url.to_string(),
            username: instance.username,
            // Ensure backwards compatibility.
            #[allow(deprecated)]
            disable_all_traffic: instance.client_traffic_policy
                == ClientTrafficPolicy::DisableAllTraffic,
            client_traffic_policy: Some(instance.client_traffic_policy as i32),
            enterprise_enabled: instance.enterprise_enabled,
            openid_display_name: instance.openid_display_name,
            disable_tunnels: Some(instance.disable_tunnels),
            mfa_user_state: Some(MfaUserState {
                configured_methods: instance
                    .mfa_user_state
                    .into_iter()
                    .map(|method| MfaMethod::from(method) as i32)
                    .collect(),
            }),
        }
    }
}

pub use defguard_common::gateway_event::{
    GatewayCommand, send_gateway_command, send_multiple_gateway_commands,
};

/// If this location is marked as a service location, checks if all requirements are met for it to
/// function:
/// - Enterprise is enabled
#[must_use]
pub fn should_prevent_service_location_usage(location: &WireguardNetwork<Id>) -> bool {
    location.service_location_mode != ServiceLocationMode::Disabled
        && !has_enterprise_access(Some(LicenseFeature::ServiceLocations))
}
