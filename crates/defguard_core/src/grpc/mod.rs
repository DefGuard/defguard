use std::{
    collections::hash_map::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use defguard_common::{
    auth::claims::ClaimsType,
    db::{
        Id,
        models::{
            Device, Settings, WireguardNetwork,
            device::{DeviceInfo, WireguardNetworkDevice},
            wireguard::ServiceLocationMode,
        },
    },
    types::UrlParseError,
};
use reqwest::Url;
use serde::Serialize;
use sqlx::PgPool;
use tokio::sync::{broadcast::Sender, mpsc::UnboundedSender};
use tonic::transport::{Identity, Server, ServerTlsConfig, server::Router};

use self::interceptor::JwtInterceptor;
use crate::{
    auth::failed_login::FailedLoginMap,
    db::AppEvent,
    enterprise::{
        db::models::{
            enterprise_settings::{ClientTrafficPolicy, EnterpriseSettings},
            openid_provider::OpenIdProvider,
        },
        is_business_license_active, is_enterprise_license_active,
    },
    server_config,
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
    auth::auth_service_server::AuthServiceServer, enterprise::firewall::FirewallConfig,
    gateway::Peer, worker::worker_service_server::WorkerServiceServer,
};

// gRPC header for passing auth token from clients
pub static AUTHORIZATION_HEADER: &str = "authorization";

// gRPC header for passing hostname from clients
pub static HOSTNAME_HEADER: &str = "hostname";

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
}

impl InstanceInfo {
    pub fn new<S: Into<String>>(
        settings: Settings,
        username: S,
        enterprise_settings: &EnterpriseSettings,
        openid_provider: Option<OpenIdProvider<Id>>,
    ) -> Result<Self, UrlParseError> {
        let openid_display_name = openid_provider
            .as_ref()
            .map(|provider| provider.display_name.clone())
            .unwrap_or_default();
        let url = Settings::url()?;
        Ok(Self {
            id: settings.uuid,
            name: settings.instance_name.clone(),
            url,
            proxy_url: settings.proxy_public_url()?,
            username: username.into(),
            client_traffic_policy: enterprise_settings.client_traffic_policy,
            enterprise_enabled: is_business_license_active(),
            openid_display_name,
        })
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
            // Ensure backwards compatibility.
            #[allow(deprecated)]
            disable_all_traffic: instance.client_traffic_policy
                == ClientTrafficPolicy::DisableAllTraffic,
            client_traffic_policy: Some(instance.client_traffic_policy as i32),
            enterprise_enabled: instance.enterprise_enabled,
            openid_display_name: instance.openid_display_name,
        }
    }
}

// TODO: move this to common crate
#[derive(Clone, Debug)]
pub enum GatewayEvent {
    NetworkCreated(Id, WireguardNetwork<Id>),
    NetworkModified(Id, WireguardNetwork<Id>, Vec<Peer>, Option<FirewallConfig>),
    NetworkDeleted(Id, String),
    DeviceCreated(DeviceInfo),
    DeviceModified(DeviceInfo),
    DeviceDeleted(DeviceInfo),
    FirewallConfigChanged(Id, FirewallConfig),
    FirewallDisabled(Id),
    MfaSessionAuthorized(Id, Device<Id>, WireguardNetworkDevice),
    MfaSessionDisconnected(Id, Device<Id>),
}

/// Sends given `GatewayEvent` to be handled by gateway GRPC server
///
/// If you want to use it inside the API context, use [`crate::AppState::send_wireguard_event`] instead
pub fn send_wireguard_event(event: GatewayEvent, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending the following WireGuard event to Defguard Gateway: {event:?}");
    if let Err(err) = wg_tx.send(event) {
        error!("Error sending WireGuard event {err}");
    }
}

/// Sends multiple events to be handled by gateway gRPC server.
///
/// If you want to use it inside the API context, use [`crate::AppState::send_multiple_wireguard_events`] instead
pub fn send_multiple_wireguard_events(events: Vec<GatewayEvent>, wg_tx: &Sender<GatewayEvent>) {
    debug!("Sending {} WireGuard events", events.len());
    for event in events {
        send_wireguard_event(event, wg_tx);
    }
}

/// If this location is marked as a service location, checks if all requirements are met for it to
/// function:
/// - Enterprise is enabled
#[must_use]
pub fn should_prevent_service_location_usage(location: &WireguardNetwork<Id>) -> bool {
    location.service_location_mode != ServiceLocationMode::Disabled
        && !is_enterprise_license_active()
}
