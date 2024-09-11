use tonic::Status;

use crate::{
    db::{models::polling_token::PollingToken, DbPool, Device, User},
    enterprise::license::{get_cached_license, validate_license},
    grpc::{
        proto::{InstanceInfoRequest, InstanceInfoResponse},
        utils::{build_device_config_response, build_instance_config_response},
    },
};

pub struct PollingServer {
    pool: DbPool,
}

impl PollingServer {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Checks validity of polling session
    async fn validate_session(&self, token: &str) -> Result<PollingToken, Status> {
        debug!("Validating polling token. Token: {token}");

        // Polling service is enterprise-only, check the lincense
        if validate_license(get_cached_license().as_ref()).is_err() {
            debug!("No valid license, denying instance polling info");
            return Err(Status::permission_denied("no valid license"));
        }

        // Validate the token
        let Some(token) = PollingToken::find(&self.pool, token).await.map_err(|err| {
            error!("Failed to retrieve token: {err}");
            Status::internal("failed to retrieve token")
        })?
        else {
            error!("Invalid token {token:?}");
            return Err(Status::permission_denied("invalid token"));
        };

        // Polling tokens are valid indefinitely
        info!("Token validation successful {token:?}.");
        Ok(token)
    }

    /// Prepares instance info for polling requests. Enterprise only.
    pub async fn info(&self, request: InstanceInfoRequest) -> Result<InstanceInfoResponse, Status> {
        trace!("Polling info start");
        let token = self.validate_session(&request.token).await?;
        let Some(device) = Device::find_by_id(&self.pool, token.device_id)
            .await
            .map_err(|err| {
                error!("Failed to retrieve device id {}: {err}", token.device_id);
                Status::internal("failed to retrieve device")
            })?
        else {
            error!("Device id {} not found", token.device_id);
            return Err(Status::internal("device not found"));
        };
        debug!("Polling info for device: {}", device.wireguard_pubkey);

        // Ensure user is active
        let device_id = device.id.expect("missing device id");
        let Some(user) = User::find_by_device_id(&self.pool, device_id)
            .await
            .map_err(|err| {
                error!("Failed to retrieve user for device id {device_id}: {err}");
                Status::internal("failed to retrieve user")
            })?
        else {
            error!("User for device id {device_id} not found");
            return Err(Status::internal("user not found"));
        };
        if !user.is_active {
            warn!(
                "Denying polling info for inactive user {}({:?})",
                user.username, user.id
            );
            return Err(Status::permission_denied("user inactive"));
        }

        // Build & return polling info
        let device_config =
            build_device_config_response(&self.pool, &device.wireguard_pubkey).await?;
        let instance_config = build_instance_config_response(&self.pool).await?;
        Ok(InstanceInfoResponse {
            device_config: Some(device_config),
            instance_config: Some(instance_config),
        })
    }
}
