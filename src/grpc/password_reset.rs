use tokio::sync::mpsc::UnboundedSender;
use tonic::{Request, Response, Status};

use crate::{
    config::DefGuardConfig,
    db::{
        models::enrollment::{Token, PASSWORD_RESET_TOKEN_TYPE},
        DbPool, User,
    },
    handlers::{
        mail::{send_password_reset_email, send_password_reset_success_email},
        user::check_password_strength,
    },
    ldap::utils::ldap_change_password,
    mail::Mail,
};

use super::proto::{
    password_reset_service_server, PasswordResetInitializeRequest, PasswordResetRequest,
    PasswordResetStartRequest, PasswordResetStartResponse,
};

pub struct PasswordResetServer {
    pool: DbPool,
    mail_tx: UnboundedSender<Mail>,
    config: DefGuardConfig,
    ldap_feature_active: bool,
}

impl PasswordResetServer {
    #[must_use]
    pub fn new(pool: DbPool, mail_tx: UnboundedSender<Mail>, config: DefGuardConfig) -> Self {
        // FIXME: check if LDAP feature is enabled
        let ldap_feature_active = true;
        Self {
            pool,
            mail_tx,
            config,
            ldap_feature_active,
        }
    }

    // check if token provided with request corresponds to a valid enrollment session
    async fn validate_session<T: std::fmt::Debug>(
        &self,
        request: &Request<T>,
    ) -> Result<Token, Status> {
        debug!("Validating enrollment session token: {request:?}");
        let token = if let Some(token) = request.metadata().get("authorization") {
            token
                .to_str()
                .map_err(|_| Status::unauthenticated("Invalid token"))?
        } else {
            error!("Missing authorization header in request");
            return Err(Status::unauthenticated("Missing authorization header"));
        };

        let enrollment = Token::find_by_id(&self.pool, token).await?;

        if enrollment.is_session_valid(self.config.enrollment_session_timeout.as_secs()) {
            Ok(enrollment)
        } else {
            error!("Enrollment session expired");
            Err(Status::unauthenticated("Enrollment session expired"))
        }
    }
}

#[tonic::async_trait]
impl password_reset_service_server::PasswordResetService for PasswordResetServer {
    async fn request_password_reset(
        &self,
        request: Request<PasswordResetInitializeRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Starting password reset request");

        let ip_address = request
            .metadata()
            .get("ip_address")
            .and_then(|value| value.to_str().map(ToString::to_string).ok())
            .unwrap_or_default();

        let user_agent = request
            .metadata()
            .get("user_agent")
            .and_then(|value| value.to_str().map(ToString::to_string).ok())
            .unwrap_or_default();

        let request = request.into_inner();
        let email = request.email;

        let user = User::find_by_email(&self.pool, email.to_string().as_str())
            .await
            .map_err(|_| {
                error!("Failed to fetch user by email");
                Status::internal("unexpected error")
            })?;

        if user.is_none() {
            // Do not return information whether user exists
            return Ok(Response::new(()));
        }

        let user = user.unwrap();

        // Do not allow password change if user is not active
        if !user.has_password() {
            return Ok(Response::new(()));
        }

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        Token::delete_unused_user_password_reset_tokens(
            &mut transaction,
            user.id.expect("Missing user ID"),
        )
        .await?;

        let enrollment = Token::new(
            user.id.expect("Missing user ID"),
            None,
            Some(email.clone()),
            self.config.password_reset_token_timeout.as_secs(),
            Some(PASSWORD_RESET_TOKEN_TYPE.to_string()),
        );
        enrollment.save(&mut transaction).await?;

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        send_password_reset_email(
            &user,
            &self.mail_tx,
            self.config.enrollment_url.clone(),
            &enrollment.id,
            Some(&ip_address),
            Some(&user_agent),
        )?;

        Ok(Response::new(()))
    }

    async fn start_password_reset(
        &self,
        request: Request<PasswordResetStartRequest>,
    ) -> Result<Response<PasswordResetStartResponse>, Status> {
        debug!("Starting password reset session: {request:?}");
        let request = request.into_inner();

        let mut enrollment = Token::find_by_id(&self.pool, &request.token).await?;

        if enrollment.token_type != Some("PASSWORD_RESET".to_string()) {
            return Err(Status::permission_denied("invalid token"));
        }

        let user = enrollment.fetch_user(&self.pool).await?;

        if !user.has_password() {
            return Err(Status::permission_denied("user inactive"));
        }

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        let session_deadline = enrollment
            .start_session(
                &mut transaction,
                self.config.password_reset_session_timeout.as_secs(),
            )
            .await?;

        let response = PasswordResetStartResponse {
            deadline_timestamp: session_deadline.timestamp(),
        };

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        Ok(Response::new(response))
    }

    async fn reset_password(
        &self,
        request: Request<PasswordResetRequest>,
    ) -> Result<Response<()>, Status> {
        debug!("Starting password reset: {request:?}");
        let enrollment = self.validate_session(&request).await?;

        let ip_address = request
            .metadata()
            .get("ip_address")
            .and_then(|value| value.to_str().map(ToString::to_string).ok())
            .unwrap_or_default();

        let user_agent = request
            .metadata()
            .get("user_agent")
            .and_then(|value| value.to_str().map(ToString::to_string).ok())
            .unwrap_or_default();

        let request = request.into_inner();
        if let Err(err) = check_password_strength(&request.password) {
            error!("Password not strong enough: {err}");
            return Err(Status::invalid_argument("password not strong enough"));
        }

        let mut user = enrollment.fetch_user(&self.pool).await?;

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        // update user
        user.set_password(&request.password);
        user.save(&mut *transaction).await.map_err(|err| {
            error!("Failed to update user {}: {err}", user.username);
            Status::internal("unexpected error")
        })?;

        if self.ldap_feature_active {
            let _ = ldap_change_password(&self.pool, &user.username, &request.password).await;
        };

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        send_password_reset_success_email(
            &user,
            &self.mail_tx,
            Some(&ip_address),
            Some(&user_agent),
        )?;

        Ok(Response::new(()))
    }
}
