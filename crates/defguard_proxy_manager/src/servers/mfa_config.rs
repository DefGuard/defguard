use defguard_common::db::{
    Id,
    models::{
        Device, Settings, User, polling_token::PollingToken, vpn_client_session::VpnClientMfaMethod,
    },
};
use defguard_core::{
    db::models::enrollment::{MFA_CONFIG_SESSION_TIMEOUT, MFA_CONFIG_TOKEN_TYPE, Token},
    mail::templates::mfa_code_mail,
};
use defguard_proto::client_types::{
    MfaConfigAuthorizeRequest, MfaConfigAuthorizeResponse, MfaConfigSendCodeRequest,
    MfaConfigSendCodeResponse, MfaConfigStartRequest, MfaConfigStartResponse, MfaMethod,
};
use sqlx::PgPool;
use tonic::Status;

/// An unauthorized MFA configuration session and its user's current factor state.
struct MfaConfigSession {
    token: Token,
    user: User<Id>,
    totp_configured: bool,
    email_configured: bool,
}

/// Handles MFA factor configuration requested by an already enrolled desktop client.
pub(crate) struct MfaConfigServer {
    pool: PgPool,
}

impl MfaConfigServer {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn is_configured(
        &self,
        method: VpnClientMfaMethod,
        user: &User<Id>,
        device_id: Id,
        smtp_configured: bool,
    ) -> Result<bool, Status> {
        method
            .is_configured(&self.pool, user, Some(device_id), smtp_configured, false)
            .await
            .map_err(|err| {
                error!(
                    "MFA config: failed to read {method:?} state for user {}: {err}",
                    user.username
                );
                Status::internal("unexpected error")
            })
    }

    /// Starts an MFA configuration session for the device identified by the polling token.
    ///
    /// The returned token remains unused until the user proves an existing factor.
    /// The client requests an email code separately with `mfa_config_send_code`.
    /// When no factor is configured, an email code is the only authorization method.
    #[instrument(skip_all)]
    pub(crate) async fn mfa_config_start(
        &self,
        request: MfaConfigStartRequest,
    ) -> Result<MfaConfigStartResponse, Status> {
        debug!(
            "Starting MFA configuration for device pubkey {}",
            request.pubkey
        );
        // Authenticate before any lookup so error codes cannot probe which pubkeys exist.
        if request.token.is_empty() {
            error!("MFA config start: missing polling token");
            return Err(Status::unauthenticated("missing token"));
        }
        let polling_token = PollingToken::find(&self.pool, &request.token)
            .await
            .map_err(|err| {
                error!("MFA config start: failed to look up polling token: {err}");
                Status::internal("unexpected error")
            })?
            .ok_or_else(|| {
                error!("MFA config start: unknown polling token");
                Status::unauthenticated("invalid token")
            })?;
        let Ok(Some(device)) = Device::find_by_pubkey(&self.pool, &request.pubkey).await else {
            error!(
                "MFA config start: device with pubkey {} not found",
                request.pubkey
            );
            return Err(Status::unauthenticated("invalid token"));
        };
        if polling_token.device_id != device.id {
            error!(
                "MFA config start: polling token belongs to device {} but request claims device {}",
                polling_token.device_id, device.id
            );
            return Err(Status::unauthenticated("token does not match device"));
        }
        let Ok(Some(mut user)) = User::find_by_id(&self.pool, device.user_id).await else {
            error!("MFA config start: user {} not found", device.user_id);
            return Err(Status::internal("user not found"));
        };
        if !user.is_active {
            error!("MFA config start: user {} is inactive", user.username);
            return Err(Status::permission_denied("user is inactive"));
        }

        let settings = Settings::get_current_settings();
        let smtp_configured = settings.smtp_configured();
        let mut available_methods = Vec::with_capacity(2);
        for method in [VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email] {
            if self
                .is_configured(method, &user, device.id, smtp_configured)
                .await?
            {
                available_methods.push(MfaMethod::from(method) as i32);
            }
        }

        let mut transaction = self.pool.begin().await.map_err(|err| {
            error!("MFA config start: failed to begin transaction: {err}");
            Status::internal("unexpected error")
        })?;

        let email_fallback = available_methods.is_empty();
        if email_fallback {
            if !smtp_configured {
                error!(
                    "MFA config start: user {} has no MFA method and SMTP is not configured",
                    user.username
                );
                return Err(Status::failed_precondition("SMTP not configured"));
            }
            // Rotate the secret because a disabled email factor retains its old secret.
            user.new_email_secret(&mut *transaction)
                .await
                .map_err(|err| {
                    error!("MFA config start: failed to create email secret: {err}");
                    Status::internal("unexpected error")
                })?;
        }

        Token::delete_unused_user_tokens_of_type(&mut *transaction, user.id, MFA_CONFIG_TOKEN_TYPE)
            .await?;
        let mut token = Token::new(
            user.id,
            None,
            None,
            MFA_CONFIG_SESSION_TIMEOUT.as_secs(),
            Some(MFA_CONFIG_TOKEN_TYPE.to_owned()),
        );
        token.device_id = Some(device.id);
        token.save(&mut *transaction).await?;

        transaction.commit().await.map_err(|err| {
            error!("MFA config start: failed to commit transaction: {err}");
            Status::internal("unexpected error")
        })?;
        info!(
            "User {} started MFA configuration from device {} (email fallback: {email_fallback})",
            user.username, device.name
        );

        Ok(MfaConfigStartResponse {
            session_token: token.id,
            available_methods,
            email_fallback,
            deadline_timestamp: token.expires_at.and_utc().timestamp(),
        })
    }

    /// Loads an unauthorized MFA configuration session and its user's current factor state.
    async fn load_session(&self, session_token: &str) -> Result<MfaConfigSession, Status> {
        let token = Token::find_by_id(&self.pool, session_token).await?;
        if token.token_type.as_deref() != Some(MFA_CONFIG_TOKEN_TYPE) {
            error!(
                "MFA config: token {} has type {:?}, expected {MFA_CONFIG_TOKEN_TYPE}",
                token.id, token.token_type
            );
            return Err(Status::unauthenticated("invalid token"));
        }
        if token.is_expired() {
            error!("MFA config: token {} expired", token.id);
            return Err(Status::unauthenticated("invalid token"));
        }
        if token.is_used() {
            error!("MFA config: token {} is already authorized", token.id);
            return Err(Status::failed_precondition("session already authorized"));
        }
        let user = token.fetch_user(&self.pool).await?;
        if !user.is_active {
            error!("MFA config: user {} is inactive", user.username);
            return Err(Status::permission_denied("user is inactive"));
        }
        let Some(device_id) = token.device_id else {
            error!("MFA config: token {} has no device", token.id);
            return Err(Status::internal("unexpected error"));
        };

        let smtp_configured = Settings::get_current_settings().smtp_configured();
        let totp_configured = self
            .is_configured(VpnClientMfaMethod::Totp, &user, device_id, smtp_configured)
            .await?;
        let email_configured = self
            .is_configured(VpnClientMfaMethod::Email, &user, device_id, smtp_configured)
            .await?;

        Ok(MfaConfigSession {
            token,
            user,
            totp_configured,
            email_configured,
        })
    }

    /// Sends the email code used to authorize an MFA configuration session.
    ///
    /// The client calls this after the user selects Email, including the no-factor fallback.
    #[instrument(skip_all)]
    pub(crate) async fn mfa_config_send_code(
        &self,
        request: MfaConfigSendCodeRequest,
    ) -> Result<MfaConfigSendCodeResponse, Status> {
        debug!("Sending MFA configuration email code");
        let session = self.load_session(&request.session_token).await?;
        let user = &session.user;
        let email_fallback = !session.totp_configured && !session.email_configured;
        if !session.email_configured && !email_fallback {
            error!(
                "MFA config send code: user {} has no configured email MFA",
                user.username
            );
            return Err(Status::permission_denied("method not configured"));
        }

        let code = user.generate_email_mfa_code().map_err(|err| {
            error!("MFA config send code: failed to generate email code: {err}");
            Status::internal("unexpected error")
        })?;
        let mut conn = self.pool.acquire().await.map_err(|err| {
            error!("MFA config send code: failed to acquire connection: {err}");
            Status::internal("unexpected error")
        })?;
        mfa_code_mail(&user.email, &mut conn, &user.first_name, &code, None, true)
            .await
            .map_err(|err| {
                error!("MFA config send code: failed to send email code: {err}");
                Status::internal("unexpected error")
            })?;
        info!(
            "Sent MFA configuration email code to user {}",
            user.username
        );

        Ok(MfaConfigSendCodeResponse {})
    }

    /// Authorizes an MFA configuration session with a TOTP or email code.
    ///
    /// Authorization turns the token into a setup session. The email fallback proves mailbox
    /// access, but only MFA setup enables a factor.
    #[instrument(skip_all)]
    pub(crate) async fn mfa_config_authorize(
        &self,
        request: MfaConfigAuthorizeRequest,
    ) -> Result<MfaConfigAuthorizeResponse, Status> {
        debug!("Authorizing MFA configuration session");
        let MfaConfigSession {
            mut token,
            user,
            totp_configured,
            email_configured,
        } = self.load_session(&request.session_token).await?;
        // With no configured factor, Email is the fallback authorization method.
        let email_fallback = !totp_configured && !email_configured;

        let method = MfaMethod::try_from(request.method).map_err(|_| {
            error!("MFA config authorize: unknown method {}", request.method);
            Status::invalid_argument("unknown method")
        })?;
        let allowed = match method {
            MfaMethod::Totp => totp_configured,
            MfaMethod::Email => email_configured || email_fallback,
            _ => {
                error!("MFA config authorize: method {method} cannot authorize a session");
                return Err(Status::invalid_argument("method cannot authorize"));
            }
        };
        if !allowed {
            error!(
                "MFA config authorize: user {} has no configured {method}",
                user.username
            );
            return Err(Status::permission_denied("method not configured"));
        }

        let valid = match method {
            MfaMethod::Totp => user.verify_totp_code(&request.code),
            _ => user.verify_email_mfa_code(&request.code),
        };
        if !valid {
            error!(
                "MFA config authorize: invalid {method} code for user {}",
                user.username
            );
            return Err(Status::unauthenticated("invalid code"));
        }

        let mut transaction = self.pool.begin().await.map_err(|err| {
            error!("MFA config authorize: failed to begin transaction: {err}");
            Status::internal("unexpected error")
        })?;
        let deadline = token
            .start_session(&mut transaction, MFA_CONFIG_SESSION_TIMEOUT.as_secs())
            .await?;
        transaction.commit().await.map_err(|err| {
            error!("MFA config authorize: failed to commit transaction: {err}");
            Status::internal("unexpected error")
        })?;

        info!(
            "User {} authorized MFA configuration with {method} (email fallback: {email_fallback})",
            user.username
        );

        Ok(MfaConfigAuthorizeResponse {
            deadline_timestamp: deadline.and_utc().timestamp(),
        })
    }
}
