use defguard_common::{
    config::server_config,
    db::models::{Settings, User},
};
use defguard_core::{
    db::models::enrollment::{PASSWORD_RESET_TOKEN_TYPE, Token},
    enterprise::ldap::utils::ldap_change_password,
    events::{BidiRequestContext, BidiStreamEvent, BidiStreamEventType, PasswordResetEvent},
    grpc::utils::parse_client_ip_agent,
    handlers::{
        mail::{send_password_reset_email, send_password_reset_success_email},
        user::check_password_strength,
    },
    headers::get_device_info,
};
use defguard_proto::proxy::{
    DeviceInfo, PasswordResetInitializeRequest, PasswordResetRequest, PasswordResetStartRequest,
    PasswordResetStartResponse,
};
use sqlx::PgPool;
use tokio::sync::mpsc::{UnboundedSender, error::SendError};
use tonic::Status;

pub(crate) struct PasswordResetServer {
    pool: PgPool,
    bidi_event_tx: UnboundedSender<BidiStreamEvent>,
}

impl PasswordResetServer {
    #[must_use]
    pub fn new(pool: PgPool, bidi_event_tx: UnboundedSender<BidiStreamEvent>) -> Self {
        // FIXME: check if LDAP feature is enabled
        // let ldap_feature_active = true;
        Self {
            pool,
            bidi_event_tx,
            // ldap_feature_active,
        }
    }

    /// Checks if token provided with request corresponds to a valid password reset session
    async fn validate_session(&self, token: Option<&String>) -> Result<Token, Status> {
        info!("Validating password reset session. Token: {token:?}");
        let Some(token) = token else {
            error!("Missing authorization header in request");
            return Err(Status::unauthenticated("Missing authorization header"));
        };
        let enrollment = Token::find_by_id(&self.pool, token).await?;
        debug!("Found matching token, verifying validity: {enrollment:?}.");
        if enrollment
            .token_type
            .as_ref()
            .is_none_or(|token_type| token_type != PASSWORD_RESET_TOKEN_TYPE)
        {
            error!(
                "Invalid token type used in password reset process: {:?}",
                enrollment.token_type
            );
            return Err(Status::permission_denied("invalid token"));
        }

        if enrollment.is_session_valid(server_config().enrollment_session_timeout.as_secs()) {
            info!("Password reset session validated: {enrollment:?}.",);
            Ok(enrollment)
        } else {
            error!("Password reset session expired: {enrollment:?}");
            Err(Status::unauthenticated("Session expired"))
        }
    }

    // Send event to the dedicated bidi stream event channel
    fn emit_event(
        &self,
        context: BidiRequestContext,
        event: PasswordResetEvent,
    ) -> Result<(), SendError<BidiStreamEvent>> {
        let event = BidiStreamEvent {
            context,
            event: BidiStreamEventType::PasswordReset(Box::new(event)),
        };

        self.bidi_event_tx.send(event)
    }

    #[instrument(skip_all)]
    pub(crate) async fn request_password_reset(
        &self,
        request: PasswordResetInitializeRequest,
        req_device_info: Option<DeviceInfo>,
    ) -> Result<(), Status> {
        let config = server_config();
        debug!("Starting password reset request");

        let ip_address;
        let device_info;
        if let Some(info) = &req_device_info {
            ip_address = info.ip_address.clone();
            let agent = info.user_agent.clone().unwrap_or_default();
            device_info = get_device_info(&agent);
        } else {
            ip_address = String::new();
            device_info = String::new();
        }

        let email = request.email;

        let user = User::find_by_email(&self.pool, email.clone().as_str())
            .await
            .map_err(|_| {
                error!("Failed to fetch user by email: {email}");
                Status::internal("unexpected error")
            })?;

        let Some(user) = user else {
            // Do not return information whether user exists
            debug!("Password reset skipped for non-existing user {email}");
            return Ok(());
        };

        // Do not allow password change if user is disabled or not enrolled
        if !user.has_password() || !user.is_active {
            debug!(
                "Password reset skipped for disabled or not enrolled user {} ({email})",
                user.username
            );
            return Ok(());
        }

        let mut transaction = self.pool.begin().await.map_err(|_| {
            error!("Failed to begin transaction");
            Status::internal("unexpected error")
        })?;

        Token::delete_unused_user_password_reset_tokens(&mut transaction, user.id).await?;

        let enrollment = Token::new(
            user.id,
            None,
            Some(email.clone()),
            config.password_reset_token_timeout.as_secs(),
            Some(PASSWORD_RESET_TOKEN_TYPE.to_string()),
        );
        enrollment.save(&mut *transaction).await?;

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        let settings = Settings::get_current_settings();
        let public_proxy_url = settings.proxy_public_url().map_err(|err| {
            error!("Failed to get public proxy URL: {err}");
            Status::internal("unexpected error")
        })?;

        send_password_reset_email(
            &user,
            public_proxy_url,
            &enrollment.id,
            Some(&ip_address),
            Some(&device_info),
        )?;

        info!(
            "Finished processing password reset request for user {}.",
            user.username
        );

        // Prepare event context and push the event
        let (ip, user_agent) = parse_client_ip_agent(&req_device_info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(user.id, user.username, ip, user_agent);
        self.emit_event(context, PasswordResetEvent::PasswordResetRequested)
            .map_err(|err| {
                error!("Failed to send event. Reason: {err}",);
                Status::internal("unexpected error")
            })?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub(crate) async fn start_password_reset(
        &self,
        request: PasswordResetStartRequest,
        info: Option<DeviceInfo>,
    ) -> Result<PasswordResetStartResponse, Status> {
        debug!("Starting password reset session: {request:?}");

        let mut enrollment = Token::find_by_id(&self.pool, &request.token).await?;

        if enrollment.token_type != Some("PASSWORD_RESET".to_string()) {
            error!(
                "Invalid token type ({:?}) for password reset session",
                enrollment.token_type
            );
            return Err(Status::permission_denied("invalid token"));
        }

        let user = enrollment.fetch_user(&self.pool).await?;

        if !user.has_password() || !user.is_active {
            error!(
                "Can't start password reset for a disabled or not enrolled user {}.",
                user.username
            );
            return Err(Status::permission_denied(
                "user disabled or not yet enrolled",
            ));
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

        info!(
            "Finished processing password reset session for user {}.",
            user.username
        );
        // Prepare event context and push the event
        let (ip, user_agent) = parse_client_ip_agent(&info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(user.id, user.username, ip, user_agent);
        self.emit_event(context, PasswordResetEvent::PasswordResetStarted)
            .map_err(|err| {
                error!("Failed to send event. Reason: {err}",);
                Status::internal("unexpected error")
            })?;

        Ok(response)
    }

    #[instrument(skip_all)]
    pub(crate) async fn reset_password(
        &self,
        request: PasswordResetRequest,
        req_device_info: Option<DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Starting password reset");
        let enrollment = self.validate_session(request.token.as_ref()).await?;

        let ip_address;
        let device_info;
        if let Some(ref info) = req_device_info {
            ip_address = info.ip_address.clone();
            let agent = info.user_agent.clone().unwrap_or_default();
            device_info = get_device_info(&agent);
        } else {
            ip_address = String::new();
            device_info = String::new();
        }

        if let Err(err) = check_password_strength(&request.password) {
            error!("Password not strong enough: {err}");
            return Err(Status::invalid_argument("password not strong enough"));
        }

        let mut user = enrollment.fetch_user(&self.pool).await?;

        if !user.is_active {
            error!(
                "Can't reset password for a disabled user {}.",
                user.username
            );
            return Err(Status::permission_denied("user disabled"));
        }

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

        transaction.commit().await.map_err(|_| {
            error!("Failed to commit transaction");
            Status::internal("unexpected error")
        })?;

        ldap_change_password(&mut user, &request.password, &self.pool).await;

        send_password_reset_success_email(&user, Some(&ip_address), Some(&device_info))?;

        // Prepare event context and push the event
        let (ip, user_agent) = parse_client_ip_agent(&req_device_info).map_err(Status::internal)?;
        let context = BidiRequestContext::new(user.id, user.username, ip, user_agent);
        self.emit_event(context, PasswordResetEvent::PasswordResetCompleted)
            .map_err(|err| {
                error!("Failed to send event. Reason: {err}",);
                Status::internal("unexpected error")
            })?;

        Ok(())
    }
}
