//! Core-client operations forwarded by Edge. Callers authenticate with the polling token, and each
//! operation applies its own license and device state guards.

mod device_verification;

use defguard_common::db::{
    Id,
    models::{Device, DeviceState, polling_token::PollingToken, user::User},
};
use defguard_proto::proxy::{CoreError, CoreErrorCode};
pub use device_verification::status_message;
use sqlx::PgPool;

use crate::enterprise::{LicenseFeature, has_enterprise_access};

pub struct ClientCommonServer {
    pool: PgPool,
}

impl ClientCommonServer {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Identifies the calling device without applying any license or state rule.
    async fn authenticate(&self, token: &str) -> Result<ClientDevice, CoreError> {
        if token.is_empty() {
            return Err(CoreError::unauthenticated("missing token")
                .with_code(CoreErrorCode::DeviceTokenInvalid));
        }
        let token = PollingToken::find(&self.pool, token)
            .await
            .map_err(|err| {
                error!("Failed to look up client token: {err}");
                CoreError::internal("unexpected error")
            })?
            .ok_or_else(|| {
                warn!("Client request with an unknown token");
                CoreError::unauthenticated("invalid token")
                    .with_code(CoreErrorCode::DeviceTokenInvalid)
            })?;

        let device = Device::find_by_id(&self.pool, token.device_id)
            .await
            .map_err(|err| {
                error!("Failed to retrieve device id {}: {err}", token.device_id);
                CoreError::internal("unexpected error")
            })?
            .ok_or_else(|| {
                error!("Device id {} of a client token not found", token.device_id);
                CoreError::internal("unexpected error")
            })?;
        let user = User::find_by_device_id(&self.pool, device.id)
            .await
            .map_err(|err| {
                error!("Failed to retrieve user of device id {}: {err}", device.id);
                CoreError::internal("unexpected error")
            })?
            .ok_or_else(|| {
                error!("User of device id {} not found", device.id);
                CoreError::internal("unexpected error")
            })?;

        Ok(ClientDevice { device, user })
    }
}

struct ClientDevice {
    device: Device<Id>,
    user: User<Id>,
}

/// None requires the Enterprise tier, Some also accepts a license granting that feature.
fn require_license(feature: Option<LicenseFeature>) -> Result<(), CoreError> {
    if has_enterprise_access(feature) {
        Ok(())
    } else {
        // Edge matches this exact message to answer with 402
        Err(CoreError::failed_precondition("no valid license")
            .with_code(CoreErrorCode::LicenseRequired))
    }
}

fn require_active_user(client: &ClientDevice) -> Result<(), CoreError> {
    if client.user.is_active {
        Ok(())
    } else {
        warn!(
            "Denying client request of inactive user {}({})",
            client.user.username, client.user.id
        );
        Err(CoreError::permission_denied("user inactive").with_code(CoreErrorCode::UserInactive))
    }
}

fn require_enabled_device(client: &ClientDevice) -> Result<(), CoreError> {
    if client.device.state == DeviceState::Blocked {
        warn!(
            "Denying client request of blocked device {}({})",
            client.device.name, client.device.id
        );
        Err(CoreError::permission_denied("device blocked").with_code(CoreErrorCode::DeviceBlocked))
    } else {
        Ok(())
    }
}
