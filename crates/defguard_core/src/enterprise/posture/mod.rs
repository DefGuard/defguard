use sqlx::PgExecutor;
use thiserror::Error;

use defguard_proto::enterprise::posture::DevicePostureCheckRequest;

use crate::enterprise::is_enterprise_license_active;

#[derive(Debug, Error)]
pub enum PostureCheckError {
    #[error("No active enterprise license found")]
    NoActiveEnterpriseLicense,
    #[error(transparent)]
    DbError(#[from] sqlx::Error),
}

pub enum FailureReason {
    OsNotAllowed,
    MissingPostureData,
}

pub enum PostureResult {
    Pass,
    Fail(Vec<FailureReason>),
}

pub fn validate_posture<'e, E>(
    executor: E,
    request: &DevicePostureCheckRequest,
) -> Result<PostureResult, PostureCheckError>
where
    E: PgExecutor<'e>,
{
    debug!(
        "Performing posture check for device {}: {:?}",
        request.pubkey, request.device_posture_data
    );

    // Postures are only available for enterprise deployments
    if !is_enterprise_license_active() {
        warn!(
            "No active enterprise license - posture check aborted for device {}",
            request.pubkey
        );
        return Err(PostureCheckError::NoActiveEnterpriseLicense);
    }

    let Some(ref data) = request.device_posture_data else {
        info!(
            "Missing posture data - posture check failed for device {}",
            request.pubkey
        );
        return Ok(PostureResult::Fail(vec![FailureReason::MissingPostureData]));
    };

    info!("Posture check successful for device {}", request.pubkey);
    Ok(PostureResult::Pass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        todo!();
    }
}
