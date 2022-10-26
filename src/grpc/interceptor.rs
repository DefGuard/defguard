use crate::auth::{Claims, ClaimsType};
use tonic::{service::Interceptor, Status};

/// Auth interceptor used by GRPC services. Verifies JWT token sent
/// in GRPC metadata under "authorization" key.
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
        if let Ok(claims) = Claims::from_jwt(self.claims_type.clone(), token) {
            // FIXME: can we push whole Claims object into metadata?
            req.metadata_mut().insert(
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
