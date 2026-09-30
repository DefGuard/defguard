use defguard_common::{
    db::{
        Id,
        models::{
            Device, MFAMethod, Settings, User, biometric_auth::BiometricChallenge,
            polling_token::PollingToken, vpn_client_session::VpnClientMfaMethod,
        },
    },
    random::gen_alphanumeric,
    types::AuthFlowType,
};
use defguard_core::{
    db::models::enrollment::{
        MFA_CONFIG_SESSION_TIMEOUT, MFA_CONFIG_TOKEN_TYPE, MfaConfigAuthState, Token, TokenError,
    },
    enterprise::{
        db::models::openid_provider::OpenIdProvider,
        handlers::openid_login::{
            ClaimsUserResolution, MfaOidcState, extract_state_data, user_from_claims,
        },
        is_business_license_active, is_oidc_mfa_available,
    },
    events::{ApiEvent, ApiEventType},
    grpc::utils::parse_client_ip_agent,
    mail::templates::mfa_code_mail,
    mfa_engine::method::{VerifyError, fido2_credential_ids, verify_fido2_assertion},
};
use defguard_proto::{
    client_types::{
        MfaConfigAuthorizeRequest, MfaConfigAuthorizeResponse, MfaConfigEndRequest,
        MfaConfigFido2ChallengeRequest, MfaConfigFido2ChallengeResponse, MfaConfigSendCodeRequest,
        MfaConfigSendCodeResponse, MfaConfigStartRequest, MfaConfigStartResponse, MfaMethod,
    },
    proxy::{ClientMfaOidcAuthenticateRequest, DeviceInfo},
};
use openidconnect::{AuthorizationCode, Nonce};
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;
use tonic::Status;

use super::mfa_setup::finalize_mfa_factor;

/// An unauthorized MFA configuration session and its user's current factor state.
struct MfaConfigSession {
    token: Token,
    user: User<Id>,
    totp_configured: bool,
    email_configured: bool,
    fido2_configured: bool,
    oidc_configured: bool,
}

impl MfaConfigSession {
    /// With no method able to authorize, an email code is the fallback authorization method.
    fn email_fallback(&self) -> bool {
        !self.totp_configured
            && !self.email_configured
            && !self.fido2_configured
            && !self.oidc_configured
    }
}

/// Whether this setup lets OIDC authorize: a business license and a configured provider.
async fn oidc_available(pool: &PgPool) -> Result<bool, Status> {
    if !is_business_license_active() {
        return Ok(false);
    }
    let provider = OpenIdProvider::get_current(pool).await.map_err(|err| {
        error!("MFA config: failed to read the OpenID provider: {err}");
        Status::internal("unexpected error")
    })?;
    Ok(is_oidc_mfa_available(provider.is_some()))
}

async fn is_configured(
    pool: &PgPool,
    method: VpnClientMfaMethod,
    user: &User<Id>,
    device_id: Id,
    smtp_configured: bool,
    oidc_available: bool,
) -> Result<bool, Status> {
    method
        .is_configured(pool, user, Some(device_id), smtp_configured, oidc_available)
        .await
        .map_err(|err| {
            error!(
                "MFA config: failed to read {method:?} state for user {}: {err}",
                user.username
            );
            Status::internal("unexpected error")
        })
}

async fn is_mfa_config_token(pool: &PgPool, token: &str) -> Result<bool, Status> {
    match Token::find_by_id(pool, token).await {
        Ok(token) => Ok(token.token_type.as_deref() == Some(MFA_CONFIG_TOKEN_TYPE)),
        Err(TokenError::NotFound) => Ok(false),
        Err(err) => Err(err.into()),
    }
}

/// Loads an unauthorized MFA configuration session and its user's current factor state.
async fn load_session(pool: &PgPool, session_token: &str) -> Result<MfaConfigSession, Status> {
    let token = Token::find_by_id(pool, session_token).await?;
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
    let user = token.fetch_user(pool).await?;
    if !user.is_active {
        error!("MFA config: user {} is inactive", user.username);
        return Err(Status::permission_denied("user is inactive"));
    }
    let Some(device_id) = token.device_id else {
        error!("MFA config: token {} has no device", token.id);
        return Err(Status::internal("unexpected error"));
    };

    let smtp_configured = Settings::get_current_settings().smtp_configured();
    let oidc_available = user.openid_sub.is_some() && oidc_available(pool).await?;
    // Biometric and mobile-approve are mobile-only, carry no recovery codes and never
    // reach this flow, so they stay ignored.
    let configured = async |method| {
        is_configured(
            pool,
            method,
            &user,
            device_id,
            smtp_configured,
            oidc_available,
        )
        .await
    };
    let totp_configured = configured(VpnClientMfaMethod::Totp).await?;
    let email_configured = configured(VpnClientMfaMethod::Email).await?;
    let fido2_configured = configured(VpnClientMfaMethod::Fido2).await?;
    let oidc_configured = configured(VpnClientMfaMethod::Oidc).await?;

    Ok(MfaConfigSession {
        token,
        user,
        totp_configured,
        email_configured,
        fido2_configured,
        oidc_configured,
    })
}

/// Starts an OIDC attempt when `token` names an MFA configuration session. Returns the state
/// payload binding the callback to it, or `None` for another flow's token.
pub(crate) async fn mfa_config_oidc_begin(
    pool: &PgPool,
    token: &str,
) -> Result<Option<String>, Status> {
    if !is_mfa_config_token(pool, token).await? {
        return Ok(None);
    }
    let mut session = load_session(pool, token).await?;
    if !session.oidc_configured {
        error!(
            "MFA config OIDC begin: user {} cannot use OIDC",
            session.user.username
        );
        return Err(Status::permission_denied("method not configured"));
    }
    let attempt_id = gen_alphanumeric(16);
    if !session
        .token
        .replace_mfa_config_auth_state(
            pool,
            &MfaConfigAuthState::Oidc {
                attempt_id: attempt_id.clone(),
                completed: false,
            },
        )
        .await?
    {
        debug!(
            "MFA config OIDC begin: user {} already completed OIDC authentication",
            session.user.username
        );
        return Err(Status::failed_precondition(
            "OIDC authentication already completed",
        ));
    }
    debug!(
        "User {} started an OIDC MFA configuration authorization",
        session.user.username
    );

    Ok(Some(MfaOidcState::build(token, &attempt_id)))
}

/// Decodes an OIDC callback state, returning it only when it names an MFA configuration session.
pub(crate) async fn mfa_config_oidc_state(
    pool: &PgPool,
    state: &str,
) -> Result<Option<MfaOidcState>, Status> {
    let Some(state) = extract_state_data(state).and_then(|data| MfaOidcState::parse(&data)) else {
        return Ok(None);
    };
    if is_mfa_config_token(pool, &state.token).await? {
        Ok(Some(state))
    } else {
        Ok(None)
    }
}

/// Ends the session of a failed OIDC attempt, unless a newer attempt or session replaced it
/// during the OIDC round trip.
async fn end_oidc_attempt(pool: &PgPool, token: &Token, attempt_id: &str) -> Result<(), Status> {
    if !token
        .delete_pending_mfa_config_oidc_attempt(pool, attempt_id)
        .await?
    {
        debug!("MFA config OIDC: attempt superseded during the OIDC round trip");
    }
    Ok(())
}

/// Handles MFA factor configuration requested by an already enrolled desktop client.
pub(crate) struct MfaConfigServer {
    pool: PgPool,
    event_tx: UnboundedSender<ApiEvent>,
}

impl MfaConfigServer {
    #[must_use]
    pub fn new(pool: PgPool, event_tx: UnboundedSender<ApiEvent>) -> Self {
        Self { pool, event_tx }
    }

    /// Starts an MFA configuration session for the device identified by the polling token.
    ///
    /// The returned token remains unused until the user proves an existing factor.
    /// The client requests an email code with `mfa_config_send_code` and a FIDO2 challenge with
    /// `mfa_config_fido2_challenge`; OIDC runs through Edge's OpenID MFA page.
    /// When no method can authorize, an email code is the only authorization method.
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
        let oidc_available = user.openid_sub.is_some() && oidc_available(&self.pool).await?;
        let mut available_methods = Vec::with_capacity(4);
        for method in [
            VpnClientMfaMethod::Totp,
            VpnClientMfaMethod::Email,
            VpnClientMfaMethod::Fido2,
            VpnClientMfaMethod::Oidc,
        ] {
            if is_configured(
                &self.pool,
                method,
                &user,
                device.id,
                smtp_configured,
                oidc_available,
            )
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

        Token::delete_user_tokens_of_type(&mut *transaction, user.id, MFA_CONFIG_TOKEN_TYPE)
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

    /// Sends the email code used to authorize an MFA configuration session.
    ///
    /// The client calls this after the user selects Email, including the no-factor fallback.
    #[instrument(skip_all)]
    pub(crate) async fn mfa_config_send_code(
        &self,
        request: MfaConfigSendCodeRequest,
    ) -> Result<MfaConfigSendCodeResponse, Status> {
        debug!("Sending MFA configuration email code");
        let session = load_session(&self.pool, &request.session_token).await?;
        let user = &session.user;
        if !session.email_configured && !session.email_fallback() {
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

    /// Issues the single-use challenge for authorizing an MFA configuration session with FIDO2.
    #[instrument(skip_all)]
    pub(crate) async fn mfa_config_fido2_challenge(
        &self,
        request: MfaConfigFido2ChallengeRequest,
    ) -> Result<MfaConfigFido2ChallengeResponse, Status> {
        debug!("Issuing MFA configuration FIDO2 challenge");
        let MfaConfigSession {
            mut token,
            user,
            fido2_configured,
            ..
        } = load_session(&self.pool, &request.session_token).await?;
        if !fido2_configured {
            error!(
                "MFA config FIDO2 challenge: user {} has no security key",
                user.username
            );
            return Err(Status::permission_denied("method not configured"));
        }

        let challenge = BiometricChallenge::new().challenge;
        if !token
            .replace_mfa_config_auth_state(
                &self.pool,
                &MfaConfigAuthState::Fido2 {
                    challenge: challenge.clone(),
                },
            )
            .await?
        {
            debug!(
                "MFA config FIDO2 challenge: user {} already completed OIDC authentication",
                user.username
            );
            return Err(Status::failed_precondition(
                "OIDC authentication already completed",
            ));
        }
        let credential_ids = fido2_credential_ids(&self.pool, user.id)
            .await
            .map_err(|err| {
                error!("MFA config FIDO2 challenge: failed to read security keys: {err}");
                Status::internal("unexpected error")
            })?;
        info!(
            "Issued MFA configuration FIDO2 challenge to user {}",
            user.username
        );

        Ok(MfaConfigFido2ChallengeResponse {
            challenge,
            credential_ids,
        })
    }

    /// Authorizes an MFA configuration session with a TOTP or email code, a FIDO2 assertion
    /// or a completed OIDC authentication.
    ///
    /// Authorization turns the token into a setup session. In the no-factor email fallback
    /// a valid code also enables the email factor and returns recovery codes. For OIDC the
    /// client polls this until the Edge callback has completed the attempt.
    #[instrument(skip_all)]
    pub(crate) async fn mfa_config_authorize(
        &self,
        request: MfaConfigAuthorizeRequest,
        device_info: Option<DeviceInfo>,
    ) -> Result<MfaConfigAuthorizeResponse, Status> {
        debug!("Authorizing MFA configuration session");
        let session = load_session(&self.pool, &request.session_token).await?;
        let email_fallback = session.email_fallback();
        let MfaConfigSession {
            mut token,
            mut user,
            totp_configured,
            email_configured,
            fido2_configured,
            oidc_configured,
        } = session;

        let method = MfaMethod::try_from(request.method).map_err(|_| {
            error!("MFA config authorize: unknown method {}", request.method);
            Status::invalid_argument("unknown method")
        })?;
        let allowed = match method {
            MfaMethod::Totp => totp_configured,
            MfaMethod::Email => email_configured || email_fallback,
            MfaMethod::Fido2 => fido2_configured,
            MfaMethod::Oidc => oidc_configured,
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
            MfaMethod::Fido2 => {
                // Taking the challenge consumes it, so every attempt needs a fresh one.
                let Some(challenge) = token.take_fido2_challenge(&self.pool).await? else {
                    error!(
                        "MFA config authorize: user {} has no pending FIDO2 challenge",
                        user.username
                    );
                    return Err(Status::failed_precondition("no FIDO2 challenge"));
                };
                verify_fido2_assertion(
                    &self.pool,
                    user.id,
                    &challenge,
                    request.signature.as_deref(),
                    request.auth_data.as_deref(),
                    request.credential_id.as_deref(),
                )
                .await
                .map_err(|err| {
                    error!("MFA config authorize: failed to verify FIDO2 assertion: {err}");
                    match err {
                        VerifyError::MalformedProof { message, .. } => {
                            Status::invalid_argument(message)
                        }
                        _ => Status::internal("unexpected error"),
                    }
                })?
            }
            MfaMethod::Oidc => {
                if !matches!(
                    token.get_mfa_config_auth_state(),
                    Some(MfaConfigAuthState::Oidc {
                        completed: true,
                        ..
                    })
                ) {
                    debug!(
                        "MFA config authorize: OIDC authentication of user {} not completed yet",
                        user.username
                    );
                    return Err(Status::failed_precondition(
                        "OIDC authentication not completed",
                    ));
                }
                true
            }
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
        let deadline = match token
            .authorize_mfa_config_session(&mut transaction, MFA_CONFIG_SESSION_TIMEOUT.as_secs())
            .await
        {
            Ok(deadline) => deadline,
            Err(TokenError::TokenUsed) => {
                error!(
                    "MFA config authorize: a concurrent request already authorized user {}",
                    user.username
                );
                return Err(Status::failed_precondition("session already authorized"));
            }
            Err(err) => return Err(err.into()),
        };

        // In the fallback the verified code also enables email MFA; otherwise authorization
        // only opens the setup session and configuring a factor is a separate step.
        let recovery_codes = if email_fallback {
            user.enable_email_mfa(&mut *transaction)
                .await
                .map_err(|err| {
                    error!("MFA config authorize: failed to enable email MFA: {err}");
                    Status::internal("unexpected error")
                })?;
            finalize_mfa_factor(
                &self.pool,
                &self.event_tx,
                transaction,
                &mut user,
                MFAMethod::Email,
                ApiEventType::MfaEmailEnabled,
                device_info,
                // Email is the user's first factor here, so issue fresh recovery codes.
                true,
            )
            .await?
        } else {
            transaction.commit().await.map_err(|err| {
                error!("MFA config authorize: failed to commit transaction: {err}");
                Status::internal("unexpected error")
            })?;
            Vec::new()
        };

        info!(
            "User {} authorized MFA configuration with {method} (email fallback: {email_fallback})",
            user.username
        );

        Ok(MfaConfigAuthorizeResponse {
            deadline_timestamp: deadline.and_utc().timestamp(),
            recovery_codes,
        })
    }

    /// Completes an OIDC authorization attempt from the Edge callback.
    ///
    /// The session itself is authorized by the client's next `mfa_config_authorize` poll.
    /// A failed or foreign-identity authentication ends the session while its attempt is current.
    #[instrument(skip_all)]
    pub(crate) async fn mfa_config_oidc_authenticate(
        &self,
        state: MfaOidcState,
        request: ClientMfaOidcAuthenticateRequest,
        device_info: Option<DeviceInfo>,
    ) -> Result<(), Status> {
        debug!("Completing MFA configuration OIDC authentication");
        if !is_business_license_active() {
            error!("MFA config OIDC: OIDC MFA requires an active business license");
            return Err(Status::invalid_argument("OIDC MFA method is not supported"));
        }
        let MfaConfigSession {
            mut token,
            user,
            oidc_configured,
            ..
        } = load_session(&self.pool, &state.token).await?;
        if !oidc_configured {
            error!("MFA config OIDC: user {} cannot use OIDC", user.username);
            return Err(Status::permission_denied("method not configured"));
        }
        // Turn a callback for a superseded attempt away before anything can end the session.
        if !matches!(
            token.get_mfa_config_auth_state(),
            Some(MfaConfigAuthState::Oidc { attempt_id, completed: false })
                if attempt_id == state.attempt_id
        ) {
            debug!("MFA config OIDC: callback arrived for a superseded attempt");
            return Err(Status::invalid_argument("stale OIDC MFA attempt"));
        }

        let (ip, user_agent) = parse_client_ip_agent(&device_info).map_err(|err| {
            error!("MFA config OIDC: failed to parse client IP and agent: {err}");
            Status::internal("unexpected error")
        })?;
        let url = Settings::get_current_settings()
            .edge_callback_url(AuthFlowType::Mfa)
            .map_err(|err| {
                error!("MFA config OIDC: invalid callback URL configuration: {err}");
                Status::invalid_argument("invalid callback URL")
            })?;
        // Re-verifies an existing link, so it must neither link nor create an account.
        let claims_user = match user_from_claims(
            &self.pool,
            Nonce::new(request.nonce),
            AuthorizationCode::new(request.code),
            url,
            Some(ip),
            Some(&user_agent),
            None,
            ClaimsUserResolution::LookupOnly,
        )
        .await
        {
            Ok(claims_user) => claims_user,
            Err(err) => {
                info!(
                    "MFA config OIDC: failed to verify OIDC code for user {}: {err}",
                    user.username
                );
                end_oidc_attempt(&self.pool, &token, &state.attempt_id).await?;
                return Err(Status::unauthenticated("unauthorized"));
            }
        };
        if claims_user.id != user.id {
            info!(
                "User {claims_user} tried to authorize MFA configuration for another user: {user}"
            );
            end_oidc_attempt(&self.pool, &token, &state.attempt_id).await?;
            return Err(Status::unauthenticated("unauthorized"));
        }

        // The attempt is checked again under a row lock, catching one re-issued mid round trip.
        if !token
            .mark_mfa_config_oidc_completed(&self.pool, &state.attempt_id)
            .await?
        {
            debug!("MFA config OIDC: attempt superseded during the OIDC round trip");
            return Err(Status::invalid_argument("stale OIDC MFA attempt"));
        }
        info!(
            "User {} completed OIDC authentication for MFA configuration",
            user.username
        );

        Ok(())
    }

    /// Ends the user's MFA configuration session.
    #[instrument(skip_all)]
    pub(crate) async fn mfa_config_end(&self, request: MfaConfigEndRequest) -> Result<(), Status> {
        let token = Token::find_by_id(&self.pool, &request.session_token).await?;
        // Only MFA configuration tokens may be deleted here.
        if token.token_type.as_deref() != Some(MFA_CONFIG_TOKEN_TYPE) {
            error!(
                "MFA config end: token {} has type {:?}, expected {MFA_CONFIG_TOKEN_TYPE}",
                token.id, token.token_type
            );
            return Err(Status::permission_denied("invalid token"));
        }
        Token::delete_user_tokens_of_type(&self.pool, token.user_id, MFA_CONFIG_TOKEN_TYPE).await?;
        info!("Ended MFA configuration session for user {}", token.user_id);

        Ok(())
    }
}
