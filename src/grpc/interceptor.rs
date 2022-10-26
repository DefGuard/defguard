use crate::auth::{Claims, ClaimsType};
use tonic::{Request, Status};

/// Auth interceptor used by GRPC services. Verifies JWT token sent
/// in GRPC metadata under "authorization" key.
pub fn jwt_auth_interceptor(mut req: Request<()>) -> Result<Request<()>, Status> {
    let token = match req.metadata().get("authorization") {
        Some(token) => token
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid token"))?,
        None => return Err(Status::unauthenticated("Missing authorization header")),
    };
    if let Ok(claims) = Claims::from_jwt(ClaimsType::Gateway, token) {
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
