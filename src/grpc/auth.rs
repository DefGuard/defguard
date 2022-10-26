use crate::{
    auth::{Claims, ClaimsType, SESSION_TIMEOUT},
    db::{DbPool, User},
};
use jsonwebtoken::errors::Error as JWTError;
use tonic::{Request, Response, Status};

tonic::include_proto!("auth");

pub struct AuthServer {
    pool: DbPool,
}

impl AuthServer {
    #[must_use]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Creates JWT token for specified user
    fn create_jwt(uid: &str) -> Result<String, JWTError> {
        Claims::new(ClaimsType::Auth, uid.into(), String::new(), SESSION_TIMEOUT).to_jwt()
    }
}

#[tonic::async_trait]
impl auth_service_server::AuthService for AuthServer {
    /// Authentication gRPC service. Verifies provided username and password
    /// agains LDAP and returns JWT token if correct.
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let request = request.into_inner();
        debug!("Authenticating user {}", &request.username);
        match User::find_by_username(&self.pool, &request.username).await {
            Ok(Some(user)) => match user.verify_password(&request.password) {
                Ok(_) => {
                    info!("Authentication successful for user {}", &request.username);
                    Ok(Response::new(AuthenticateResponse {
                        token: Self::create_jwt(&request.username)
                            .map_err(|_| Status::unauthenticated("error creating JWT token"))?,
                    }))
                }
                Err(_) => Err(Status::unauthenticated("invalid credentials")),
            },
            _ => Err(Status::unauthenticated("user not found")),
        }
    }
}
