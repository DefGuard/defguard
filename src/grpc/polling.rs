use crate::{
    db::{
        models::enrollment::{Token, AUTH_TOKEN_TYPE},
        DbPool,
    },
    grpc::utils::build_device_config_response,
};
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
    async fn validate_session(&self, token: &str) -> Result<Token, Status> {
        debug!("Validating auth session. Token: {token}");
        let token = Token::find_by_id(&self.pool, token).await?;
        debug!("Found matching token, verifying validity: {token:?}.");
        // Auth tokens are valid indefinitely
        if token
            .token_type
            .as_ref()
            .is_some_and(|token_type| token_type == AUTH_TOKEN_TYPE)
        {
            Ok(token)
        } else {
            error!(
                "Invalid token type used in polling process: {:?}",
                token.token_type
            );
            Err(Status::permission_denied("invalid token"))
        }
    }

    /// Get all information needed to update instance information for desktop client
    pub async fn info(&self, request: InstanceInfoRequest) -> Result<InstanceInfoResponse, Status> {
        debug!("Getting network info for device: {:?}", request.pubkey);
        let token = self.validate_session(&request.token).await?;
        let device_config =
            build_device_config_response(&self.pool, &token, &request.pubkey).await?;
        Ok(InstanceInfoResponse {
            device_config: Some(device_config),
            // TODO(jck): actually check enterprise status
            is_enterprise: true,
        })
    }
}
