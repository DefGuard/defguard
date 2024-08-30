use super::{
    proto::{DeviceConfig as ProtoDeviceConfig, DeviceConfigResponse},
    InstanceInfo,
};
use crate::db::{
    models::{device::WireguardNetworkDevice, enrollment::Token, wireguard::WireguardNetwork},
    DbPool, Device, Settings,
};
use ipnetwork::IpNetwork;
use tonic::Status;

use super::proto::{InstanceInfoRequest, InstanceInfoResponse};

pub(super) struct PollingServer {
    pool: DbPool,
}

impl PollingServer {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // check if token provided with request corresponds to a valid session
    async fn validate_session(&self, token: Option<&str>) -> Result<Token, Status> {
        info!("Start validating session. Token {token:?}");
        let Some(token) = token else {
            error!("Missing authorization header in request");
            return Err(Status::unauthenticated("Missing authorization header"));
        };
        debug!("Validating session token: {token}");

        let token = Token::find_by_id(&self.pool, token).await?;
        debug!("Verify is token valid {token:?}.");
        // TODO(jck): proper validation
        // if token.is_session_valid(server_config().client_auth_token_timeout.as_secs()) {
        //     info!("Session validated");
        //     Ok(token)
        // } else {
        //     error!("Session expired");
        //     Err(Status::unauthenticated("Session expired"))
        // }
        Ok(token)
    }

    /// Get all information needed
    /// to update instance information for desktop client
    pub async fn info(&self, request: InstanceInfoRequest) -> Result<InstanceInfoResponse, Status> {
        // TODO(jck): extract to get_config() function
        debug!("Getting network info for device: {:?}", request.pubkey);
        let token = self.validate_session(Some(&request.token)).await?;

        // get enrollment user
        let user = token.fetch_user(&self.pool).await?;

        Device::validate_pubkey(&request.pubkey).map_err(|_| {
            error!("Invalid pubkey {}", request.pubkey);
            Status::invalid_argument("invalid pubkey")
        })?;
        // Find existing device by public key
        let device = Device::find_by_pubkey(&self.pool, &request.pubkey)
            .await
            .map_err(|_| {
                error!("Failed to get device by its pubkey: {}", request.pubkey);
                Status::internal("unexpected error")
            })?;

        let settings = Settings::get_settings(&self.pool).await.map_err(|_| {
            error!("Failed to get settings");
            Status::internal("unexpected error")
        })?;

        let networks = WireguardNetwork::all(&self.pool).await.map_err(|err| {
            error!("Failed to fetch all networks: {err}");
            Status::internal(format!("unexpected error: {err}"))
        })?;

        let mut configs: Vec<ProtoDeviceConfig> = Vec::new();
        if let Some(device) = device {
            for network in networks {
                let (Some(device_id), Some(network_id)) = (device.id, network.id) else {
                    continue;
                };
                let wireguard_network_device =
                    WireguardNetworkDevice::find(&self.pool, device_id, network_id)
                        .await
                        .map_err(|err| {
                            error!("Failed to fetch wireguard network device for device {} and network {}: {err}", device_id, network_id);
                            Status::internal(format!("unexpected error: {err}"))
                        })?;
                if let Some(wireguard_network_device) = wireguard_network_device {
                    let allowed_ips = network
                        .allowed_ips
                        .iter()
                        .map(IpNetwork::to_string)
                        .collect::<Vec<String>>()
                        .join(",");
                    let config = ProtoDeviceConfig {
                        config: device.create_config(&network, &wireguard_network_device),
                        network_id,
                        network_name: network.name,
                        assigned_ip: wireguard_network_device.wireguard_ip.to_string(),
                        endpoint: format!("{}:{}", network.endpoint, network.port),
                        pubkey: network.pubkey,
                        allowed_ips,
                        dns: network.dns,
                        mfa_enabled: network.mfa_enabled,
                        keepalive_interval: network.keepalive_interval,
                    };
                    configs.push(config);
                }
            }

            info!("Device {} configs fetched", device.name);

            let device_config = DeviceConfigResponse {
                device: Some(device.into()),
                configs,
                instance: Some(InstanceInfo::new(settings, &user.username).into()),
            };

            let response = InstanceInfoResponse {
                device_config: Some(device_config),
                // TODO(jck): actually check enterprise status
                is_enterprise: true,
            };

            Ok(response)
        } else {
            Err(Status::internal("device not found error"))
        }
    }
}
