use chrono::{NaiveDateTime, Utc};
use defguard_common::db::{
    Id,
    models::{
        DeviceType,
        device_verification::{
            DEVICE_VERIFICATION_CODE_DIGITS, DEVICE_VERIFICATION_CODE_PERIOD, DeviceVerification,
            DeviceVerificationOutcome, DeviceVerificationState,
        },
    },
};
use defguard_proto::{
    client_common::{
        self, DeviceVerificationStartRequest, DeviceVerificationStartResponse,
        DeviceVerificationStatus, DeviceVerificationStatusRequest, TotpAlgorithm,
    },
    proxy::{CoreError, CoreErrorCode},
};

use super::{ClientCommonServer, require_active_user, require_enabled_device, require_license};

impl ClientCommonServer {
    /// Starts a verification session for the calling device, or joins the active one.
    #[instrument(skip_all)]
    pub async fn verification_start(
        &self,
        request: DeviceVerificationStartRequest,
    ) -> Result<DeviceVerificationStartResponse, CoreError> {
        require_license(None)?;
        let client = self.authenticate(&request.token).await?;
        require_active_user(&client)?;
        require_enabled_device(&client)?;
        let device = client.device;
        if device.device_type != DeviceType::User {
            return Err(CoreError::failed_precondition("not a user device"));
        }

        let mut conn = self.pool.acquire().await.map_err(|err| {
            error!("Failed to acquire database connection: {err}");
            CoreError::internal("unexpected error")
        })?;
        let (verification, created) = DeviceVerification::new(device.id, None)
            .start_or_get(&mut conn)
            .await
            .map_err(|err| {
                error!(
                    "Failed to start verification of device {}({}): {err}",
                    device.name, device.id
                );
                CoreError::internal("failed to start verification")
            })?;
        if created {
            info!(
                "Device {}({}) started its verification",
                device.name, device.id
            );
        }

        Ok(DeviceVerificationStartResponse {
            session_id: verification.id,
            secret: verification.secret,
            algorithm: TotpAlgorithm::Sha1.into(),
            digits: DEVICE_VERIFICATION_CODE_DIGITS,
            period: DEVICE_VERIFICATION_CODE_PERIOD as u32,
            expires_at: unix_timestamp(verification.expires_at),
        })
    }

    /// Administrator decisions are also pushed to Edge.
    #[instrument(skip_all)]
    pub async fn verification_status(
        &self,
        request: DeviceVerificationStatusRequest,
    ) -> Result<DeviceVerificationStatus, CoreError> {
        require_license(None)?;
        let client = self.authenticate(&request.token).await?;
        // blocked devices are let through to learn that the administrator blocked them
        require_active_user(&client)?;
        let device_id = client.device.id;
        let verification =
            DeviceVerification::find_for_device(&self.pool, request.session_id, device_id)
                .await
                .map_err(|err| {
                    error!(
                        "Failed to retrieve verification {} of device {device_id}: {err}",
                        request.session_id
                    );
                    CoreError::internal("failed to retrieve verification")
                })?
                .ok_or_else(|| {
                    CoreError::not_found("verification session not found")
                        .with_code(CoreErrorCode::VerificationSessionNotFound)
                })?;

        Ok(status_message(&verification))
    }
}

/// Maps a session to the message used both for status replies and for pushes to Edge.
#[must_use]
pub fn status_message(verification: &DeviceVerification<Id>) -> DeviceVerificationStatus {
    let outcome = match verification.state_at(Utc::now().naive_utc()) {
        DeviceVerificationState::Pending => client_common::DeviceVerificationOutcome::Pending,
        DeviceVerificationState::Expired => client_common::DeviceVerificationOutcome::Expired,
        DeviceVerificationState::Resolved(DeviceVerificationOutcome::Verified) => {
            client_common::DeviceVerificationOutcome::Verified
        }
        DeviceVerificationState::Resolved(DeviceVerificationOutcome::Blocked) => {
            client_common::DeviceVerificationOutcome::Blocked
        }
        DeviceVerificationState::Resolved(DeviceVerificationOutcome::Cancelled) => {
            client_common::DeviceVerificationOutcome::Cancelled
        }
    };
    DeviceVerificationStatus {
        session_id: verification.id,
        outcome: outcome.into(),
        expires_at: unix_timestamp(verification.expires_at),
        resolved_at: verification.resolved_at.map(unix_timestamp),
    }
}

fn unix_timestamp(value: NaiveDateTime) -> i64 {
    value.and_utc().timestamp()
}
