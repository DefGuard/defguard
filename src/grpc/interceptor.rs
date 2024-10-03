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
        let token = match req.metadata().get("authorization") {
            Some(token) => token
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid token"))?,
            None => return Err(Status::unauthenticated("Missing authorization header")),
        };
        if let Ok(claims) = Claims::from_jwt(self.claims_type, token) {
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
        } else {
            Err(Status::unauthenticated("Invalid token"))
        }
    }
}
