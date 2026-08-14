use defguard_proto::proxy::{ClientMfaOidcAuthenticateRequest, DeviceInfo};
use tonic::Status;

use crate::{enterprise::is_business_license_active, grpc::proxy::client_mfa::ClientMfaServer};

impl ClientMfaServer {
    #[instrument(skip_all)]
    pub async fn auth_mfa_session_with_oidc(
        &mut self,
        request: ClientMfaOidcAuthenticateRequest,
        info: Option<DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Received OIDC MFA authentication request: {request:?}");
        if !is_business_license_active() {
            error!("OIDC MFA method requires enterprise feature to be enabled");
            return Err(Status::invalid_argument("OIDC MFA method is not supported"));
        }
        // TODO(#3043): resolve against the durable store (Step 3.4). Until then, the OIDC
        // callback path is non-functional.
        let _ = info;
        Err(Status::unimplemented("OIDC MFA login not yet implemented"))
    }
}
