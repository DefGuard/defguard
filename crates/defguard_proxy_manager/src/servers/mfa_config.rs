use defguard_common::db::models::{
    Device, Settings, User, polling_token::PollingToken, vpn_client_session::VpnClientMfaMethod,
};
use defguard_core::{
    db::models::enrollment::{MFA_CONFIG_SESSION_TIMEOUT, MFA_CONFIG_TOKEN_TYPE, Token},
    mail::templates::mfa_activation_mail,
};
use defguard_proto::client_types::{MfaConfigStartRequest, MfaConfigStartResponse, MfaMethod};
use sqlx::PgPool;
use tonic::Status;

/// Handles MFA factor configuration requested by an already enrolled desktop client.
pub(crate) struct MfaConfigServer {
    pool: PgPool,
}

impl MfaConfigServer {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Starts an MFA configuration session for the device identified by the polling token.
    ///
    /// The session token is returned unauthorized; `used_at` stays `NULL` until the user
    /// proves an existing factor. When no factor is configured an email code is sent and Email
    /// MFA becomes the first factor once the session is authorized.
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
            let configured = method
                .is_configured(&self.pool, &user, device.id, smtp_configured, false)
                .await
                .map_err(|err| {
                    error!("MFA config start: failed to read MFA state: {err}");
                    Status::internal("unexpected error")
                })?;
            if configured {
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
            if user.email_mfa_secret.is_none() {
                user.new_email_secret(&mut *transaction)
                    .await
                    .map_err(|err| {
                        error!("MFA config start: failed to create email secret: {err}");
                        Status::internal("unexpected error")
                    })?;
            }
            let code = user.generate_email_mfa_code().map_err(|err| {
                error!("MFA config start: failed to generate email code: {err}");
                Status::internal("unexpected error")
            })?;
            mfa_activation_mail(
                &user.email,
                &mut transaction,
                &user.first_name,
                &code,
                None,
                true,
            )
            .await
            .map_err(|err| {
                error!("MFA config start: failed to send email code: {err}");
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
}
