use std::sync::{Arc, Mutex};

use defguard_common::{
    auth::claims::{Claims, ClaimsType},
    db::models::{Settings, User},
};
use defguard_proto::auth::{AuthenticateRequest, AuthenticateResponse, auth_service_server};
use jsonwebtoken::errors::Error as JWTError;
use sqlx::PgPool;
use tonic::{Request, Response, Status};

use crate::auth::failed_login::{FailedLoginMap, check_failed_logins, log_failed_login_attempt};

pub(super) struct AuthServer {
    pool: PgPool,
    failed_logins: Arc<Mutex<FailedLoginMap>>,
}

impl AuthServer {
    #[must_use]
    pub fn new(pool: PgPool, failed_logins: Arc<Mutex<FailedLoginMap>>) -> Self {
        Self {
            pool,
            failed_logins,
        }
    }

    /// Creates JWT token for specified user
    fn create_jwt(uid: &str) -> Result<String, JWTError> {
        let settings = Settings::get_current_settings();
        let timeout = settings.authentication_timeout();
        Claims::new(
            ClaimsType::Auth,
            uid.into(),
            String::new(),
            timeout.as_secs(),
        )
        .to_jwt()
    }
}

#[tonic::async_trait]
impl auth_service_server::AuthService for AuthServer {
    /// Authentication gRPC service. Verifies provided username and password
    /// against LDAP and returns JWT token if correct.
    async fn authenticate(
        &self,
        request: Request<AuthenticateRequest>,
    ) -> Result<Response<AuthenticateResponse>, Status> {
        let request = request.into_inner();
        debug!("Authenticating user {}", request.username);
        // check if user can proceed with login
        check_failed_logins(&self.failed_logins, &request.username)
            .map_err(|_| Status::resource_exhausted("too many login requests"))?;

        if let Ok(Some(user)) = User::find_by_username(&self.pool, &request.username).await {
            if user.verify_password(&request.password).is_ok() {
                info!("Authentication successful for user {}", request.username);
                Ok(Response::new(AuthenticateResponse {
                    token: Self::create_jwt(&request.username).map_err(|_| {
                        log_failed_login_attempt(&self.failed_logins, &request.username);
                        Status::unauthenticated("error creating JWT token")
                    })?,
                }))
            } else {
                warn!("Invalid login credentials for user {}", request.username);
                log_failed_login_attempt(&self.failed_logins, &request.username);
                Err(Status::unauthenticated("invalid credentials"))
            }
        } else {
            warn!("User {} not found", request.username);
            log_failed_login_attempt(&self.failed_logins, &request.username);
            Err(Status::unauthenticated("invalid credentials"))
        }
    }
}
