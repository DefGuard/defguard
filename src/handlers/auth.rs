use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use axum_client_ip::{InsecureClientIp, LeftmostXForwardedFor};
use axum_extra::{
    extract::{
        cookie::{Cookie, CookieJar, SameSite},
        PrivateCookieJar,
    },
    headers::UserAgent,
    TypedHeader,
};
use serde_json::json;
use sqlx::types::Uuid;
use time::Duration;
use webauthn_rs::prelude::PublicKeyCredential;
use webauthn_rs_proto::options::CollectedClientData;

use super::{
    ApiResponse, ApiResult, Auth, AuthCode, AuthResponse, AuthTotp, RecoveryCode, RecoveryCodes,
    WalletAddress, WalletSignature, WebAuthnRegistration, SESSION_COOKIE_NAME,
};
use crate::{
    appstate::AppState,
    auth::{
        failed_login::{check_username, log_failed_login_attempt},
        SessionInfo,
    },
    db::{MFAInfo, MFAMethod, Session, SessionState, Settings, User, UserInfo, Wallet, WebAuthn},
    error::WebError,
    handlers::mail::{
        send_email_mfa_activation_email, send_email_mfa_code_email, send_mfa_configured_email,
    },
    handlers::SIGN_IN_COOKIE_NAME,
    headers::{check_new_device_login, get_user_agent_device, parse_user_agent},
    ldap::utils::user_from_ldap,
    SERVER_CONFIG,
};

/// For successful login, return:
/// * 200 with MFA disabled
/// * 201 with MFA enabled when additional authentication factor is required
pub async fn authenticate(
    cookies: CookieJar,
    private_cookies: PrivateCookieJar,
    user_agent: Option<TypedHeader<UserAgent>>,
    forwarded_for_ip: Option<LeftmostXForwardedFor>,
    InsecureClientIp(insecure_ip): InsecureClientIp,
    State(appstate): State<AppState>,
    Json(data): Json<Auth>,
) -> Result<(CookieJar, PrivateCookieJar, ApiResponse), WebError> {
    let lowercase_username = data.username.to_lowercase();
    debug!("Authenticating user {lowercase_username}");
    // check if user can proceed with login
    check_username(&appstate.failed_logins, &lowercase_username)?;

    let user = match User::find_by_username(&appstate.pool, &lowercase_username).await {
        Ok(Some(user)) => match user.verify_password(&data.password) {
            Ok(()) => user,
            Err(err) => {
                info!("Failed to authenticate user {lowercase_username}: {err}");
                log_failed_login_attempt(&appstate.failed_logins, &lowercase_username);
                return Err(WebError::Authorization(err.to_string()));
            }
        },
        Ok(None) => {
            // create user from LDAP
            debug!("User not found in DB, authenticating user {lowercase_username} with LDAP");
            if let Ok(user) =
                user_from_ldap(&appstate.pool, &lowercase_username, &data.password).await
            {
                user
            } else {
                info!("Failed to authenticate user {lowercase_username} with LDAP");
                log_failed_login_attempt(&appstate.failed_logins, &lowercase_username);
                return Err(WebError::Authorization("user not found".into()));
            }
        }
        Err(err) => {
            error!("DB error when authenticating user {lowercase_username}: {err}");
            return Err(WebError::DbError(err.to_string()));
        }
    };

    let ip_address = forwarded_for_ip.map_or(insecure_ip, |v| v.0).to_string();
    let user_agent_string = match user_agent {
        Some(value) => value.to_string(),
        None => String::new(),
    };
    let agent = parse_user_agent(&appstate.user_agent_parser, &user_agent_string);
    let device_info = agent.clone().map(|v| get_user_agent_device(&v));

    Session::delete_expired(&appstate.pool).await?;
    let session = Session::new(
        user.id.unwrap(),
        SessionState::PasswordVerified,
        ip_address.clone(),
        device_info,
    );
    session.save(&appstate.pool).await?;

    let max_age = match &appstate.config.session_auth_lifetime {
        Some(seconds) => Duration::seconds(*seconds),
        None => Duration::days(7),
    };

    let server_config = SERVER_CONFIG.get().ok_or(WebError::ServerConfigMissing)?;
    let auth_cookie = Cookie::build((SESSION_COOKIE_NAME, session.clone().id))
        .domain(
            server_config
                .cookie_domain
                .clone()
                .expect("Cookie domain not found"),
        )
        .path("/")
        .http_only(true)
        .secure(!server_config.cookie_insecure)
        .same_site(SameSite::Lax)
        .max_age(max_age);
    let cookies = cookies.add(auth_cookie);

    let login_event_type = "AUTHENTICATION".to_string();

    info!("Authenticated user {lowercase_username}");
    if user.mfa_enabled {
        if let Some(mfa_info) = MFAInfo::for_user(&appstate.pool, &user).await? {
            check_new_device_login(
                &appstate.pool,
                &appstate.mail_tx,
                &session,
                &user,
                ip_address,
                login_event_type,
                agent,
            )
            .await?;
            Ok((
                cookies,
                private_cookies,
                ApiResponse {
                    json: json!(mfa_info),
                    status: StatusCode::CREATED,
                },
            ))
        } else {
            Err(WebError::DbError("MFA info read error".into()))
        }
    } else {
        let user_info = UserInfo::from_user(&appstate.pool, &user).await?;

        check_new_device_login(
            &appstate.pool,
            &appstate.mail_tx,
            &session,
            &user,
            ip_address,
            login_event_type,
            agent,
        )
        .await?;

        if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
            debug!("Found openid session cookie.");
            let redirect_url = openid_cookie.value().to_string();
            Ok((
                cookies,
                private_cookies.remove(openid_cookie),
                ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: Some(redirect_url)
                    }),
                    status: StatusCode::OK,
                },
            ))
        } else {
            debug!("No OpenID session found");
            Ok((
                cookies,
                private_cookies,
                ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: None,
                    }),
                    status: StatusCode::OK,
                },
            ))
        }
    }
}

/// Logout - forget the session cookie.
pub async fn logout(
    cookies: CookieJar,
    session: Session,
    State(appstate): State<AppState>,
) -> Result<(CookieJar, ApiResponse), WebError> {
    // remove auth cookie
    let cookies = cookies.remove(Cookie::from(SESSION_COOKIE_NAME));
    // remove stored session
    session.delete(&appstate.pool).await?;
    Ok((cookies, ApiResponse::default()))
}

/// Enable MFA
pub async fn mfa_enable(
    cookies: CookieJar,
    session: Session,
    session_info: SessionInfo,
    State(appstate): State<AppState>,
) -> Result<(CookieJar, ApiResponse), WebError> {
    let mut user = session_info.user;
    debug!("Enabling MFA for user {}", user.username);
    user.enable_mfa(&appstate.pool).await?;
    if user.mfa_enabled {
        info!("Enabled MFA for user {}", user.username);
        let cookies = cookies.remove(Cookie::from("defguard_sesssion"));
        session.delete(&appstate.pool).await?;
        debug!(
            "Removed auth session for user {} after enabling MFA",
            user.username
        );
        Ok((cookies, ApiResponse::default()))
    } else {
        error!("Error enabling MFA for user {}", user.username);
        Err(WebError::Http(StatusCode::NOT_MODIFIED))
    }
}

/// Disable MFA
pub async fn mfa_disable(session_info: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    let mut user = session_info.user;
    debug!("Disabling MFA for user {}", user.username);
    user.disable_mfa(&appstate.pool).await?;
    info!("Disabled MFA for user {}", user.username);
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
    let passkeys =
        WebAuthn::passkeys_for_user(&appstate.pool, user.id.expect("User ID missing")).await?;
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
            Ok(ApiResponse {
                json: json!(ccr),
                status: StatusCode::OK,
            })
        }
        Err(err) => Err(WebError::WebauthnRegistration(err.to_string())),
    }
}

/// Finish WebAuthn registration
pub async fn webauthn_finish(
    session: SessionInfo,
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
            .collect::<Vec<String>>()
    );

    let passkey = appstate
        .webauthn
        .finish_passkey_registration(&webauth_reg.rpkc, &passkey_reg)
        .map_err(|err| WebError::WebauthnRegistration(err.to_string()))?;
    let mut user = User::find_by_id(&appstate.pool, session.session.user_id)
        .await?
        .ok_or(WebError::WebauthnRegistration("User not found".into()))?;
    let recovery_codes = RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
    let mut webauthn = WebAuthn::new(session.session.user_id, webauth_reg.name, &passkey)?;
    webauthn.save(&appstate.pool).await?;
    if user.mfa_method == MFAMethod::None {
        send_mfa_configured_email(
            Some(&session.session),
            &user,
            &MFAMethod::Webauthn,
            &appstate.mail_tx,
        )?;
        user.set_mfa_method(&appstate.pool, MFAMethod::Webauthn)
            .await?;
    }

    info!("Finished Webauthn registration for user {}", user.username);

    Ok(ApiResponse {
        json: json!(recovery_codes),
        status: StatusCode::OK,
    })
}

/// Start WebAuthn authentication
pub async fn webauthn_start(mut session: Session, State(appstate): State<AppState>) -> ApiResult {
    let passkeys = WebAuthn::passkeys_for_user(&appstate.pool, session.user_id).await?;

    match appstate.webauthn.start_passkey_authentication(&passkeys) {
        Ok((rcr, passkey_reg)) => {
            session
                .set_passkey_authentication(&appstate.pool, &passkey_reg)
                .await?;
            Ok(ApiResponse {
                json: json!(rcr),
                status: StatusCode::OK,
            })
        }
        Err(_err) => Err(WebError::Http(StatusCode::BAD_REQUEST)),
    }
}

/// Finish WebAuthn authentication
pub async fn webauthn_end(
    private_cookies: PrivateCookieJar,
    mut session: Session,
    State(appstate): State<AppState>,
    Json(pubkey): Json<PublicKeyCredential>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    if let Some(passkey_auth) = session.get_passkey_authentication() {
        if let Ok(auth_result) = appstate
            .webauthn
            .finish_passkey_authentication(&pubkey, &passkey_auth)
        {
            if auth_result.needs_update() {
                // Find `Passkey` and try to update its credentials
                for mut webauthn in WebAuthn::all_for_user(&appstate.pool, session.user_id).await? {
                    if let Some(true) = webauthn.passkey()?.update_credential(&auth_result) {
                        webauthn.save(&appstate.pool).await?;
                    }
                }
            }
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            return if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
                let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
                if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                    debug!("Found OpenID session cookie.");
                    let redirect_url = openid_cookie.value().to_string();
                    let private_cookies = private_cookies.remove(openid_cookie);
                    Ok((
                        private_cookies,
                        ApiResponse {
                            json: json!(AuthResponse {
                                user: user_info,
                                url: Some(redirect_url),
                            }),
                            status: StatusCode::OK,
                        },
                    ))
                } else {
                    Ok((
                        private_cookies,
                        ApiResponse {
                            json: json!(AuthResponse {
                                user: user_info,
                                url: None,
                            }),
                            status: StatusCode::OK,
                        },
                    ))
                }
            } else {
                Ok((private_cookies, ApiResponse::default()))
            };
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
    Ok(ApiResponse {
        json: json!(AuthTotp::new(secret)),
        status: StatusCode::OK,
    })
}

/// Enable TOTP
pub async fn totp_enable(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<AuthCode>,
) -> ApiResult {
    let mut user = session.user;
    debug!("Enabling TOTP for user {}", user.username);
    if user.verify_totp_code(data.code) {
        let recovery_codes = RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
        user.enable_totp(&appstate.pool).await?;
        if user.mfa_method == MFAMethod::None {
            send_mfa_configured_email(
                Some(&session.session),
                &user,
                &MFAMethod::OneTimePassword,
                &appstate.mail_tx,
            )?;
            user.set_mfa_method(&appstate.pool, MFAMethod::OneTimePassword)
                .await?;
        }

        info!("Enabled TOTP for user {}", user.username);
        Ok(ApiResponse {
            json: json!(recovery_codes),
            status: StatusCode::OK,
        })
    } else {
        Err(WebError::ObjectNotFound("Invalid TOTP code".into()))
    }
}

/// Disable TOTP
pub async fn totp_disable(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    let mut user = session.user;
    debug!("Disabling TOTP for user {}", user.username);
    user.disable_totp(&appstate.pool).await?;
    user.verify_mfa_state(&appstate.pool).await?;
    info!("Disabled TOTP for user {}", user.username);
    Ok(ApiResponse::default())
}

/// Validate one-time passcode
pub async fn totp_code(
    private_cookies: PrivateCookieJar,
    mut session: Session,
    State(appstate): State<AppState>,
    Json(data): Json<AuthCode>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        let username = user.username.clone();
        debug!("Verifying TOTP for user {}", username);
        if user.totp_enabled && user.verify_totp_code(data.code) {
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
            info!("Verified TOTP for user {username}");
            if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                debug!("Found openid session cookie.");
                let redirect_url = openid_cookie.value().to_string();
                let private_cookies = private_cookies.remove(openid_cookie);
                Ok((
                    private_cookies,
                    ApiResponse {
                        json: json!(AuthResponse {
                            user: user_info,
                            url: Some(redirect_url),
                        }),
                        status: StatusCode::OK,
                    },
                ))
            } else {
                Ok((
                    private_cookies,
                    ApiResponse {
                        json: json!(AuthResponse {
                            user: user_info,
                            url: None,
                        }),
                        status: StatusCode::OK,
                    },
                ))
            }
        } else {
            Err(WebError::Authorization("Invalid TOTP code".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("Invalid user".into()))
    }
}

/// Initialize email MFA setup
pub async fn email_mfa_init(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    // check if SMTP is configured
    let settings = Settings::get_settings(&appstate.pool).await?;
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
    send_email_mfa_activation_email(&user, &appstate.mail_tx, &session.session)?;

    Ok(ApiResponse::default())
}

/// Enable email MFA
pub async fn email_mfa_enable(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<AuthCode>,
) -> ApiResult {
    let mut user = session.user;
    debug!("Enabling email MFA for user {}", user.username);
    if user.verify_email_mfa_code(data.code) {
        let recovery_codes = RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
        user.enable_email_mfa(&appstate.pool).await?;
        if user.mfa_method == MFAMethod::None {
            send_mfa_configured_email(
                Some(&session.session),
                &user,
                &MFAMethod::Email,
                &appstate.mail_tx,
            )?;
            user.set_mfa_method(&appstate.pool, MFAMethod::Email)
                .await?;
        }

        info!("Enabled email MFA for user {}", user.username);
        Ok(ApiResponse {
            json: json!(recovery_codes),
            status: StatusCode::OK,
        })
    } else {
        Err(WebError::ObjectNotFound("Invalid email code".into()))
    }
}

/// Disable email MFA
pub async fn email_mfa_disable(
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    let mut user = session.user;
    debug!("Disabling email MFA for user {}", user.username);
    user.disable_email_mfa(&appstate.pool).await?;
    user.verify_mfa_state(&appstate.pool).await?;
    info!("Disabled email MFA for user {}", user.username);
    Ok(ApiResponse::default())
}

/// Send email code to user
pub async fn request_email_mfa_code(
    session: Session,
    State(appstate): State<AppState>,
) -> ApiResult {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        debug!("Sending email MFA code for user {}", user.username);
        if user.email_mfa_enabled {
            send_email_mfa_code_email(&user, &appstate.mail_tx, &session)?;
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
    mut session: Session,
    State(appstate): State<AppState>,
    Json(data): Json<AuthCode>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        let username = user.username.clone();
        debug!("Verifying email MFA code for user {}", username);
        if user.email_mfa_enabled && user.verify_email_mfa_code(data.code) {
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
            info!("Verified email MFA code for user {username}");
            if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                debug!("Found openid session cookie.");
                let redirect_url = openid_cookie.value().to_string();
                let private_cookies = private_cookies.remove(openid_cookie);
                Ok((
                    private_cookies,
                    ApiResponse {
                        json: json!(AuthResponse {
                            user: user_info,
                            url: Some(redirect_url),
                        }),
                        status: StatusCode::OK,
                    },
                ))
            } else {
                Ok((
                    private_cookies,
                    ApiResponse {
                        json: json!(AuthResponse {
                            user: user_info,
                            url: None,
                        }),
                        status: StatusCode::OK,
                    },
                ))
            }
        } else {
            Err(WebError::Authorization("Invalid email MFA code".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("Invalid user".into()))
    }
}

/// Start Web3 authentication
pub async fn web3auth_start(
    mut session: Session,
    State(appstate): State<AppState>,
    Json(data): Json<WalletAddress>,
) -> ApiResult {
    debug!("Starting web3 authentication for wallet {}", data.address);
    match Settings::find_by_id(&appstate.pool, 1).await? {
        Some(settings) => {
            let challenge = Wallet::format_challenge(&data.address, &settings.challenge_template);
            session
                .set_web3_challenge(&appstate.pool, challenge.clone())
                .await?;
            info!("Started web3 authentication for wallet {}", data.address);
            Ok(ApiResponse {
                json: json!({ "challenge": challenge }),
                status: StatusCode::OK,
            })
        }
        None => Err(WebError::DbError("cannot retrieve settings".into())),
    }
}

/// Finish Web3 authentication
pub async fn web3auth_end(
    private_cookies: PrivateCookieJar,
    mut session: Session,
    State(appstate): State<AppState>,
    Json(signature): Json<WalletSignature>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    debug!(
        "Finishing web3 authentication for wallet {}",
        signature.address
    );
    if let Some(ref challenge) = session.web3_challenge {
        if let Some(wallet) =
            Wallet::find_by_user_and_address(&appstate.pool, session.user_id, &signature.address)
                .await?
        {
            if wallet.use_for_mfa {
                return match wallet.verify_address(challenge, &signature.signature) {
                    Ok(true) => {
                        session
                            .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                            .await?;
                        if let Some(user) =
                            User::find_by_id(&appstate.pool, session.user_id).await?
                        {
                            let username = user.username.clone();
                            let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
                            info!(
                                "User {username} authenticated with wallet {}",
                                signature.address
                            );
                            if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                                debug!("Found openid session cookie.");
                                let redirect_url = openid_cookie.value().to_string();
                                let private_cookies = private_cookies.remove(openid_cookie);
                                Ok((
                                    private_cookies,
                                    ApiResponse {
                                        json: json!(AuthResponse {
                                            user: user_info,
                                            url: Some(redirect_url),
                                        }),
                                        status: StatusCode::OK,
                                    },
                                ))
                            } else {
                                Ok((
                                    private_cookies,
                                    ApiResponse {
                                        json: json!(AuthResponse {
                                            user: user_info,
                                            url: None,
                                        }),
                                        status: StatusCode::OK,
                                    },
                                ))
                            }
                        } else {
                            Ok((private_cookies, ApiResponse::default()))
                        }
                    }
                    _ => Err(WebError::Authorization("Signature not verified".into())),
                };
            }
        }
    }
    Err(WebError::Http(StatusCode::BAD_REQUEST))
}

/// Authenticate with a recovery code.
pub async fn recovery_code(
    private_cookies: PrivateCookieJar,
    mut session: Session,
    State(appstate): State<AppState>,
    Json(recovery_code): Json<RecoveryCode>,
) -> Result<(PrivateCookieJar, ApiResponse), WebError> {
    if let Some(mut user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        let username = user.username.clone();
        debug!("Authenticating user {} with recovery code", username);
        if user
            .verify_recovery_code(&appstate.pool, &recovery_code.code)
            .await?
        {
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
            info!("Authenticated user {username} with recovery code");
            if let Some(openid_cookie) = private_cookies.get(SIGN_IN_COOKIE_NAME) {
                debug!("Found OpenID session cookie.");
                let redirect_url = openid_cookie.value().to_string();
                let private_cookies = private_cookies.remove(openid_cookie);
                return Ok((
                    private_cookies,
                    ApiResponse {
                        json: json!(AuthResponse {
                            user: user_info,
                            url: Some(redirect_url),
                        }),
                        status: StatusCode::OK,
                    },
                ));
            }

            return Ok((
                private_cookies,
                ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: None,
                    }),
                    status: StatusCode::OK,
                },
            ));
        }
    }
    Err(WebError::Http(StatusCode::UNAUTHORIZED))
}
