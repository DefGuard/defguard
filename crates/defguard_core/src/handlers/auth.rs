use std::net::IpAddr;

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_client_ip::InsecureClientIp;
use axum_extra::{
    TypedHeader,
    extract::{
        PrivateCookieJar,
        cookie::{Cookie, CookieJar, SameSite},
    },
    headers::UserAgent,
};
use defguard_common::{
    db::{
        Id,
        models::{MFAInfo, MFAMethod, Session, SessionState, Settings, User, WebAuthn},
    },
    types::user_info::UserInfo,
};
use defguard_mail::templates::mfa_code_mail;
use sqlx::{PgPool, types::Uuid};
use time::Duration;
use uaparser::Parser;
use webauthn_rs::prelude::PublicKeyCredential;
use webauthn_rs_proto::options::CollectedClientData;

use super::{
    ApiResponse, ApiResult, Auth, AuthCode, AuthResponse, AuthTotp, RecoveryCode, RecoveryCodes,
    SESSION_COOKIE_NAME, WebAuthnRegistration,
};
use crate::{
    appstate::AppState,
    auth::{
        SessionExtractor, SessionInfo,
        failed_login::{check_failed_logins, log_failed_login_attempt},
    },
    enterprise::ldap::utils::login_through_ldap,
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{
        SIGN_IN_COOKIE_NAME,
        mail::{send_email_mfa_activation_email, send_mfa_configured_email},
        user_for_admin_or_self,
    },
    headers::{USER_AGENT_PARSER, check_new_device_login, get_user_agent_device},
    server_config,
};

/// Common functionality for `authenticate()` and `auth_callback()`.
/// Returns either `AuthResponse` or `MFAInfo`.
pub async fn create_session(
    pool: &PgPool,
    ip_address: IpAddr,
    user_agent: &str,
    user: &mut User<Id>,
) -> Result<(Session, Option<UserInfo>, Option<MFAInfo>), WebError> {
    let agent = USER_AGENT_PARSER.parse(user_agent);
    let device_info = get_user_agent_device(&agent);
    debug!("Cleaning up expired sessions...");
    Session::delete_expired(pool).await?;
    debug!("Expired sessions cleaned up");

    debug!("Creating new session for user {}", user.username);
    let session = Session::new(
        user.id,
        SessionState::PasswordVerified,
        ip_address.to_string(),
        Some(device_info),
    );
    session.save(pool).await?;
    debug!("New session created for user {}", user.username);

    let login_event_type = "AUTHENTICATION".to_string();

    // Check that MFA state is correct before proceeding further
    user.verify_mfa_state(pool).await?;

    info!("Authenticated user {}", user.username);
    if user.mfa_enabled {
        debug!(
            "User {} has MFA enabled, sending MFA info for further authentication.",
            user.username
        );
        if let Some(mfa_info) = MFAInfo::for_user(pool, user).await? {
            check_new_device_login(
                pool,
                &session.clone().into(),
                user,
                ip_address.to_string(),
                login_event_type,
                agent,
            )
            .await?;
            Ok((session, None, Some(mfa_info)))
        } else {
            error!(
                "Couldn't fetch MFA info for user {} with MFA enabled",
                user.username
            );
            Err(WebError::DbError("MFA info read error".into()))
        }
    } else {
        debug!(
            "User {} has MFA disabled, returning user info for login.",
            user.username
        );
        let user_info = UserInfo::from_user(pool, user).await?;

        check_new_device_login(
            pool,
            &session.clone().into(),
            user,
            ip_address.to_string(),
            login_event_type,
            agent,
        )
        .await?;

        Ok((session, Some(user_info), None))
    }
}

/// Authenticate a user.
///
/// # For successful login, returns:
/// * 200 with MFA disabled
/// * 201 with MFA enabled when additional authentication factor is required
#[utoipa::path(
    post,
    path = "/api/v1/auth",
    responses(
        (status = OK, description = "User authenticated"),
        (status = CREATED, description = "User authenticated, but an additional authentication factor is required"),
    ),
)]
pub(crate) async fn authenticate(
    cookies: CookieJar,
    mut private_cookies: PrivateCookieJar,
    user_agent: TypedHeader<UserAgent>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
    Json(data): Json<Auth>,
) -> Result<(CookieJar, PrivateCookieJar, ApiResponse), WebError> {
    let username_or_email = data.username;
    debug!("Authenticating user {username_or_email}");

    // check if user can proceed with login
    check_failed_logins(&appstate.failed_logins, &username_or_email)?;

    let settings = Settings::get_current_settings();

    // Attempt to find a user: first by username, and then by email.
    let mut conn = appstate.pool.acquire().await?;
    let mut user = if let Some(user) =
        User::find_by_username_or_email(&mut conn, &username_or_email).await?
    {
        // user was found, attempt to authenticate by password first
        match user.verify_password(&data.password) {
            Ok(()) => user,
            Err(err) => {
                // password authentication failed, try authenticating with LDAP if configured
                if settings.ldap_enabled {
                    match login_through_ldap(&appstate.pool, &username_or_email, &data.password)
                        .await
                    {
                        Ok(user) => user,
                        Err(ldap_err) => {
                            warn!(
                                "Failed to authenticate user {username_or_email} internally and through LDAP. Internal error: {err}, LDAP error: {ldap_err}"
                            );

                            log_failed_login_attempt(&appstate.failed_logins, &user.username);
                            appstate.emit_event(ApiEvent {
                            context: ApiRequestContext::new(
                                user.id,
                                user.username,
                                insecure_ip,
                                user_agent.to_string(),
                            ),
                            event: Box::new(ApiEventType::UserLoginFailed {
                                message: format!(
                                    "Internal and LDAP authentication for {username_or_email} failed. Internal error: {err}, LDAP error: {ldap_err}"
                                ),
                            }),
                        })?;
                            return Err(WebError::Authentication);
                        }
                    }
                } else {
                    warn!("Failed to authenticate user {username_or_email}: {err}");
                    log_failed_login_attempt(&appstate.failed_logins, &user.username);
                    appstate.emit_event(ApiEvent {
                        context: ApiRequestContext::new(
                            user.id,
                            user.username,
                            insecure_ip,
                            user_agent.to_string(),
                        ),
                        event: Box::new(ApiEventType::UserLoginFailed {
                            message: format!(
                                "Authentication for {username_or_email} failed: {err}"
                            ),
                        }),
                    })?;
                    return Err(WebError::Authentication);
                }
            }
        }
    } else {
        // try to create user from LDAP
        debug!("User not found in DB, authenticating user {username_or_email} with LDAP");
        match login_through_ldap(&appstate.pool, &username_or_email, &data.password).await {
            Ok(user) => user,
            Err(err) => {
                info!("Failed to authenticate user {username_or_email} with LDAP: {err}");
                log_failed_login_attempt(&appstate.failed_logins, &username_or_email);
                return Err(WebError::Authentication);
            }
        }
    };

    // check if user account is active
    if !user.is_active {
        info!("Failed to authenticate user {username_or_email}: user is disabled");
        return Err(WebError::Authentication);
    }

    let (session, user_info, mfa_info) =
        create_session(&appstate.pool, insecure_ip, user_agent.as_str(), &mut user).await?;

    let max_age = Duration::seconds(server_config().auth_cookie_timeout.as_secs() as i64);
    let config = server_config();
    let cookie_domain = config
        .cookie_domain
        .as_ref()
        .expect("Cookie domain not found");
    let auth_cookie = Cookie::build((SESSION_COOKIE_NAME, session.id.clone()))
        .domain(cookie_domain)
        .path("/")
        .http_only(true)
        .secure(!config.cookie_insecure)
        .same_site(SameSite::Lax)
        .max_age(max_age);
    let cookies = cookies.add(auth_cookie);

    if let Some(mfa_info) = mfa_info {
        return Ok((
            cookies,
            private_cookies,
            ApiResponse::json(mfa_info, StatusCode::CREATED),
        ));
    }

    if let Some(user_info) = user_info {
        let url = if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
            debug!("Found OpenID session cookie, returning the redirect URL stored in it.");
            let url = openid_cookie.value().to_string();
            private_cookies = private_cookies.remove(openid_cookie);
            Some(url)
        } else {
            debug!("No OpenID session found, proceeding with login to Defguard.");
            None
        };

        appstate.emit_event(ApiEvent {
            context: ApiRequestContext::new(
                user_info.id,
                user_info.username.clone(),
                insecure_ip,
                user_agent.to_string(),
            ),
            event: Box::new(ApiEventType::UserLogin),
        })?;

        Ok((
            cookies,
            private_cookies,
            ApiResponse::json(
                AuthResponse {
                    user: user_info,
                    url,
                },
                StatusCode::OK,
            ),
        ))
    } else {
        unimplemented!("Impossible to get here");
    }
}

/// Logout - forget the session cookie.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = OK, description = "User logged out"),
    ),
)]
pub(crate) async fn logout(
    cookies: CookieJar,
    SessionExtractor(session): SessionExtractor,
    user_agent: TypedHeader<UserAgent>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
) -> Result<(CookieJar, ApiResponse), WebError> {
    // remove auth cookie
    let cookies = cookies.remove(Cookie::from(SESSION_COOKIE_NAME));
    let user = User::find_by_id(&appstate.pool, session.user_id)
        .await?
        .ok_or_else(|| WebError::BadRequest(format!("User {} does not exist", session.user_id)))?;
    // remove stored session
    session.delete(&appstate.pool).await?;

    appstate.emit_event(ApiEvent {
        // User may not be fully authenticated so we can't use
        // context extractor in this handler since it requires
        // the `SessionInfo` object.
        context: ApiRequestContext::new(
            user.id,
            user.username,
            insecure_ip,
            user_agent.to_string(),
        ),
        event: Box::new(ApiEventType::UserLogout),
    })?;

    Ok((cookies, ApiResponse::default()))
}

/// Enable MFA
pub async fn mfa_enable(
    cookies: CookieJar,
    SessionExtractor(_session): SessionExtractor,
    session_info: SessionInfo,
    State(appstate): State<AppState>,
) -> Result<(CookieJar, ApiResponse), WebError> {
    let mut user = session_info.user;
    debug!("Enabling MFA for user {}", user.username);
    user.enable_mfa(&appstate.pool).await?;
    if user.mfa_enabled {
        info!("Enabled MFA for user {}", user.username);
        let cookies = cookies.remove(Cookie::from("defguard_sesssion"));
        user.logout_all_sessions(&appstate.pool).await?;
        debug!(
            "Removed auth sessions for user {} after enabling MFA",
            user.username
        );
        Ok((cookies, ApiResponse::default()))
    } else {
        error!("Error enabling MFA for user {}", user.username);
        Err(WebError::Http(StatusCode::NOT_MODIFIED))
    }
}

/// Disable own MFA
pub async fn mfa_disable(
    session_info: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
) -> ApiResult {
    let mut user = session_info.user;
    debug!("Disabling MFA for user {}", user.username);
    user.disable_mfa(&appstate.pool).await?;
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::MfaDisabled),
    })?;
    info!("Disabled MFA for user {}", user.username);
    Ok(ApiResponse::default())
}

/// Disable specific user's MFA
pub async fn disable_user_mfa(
    session_info: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    let mut user = user_for_admin_or_self(&appstate.pool, &session_info, &username).await?;
    debug!("Disabling MFA for user {}", user.username);
    user.disable_mfa(&appstate.pool).await?;
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserMfaDisabled { user }),
    })?;
    info!("Disabled MFA for user {username}");
    Ok(ApiResponse::default())
}

/// Initialize WebAuthn registration
pub async fn webauthn_init(
    mut session_info: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    let user = session_info.user;
    info!(
        "Initializing WebAuthn registration for user {}",
        user.username
    );
    // passkeys to exclude
    let passkeys = WebAuthn::passkeys_for_user(&appstate.pool, user.id).await?;
    match appstate.webauthn.start_passkey_registration(
        Uuid::new_v4(),
        &user.username,
        &user.username,
        Some(passkeys.iter().map(|key| key.cred_id().clone()).collect()),
    ) {
        Ok((ccr, passkey_reg)) => {
            session_info
                .session
                .set_passkey_registration(&appstate.pool, &passkey_reg)
                .await?;
            info!(
                "Initialized WebAuthn registration for user {}",
                user.username
            );
            Ok(ApiResponse::json(ccr, StatusCode::OK))
        }
        Err(err) => Err(WebError::WebauthnRegistration(err.to_string())),
    }
}

/// Finish WebAuthn registration
pub async fn webauthn_finish(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(webauth_reg): Json<WebAuthnRegistration>,
) -> ApiResult {
    info!(
        "Finishing WebAuthn registration for user {}",
        session.user.username
    );
    let passkey_reg =
        session
            .session
            .get_passkey_registration()
            .ok_or(WebError::WebauthnRegistration(
                "Passkey registration session not found".into(),
            ))?;

    let ccdj: CollectedClientData = serde_json::from_slice(
        webauth_reg.rpkc.response.client_data_json.as_ref(),
    )
    .map_err(|_| {
        WebError::WebauthnRegistration("Failed to parse passkey registration request data".into())
    })?;
    info!(
        "Passkey registration request origin: {}",
        ccdj.origin.to_string()
    );
    info!(
        "Allowed origins: {:?}",
        appstate
            .webauthn
            .get_allowed_origins()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    );

    let passkey = appstate
        .webauthn
        .finish_passkey_registration(&webauth_reg.rpkc, &passkey_reg)
        .map_err(|err| WebError::WebauthnRegistration(err.to_string()))?;
    let mut user = User::find_by_id(&appstate.pool, session.session.user_id)
        .await?
        .ok_or(WebError::WebauthnRegistration("User not found".into()))?;
    let recovery_codes = RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
    let webauthn = WebAuthn::new(session.session.user_id, webauth_reg.name, &passkey)?
        .save(&appstate.pool)
        .await?;
    if user.mfa_method == MFAMethod::None {
        send_mfa_configured_email(Some(&session.session.into()), &user, &MFAMethod::Webauthn)?;
        user.set_mfa_method(&appstate.pool, MFAMethod::Webauthn)
            .await?;
    }

    info!("Finished Webauthn registration for user {}", user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::MfaSecurityKeyAdded { key: webauthn }),
    })?;

    Ok(ApiResponse::json(recovery_codes, StatusCode::OK))
}

/// Start WebAuthn authentication
pub async fn webauthn_start(
    SessionExtractor(mut session): SessionExtractor,
    State(appstate): State<AppState>,
) -> ApiResult {
    let passkeys = WebAuthn::passkeys_for_user(&appstate.pool, session.user_id).await?;

    match appstate.webauthn.start_passkey_authentication(&passkeys) {
        Ok((rcr, passkey_reg)) => {
            session
                .set_passkey_authentication(&appstate.pool, &passkey_reg)
                .await?;
            Ok(ApiResponse::json(rcr, StatusCode::OK))
        }
        Err(_err) => Err(WebError::Http(StatusCode::BAD_REQUEST)),
    }
}

/// Finish WebAuthn authentication
pub async fn webauthn_end(
    private_cookies: PrivateCookieJar,
    SessionExtractor(mut session): SessionExtractor,
    user_agent: TypedHeader<UserAgent>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
    Json(pubkey): Json<PublicKeyCredential>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    if let Some(passkey_auth) = session.get_passkey_authentication() {
        match appstate
            .webauthn
            .finish_passkey_authentication(&pubkey, &passkey_auth)
        {
            Ok(auth_result) => {
                if auth_result.needs_update() {
                    // Find `Passkey` and try to update its credentials
                    for mut webauthn in
                        WebAuthn::all_for_user(&appstate.pool, session.user_id).await?
                    {
                        if let Some(true) = webauthn.passkey()?.update_credential(&auth_result) {
                            webauthn.save(&appstate.pool).await?;
                        }
                    }
                }

                session
                    .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                    .await?;

                return if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await?
                {
                    let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
                    appstate.emit_event(ApiEvent {
                        // User may not be fully authenticated so we can't use
                        // context extractor in this handler since it requires
                        // the `SessionInfo` object.
                        context: ApiRequestContext::new(
                            user.id,
                            user.username,
                            insecure_ip,
                            user_agent.to_string(),
                        ),
                        event: Box::new(ApiEventType::UserMfaLogin {
                            mfa_method: MFAMethod::Webauthn,
                        }),
                    })?;

                    if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                        debug!("Found OpenID session cookie.");
                        let redirect_url = openid_cookie.value().to_string();
                        let private_cookies = private_cookies.remove(openid_cookie);
                        Ok((
                            private_cookies,
                            ApiResponse::json(
                                AuthResponse {
                                    user: user_info,
                                    url: Some(redirect_url),
                                },
                                StatusCode::OK,
                            ),
                        ))
                    } else {
                        Ok((
                            private_cookies,
                            ApiResponse::json(
                                AuthResponse {
                                    user: user_info,
                                    url: None,
                                },
                                StatusCode::OK,
                            ),
                        ))
                    }
                } else {
                    Ok((private_cookies, ApiResponse::default()))
                };
            }
            Err(err) => {
                // authentication failed, emit relevant event
                if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
                    appstate.emit_event(ApiEvent {
                        // User may not be fully authenticated so we can't use
                        // context extractor in this handler since it requires
                        // the `SessionInfo` object.
                        context: ApiRequestContext::new(
                            user.id,
                            user.username,
                            insecure_ip,
                            user_agent.to_string(),
                        ),
                        event: Box::new(ApiEventType::UserMfaLoginFailed {
                            mfa_method: MFAMethod::Webauthn,
                            message: format!("Passkey authentication failed: {err}"),
                        }),
                    })?;
                }
            }
        }
    }
    Err(WebError::Http(StatusCode::BAD_REQUEST))
}

/// Generate new TOTP secret
pub async fn totp_secret(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    let mut user = session.user;
    debug!("Generating new TOTP secret for user {}", user.username);

    let secret = user.new_totp_secret(&appstate.pool).await?;
    info!("Generated new TOTP secret for user {}", user.username);
    Ok(ApiResponse::json(AuthTotp::new(secret), StatusCode::OK))
}

/// Enable TOTP
pub async fn totp_enable(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<AuthCode>,
) -> ApiResult {
    let mut user = session.user;
    debug!("Enabling TOTP for user {}", user.username);
    if user.verify_totp_code(&data.code) {
        let recovery_codes = RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
        user.enable_totp(&appstate.pool).await?;
        if user.mfa_method == MFAMethod::None {
            send_mfa_configured_email(
                Some(&session.session.into()),
                &user,
                &MFAMethod::OneTimePassword,
            )?;
            user.set_mfa_method(&appstate.pool, MFAMethod::OneTimePassword)
                .await?;
        }

        info!("Enabled TOTP for user {}", user.username);
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::MfaTotpEnabled),
        })?;
        Ok(ApiResponse::json(recovery_codes, StatusCode::OK))
    } else {
        Err(WebError::ObjectNotFound("Invalid TOTP code".into()))
    }
}

/// Disable TOTP
pub async fn totp_disable(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    username: Path<String>,
) -> ApiResult {
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    debug!("Disabling TOTP for user {}", user.username);
    user.disable_totp(&appstate.pool).await?;
    user.verify_mfa_state(&appstate.pool).await?;
    info!("Disabled TOTP for user {}", user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::MfaTotpDisabled),
    })?;
    Ok(ApiResponse::default())
}

/// Validate one-time passcode
pub async fn totp_code(
    private_cookies: PrivateCookieJar,
    SessionExtractor(mut session): SessionExtractor,
    user_agent: TypedHeader<UserAgent>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
    Json(data): Json<AuthCode>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        let username = user.username.clone();
        // check if user can proceed with login
        check_failed_logins(&appstate.failed_logins, &username)?;

        debug!("Verifying TOTP for user {}", username);
        if user.totp_enabled && user.verify_totp_code(&data.code) {
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
            info!("Verified TOTP for user {username}");
            appstate.emit_event(ApiEvent {
                // User may not be fully authenticated so we can't use
                // context extractor in this handler since it requires
                // the `SessionInfo` object.
                context: ApiRequestContext::new(
                    user.id,
                    user.username,
                    insecure_ip,
                    user_agent.to_string(),
                ),
                event: Box::new(ApiEventType::UserMfaLogin {
                    mfa_method: MFAMethod::OneTimePassword,
                }),
            })?;
            if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                debug!("Found openid session cookie.");
                let redirect_url = openid_cookie.value().to_string();
                let private_cookies = private_cookies.remove(openid_cookie);
                Ok((
                    private_cookies,
                    ApiResponse::json(
                        AuthResponse {
                            user: user_info,
                            url: Some(redirect_url),
                        },
                        StatusCode::OK,
                    ),
                ))
            } else {
                Ok((
                    private_cookies,
                    ApiResponse::json(
                        AuthResponse {
                            user: user_info,
                            url: None,
                        },
                        StatusCode::OK,
                    ),
                ))
            }
        } else {
            let message = if user.totp_enabled {
                "TOTP code verification failed".to_string()
            } else {
                format!("TOTP authentication is disabled for {username}")
            };

            log_failed_login_attempt(&appstate.failed_logins, &username);

            appstate.emit_event(ApiEvent {
                // User may not be fully authenticated so we can't use
                // context extractor in this handler since it requires
                // the `SessionInfo` object.
                context: ApiRequestContext::new(
                    user.id,
                    user.username,
                    insecure_ip,
                    user_agent.to_string(),
                ),
                event: Box::new(ApiEventType::UserMfaLoginFailed {
                    mfa_method: MFAMethod::OneTimePassword,
                    message,
                }),
            })?;
            Err(WebError::Authorization("Invalid TOTP code".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("Invalid user".into()))
    }
}

/// Initialize email MFA setup
pub async fn email_mfa_init(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    // check if SMTP is configured
    let settings = Settings::get_current_settings();
    if !settings.smtp_configured() {
        error!("Unable to start email MFA configuration. SMTP is not configured.");
        return Err(WebError::EmailMfa("SMTP not configured".into()));
    }

    // generate TOTP secret
    let mut user = session.user;
    debug!("Generating new email MFA secret for user {}", user.username);
    user.new_email_secret(&appstate.pool).await?;
    info!("Generated new email MFA secret for user {}", user.username);

    // send email with code
    send_email_mfa_activation_email(&user, Some(&session.session.into()))?;

    Ok(ApiResponse::default())
}

/// Enable email MFA
pub async fn email_mfa_enable(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<AuthCode>,
) -> ApiResult {
    let mut user = session.user;
    debug!("Enabling email MFA for user {}", user.username);
    if user.verify_email_mfa_code(&data.code) {
        let recovery_codes = RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
        user.enable_email_mfa(&appstate.pool).await?;
        if user.mfa_method == MFAMethod::None {
            send_mfa_configured_email(Some(&session.session.into()), &user, &MFAMethod::Email)?;
            user.set_mfa_method(&appstate.pool, MFAMethod::Email)
                .await?;
        }

        info!("Enabled email MFA for user {}", user.username);
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::MfaEmailEnabled),
        })?;
        Ok(ApiResponse::json(recovery_codes, StatusCode::OK))
    } else {
        Err(WebError::ObjectNotFound("Invalid email code".into()))
    }
}

/// Disable email MFA
pub async fn email_mfa_disable(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    username: Path<String>,
) -> ApiResult {
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    debug!("Disabling email MFA for user {}", user.username);
    user.disable_email_mfa(&appstate.pool).await?;
    user.verify_mfa_state(&appstate.pool).await?;
    info!("Disabled email MFA for user {}", user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::MfaEmailDisabled),
    })?;
    Ok(ApiResponse::default())
}

/// Send email code to user
pub async fn request_email_mfa_code(
    SessionExtractor(session): SessionExtractor,
    State(appstate): State<AppState>,
) -> ApiResult {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        debug!("Sending email MFA code for user {}", user.username);
        if user.email_mfa_enabled {
            let mut transaction = appstate.pool.begin().await?;
            let code = user.generate_email_mfa_code()?;
            mfa_code_mail(
                &user.email,
                &mut transaction,
                &user.first_name,
                &code,
                Some(&session.into()),
            )
            .await?;
            info!("Sent email MFA code for user {}", user.username);
            Ok(ApiResponse::default())
        } else {
            Err(WebError::Authorization("Email MFA not enabled".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("Invalid user".into()))
    }
}

/// Validate email MFA code
pub async fn email_mfa_code(
    private_cookies: PrivateCookieJar,
    SessionExtractor(mut session): SessionExtractor,
    user_agent: TypedHeader<UserAgent>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
    Json(data): Json<AuthCode>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        let username = user.username.clone();

        // check if user can proceed with login
        check_failed_logins(&appstate.failed_logins, &username)?;

        debug!("Verifying email MFA code for user {}", username);
        if user.email_mfa_enabled && user.verify_email_mfa_code(&data.code) {
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
            info!("Verified email MFA code for user {username}");
            appstate.emit_event(ApiEvent {
                // User may not be fully authenticated so we can't use
                // context extractor in this handler since it requires
                // the `SessionInfo` object.
                context: ApiRequestContext::new(
                    user.id,
                    user.username,
                    insecure_ip,
                    user_agent.to_string(),
                ),
                event: Box::new(ApiEventType::UserMfaLogin {
                    mfa_method: MFAMethod::Email,
                }),
            })?;
            if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                debug!("Found OpenID session cookie.");
                let redirect_url = openid_cookie.value().to_string();
                let private_cookies = private_cookies.remove(openid_cookie);
                Ok((
                    private_cookies,
                    ApiResponse::json(
                        AuthResponse {
                            user: user_info,
                            url: Some(redirect_url),
                        },
                        StatusCode::OK,
                    ),
                ))
            } else {
                Ok((
                    private_cookies,
                    ApiResponse::json(
                        AuthResponse {
                            user: user_info,
                            url: None,
                        },
                        StatusCode::OK,
                    ),
                ))
            }
        } else {
            let message = if user.email_mfa_enabled {
                "Email code verification failed".to_string()
            } else {
                format!("Email code authentication is disabled for {username}")
            };

            log_failed_login_attempt(&appstate.failed_logins, &username);

            appstate.emit_event(ApiEvent {
                // User may not be fully authenticated so we can't use
                // context extractor in this handler since it requires
                // the `SessionInfo` object.
                context: ApiRequestContext::new(
                    user.id,
                    user.username,
                    insecure_ip,
                    user_agent.to_string(),
                ),
                event: Box::new(ApiEventType::UserMfaLoginFailed {
                    mfa_method: MFAMethod::Email,
                    message,
                }),
            })?;
            Err(WebError::Authorization("Invalid email MFA code".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("Invalid user".into()))
    }
}

/// Authenticate with a recovery code.
pub async fn recovery_code(
    private_cookies: PrivateCookieJar,
    SessionExtractor(mut session): SessionExtractor,
    user_agent: TypedHeader<UserAgent>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
    Json(recovery_code): Json<RecoveryCode>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    if let Some(mut user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        let username = user.username.clone();
        debug!("Authenticating user {username} with recovery code");
        if user
            .verify_recovery_code(&appstate.pool, &recovery_code.code)
            .await?
        {
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
            info!("Authenticated user {username} with recovery code");
            appstate.emit_event(ApiEvent {
                // User may not be fully authenticated so we can't use
                // context extractor in this handler since it requires
                // the `SessionInfo` object.
                context: ApiRequestContext::new(
                    user.id,
                    user.username,
                    insecure_ip,
                    user_agent.to_string(),
                ),
                event: Box::new(ApiEventType::RecoveryCodeUsed),
            })?;
            if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                debug!("Found OpenID session cookie.");
                let redirect_url = openid_cookie.value().to_string();
                let private_cookies = private_cookies.remove(openid_cookie);
                return Ok((
                    private_cookies,
                    ApiResponse::json(
                        AuthResponse {
                            user: user_info,
                            url: Some(redirect_url),
                        },
                        StatusCode::OK,
                    ),
                ));
            }

            return Ok((
                private_cookies,
                ApiResponse::json(
                    AuthResponse {
                        user: user_info,
                        url: None,
                    },
                    StatusCode::OK,
                ),
            ));
        }
    }
    Err(WebError::Http(StatusCode::UNAUTHORIZED))
}
