use tonic::{service::Interceptor, Status};

use crate::auth::{Claims, ClaimsType};

/// Auth interceptor used by gRPC services. Verifies JWT token sent
/// in gRPC metadata under "authorization" key.
#[derive(Clone)]
pub struct JwtInterceptor {
    claims_type: ClaimsType,
}

impl JwtInterceptor {
    #[must_use]
    pub fn new(claims_type: ClaimsType) -> Self {
        Self { claims_type }
    }
}

impl Interceptor for JwtInterceptor {
    fn call(&mut self, mut req: tonic::Request<()>) -> Result<tonic::Request<()>, Status> {
        // This is only used for logging purposes, so no proper error handling
        let hostname = req
            .metadata()
            .get("hostname")
            .map_or("UNKNOWN", |h| h.to_str().unwrap_or("UNKNOWN"));

        let token = match req.metadata().get("authorization") {
            Some(token) => token.to_str().map_err(|err| {
                warn!("Failed to parse authorization header during handling gRPC request from hostname {}. \
                If you recognize this hostname, there may be an issue with the token used for authorization. \
                Cause of the failed parsing: {:?}", hostname, err);
                Status::unauthenticated("Invalid token")
            })?,
            None => return Err(Status::unauthenticated("Missing authorization header")),
        };

        match Claims::from_jwt(self.claims_type, token) {
            Ok(claims) => {
                let request_metadata = req.metadata_mut();

                if let ClaimsType::Gateway = self.claims_type {
                    request_metadata.insert(
                        "gateway_network_id",
                        claims
                            .client_id
                            .parse()
                            .map_err(|_| Status::unknown("Network ID parsing error"))?,
                    );
                }

                // FIXME: can we push whole Claims object into metadata?
                request_metadata.insert(
                    "username",
                    claims
                        .sub
                        .parse()
                        .map_err(|_| Status::unknown("Username parsing error"))?,
                );
                debug!("Authorization successful for user {}", claims.sub);
                Ok(req)
            }
            Err(err) => {
                warn!(
                    "Failed to authorize a gRPC request from hostname {}. \
                    If you recognize this hostname, there may be an issue with the token used for authorization. \
                    Cause of the failed authorization: {:?}",
                    hostname, err
                );
                Err(Status::unauthenticated("Invalid token"))
            }
        }
    }
}
