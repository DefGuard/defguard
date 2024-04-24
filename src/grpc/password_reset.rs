use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;

use crate::{
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
    server_config,
};

use super::proto::{
    PasswordResetInitializeRequest, PasswordResetRequest, PasswordResetStartRequest,
    PasswordResetStartResponse,
};

pub(super) struct PasswordResetServer {
    pool: DbPool,
    mail_tx: UnboundedSender<Mail>,
    // ldap_feature_active: bool,
}

impl PasswordResetServer {
    #[must_use]
    pub fn new(pool: DbPool, mail_tx: UnboundedSender<Mail>) -> Self {
        // FIXME: check if LDAP feature is enabled
        // let ldap_feature_active = true;
        Self {
            pool,
            mail_tx,
            // ldap_feature_active,
        }
    }

    // check if token provided with request corresponds to a valid enrollment session
    async fn validate_session(&self, token: Option<&str>) -> Result<Token, Status> {
        let Some(token) = token else {
            error!("Missing authorization header in request");
            return Err(Status::unauthenticated("Missing authorization header"));
        };

        debug!("Validating enrollment session token: {token}");
        let enrollment = Token::find_by_id(&self.pool, token).await?;

        if enrollment.is_session_valid(server_config().enrollment_session_timeout.as_secs()) {
            Ok(enrollment)
        } else {
            error!("Enrollment session expired");
            Err(Status::unauthenticated("Enrollment session expired"))
        }
    }

    pub async fn request_password_reset(
        &self,
        request: PasswordResetInitializeRequest,
        req_device_info: Option<super::proto::DeviceInfo>,
    ) -> Result<(), Status> {
        let config = server_config();
        debug!("Starting password reset request");

        let ip_address;
        let user_agent;
        if let Some(info) = req_device_info {
            ip_address = info.ip_address.unwrap_or_default();
            user_agent = info.user_agent.unwrap_or_default();
        } else {
            ip_address = String::new();
            user_agent = String::new();
        }

        let email = request.email;

        let user = User::find_by_email(&self.pool, email.to_string().as_str())
            .await
            .map_err(|_| {
                error!("Failed to fetch user by email");
                Status::internal("unexpected error")
            })?;

        let Some(user) = user else {
            // Do not return information whether user exists
            return Ok(());
        };

        // Do not allow password change if user is not active
        if !user.has_password() {
            return Ok(());
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
            config.password_reset_token_timeout.as_secs(),
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
            config.enrollment_url.clone(),
            &enrollment.id,
            Some(&ip_address),
            Some(&user_agent),
        )?;

        Ok(())
    }

    pub async fn start_password_reset(
        &self,
        request: PasswordResetStartRequest,
    ) -> Result<PasswordResetStartResponse, Status> {
        debug!("Starting password reset session: {request:?}");

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
                server_config().password_reset_session_timeout.as_secs(),
            )
            .await?;

        let response = PasswordResetStartResponse {
            deadline_timestamp: session_deadline.and_utc().timestamp(),
        };

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        Ok(response)
    }

    pub async fn reset_password(
        &self,
        request: PasswordResetRequest,
        req_device_info: Option<super::proto::DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Starting password reset: {request:?}");
        let enrollment = self.validate_session(request.token.as_deref()).await?;

        let ip_address;
        let user_agent;
        if let Some(info) = req_device_info {
            ip_address = info.ip_address.unwrap_or_default();
            user_agent = info.user_agent.unwrap_or_default();
        } else {
            ip_address = String::new();
            user_agent = String::new();
        }

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

        // if self.ldap_feature_active {
        let _ = ldap_change_password(&self.pool, &user.username, &request.password).await;
        // };

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

        Ok(())
    }
}
