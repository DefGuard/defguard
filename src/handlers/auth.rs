use super::{
    ApiResponse, ApiResult, Auth, AuthCode, AuthResponse, AuthTotp, RecoveryCode, RecoveryCodes,
    WalletAddress, WalletSignature, WebAuthnRegistration,
};
use crate::auth::failed_login::{check_username, log_failed_login_attempt};
use crate::db::MFAMethod;
use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::{MFAInfo, Session, SessionState, Settings, User, UserInfo, Wallet, WebAuthn},
    error::OriWebError,
    ldap::utils::user_from_ldap,
    license::Features,
};
use rocket::serde::json::serde_json;
use rocket::time::Duration;
use rocket::{
    http::{Cookie, CookieJar, SameSite, Status},
    serde::json::{serde_json::json, Json},
    State,
};
use sqlx::types::Uuid;
use webauthn_rs::prelude::PublicKeyCredential;
use webauthn_rs_proto::options::CollectedClientData;

/// For successful login, return:
/// * 200 with MFA disabled
/// * 201 with MFA enabled when additional authentication factor is required
#[post("/auth", format = "json", data = "<data>")]
pub async fn authenticate(
    appstate: &State<AppState>,
    mut data: Json<Auth>,
    cookies: &CookieJar<'_>,
) -> ApiResult {
    debug!("Authenticating user {}", data.username);
    // check if user can proceed with login
    check_username(&appstate.failed_logins, &data.username)?;

    data.username = data.username.to_lowercase();
    let user = match User::find_by_username(&appstate.pool, &data.username).await {
        Ok(Some(user)) => match user.verify_password(&data.password) {
            Ok(_) => user,
            Err(err) => {
                info!("Failed to authenticate user {}: {}", data.username, err);
                log_failed_login_attempt(&appstate.failed_logins, &data.username);
                return Err(OriWebError::Authorization(err.to_string()));
            }
        },
        Ok(None) => {
            // create user from LDAP
            debug!(
                "User not found in DB, authenticating user {} with LDAP",
                data.username
            );
            if appstate.license.validate(&Features::Ldap) {
                if let Ok(user) = user_from_ldap(
                    &appstate.pool,
                    &appstate.config,
                    &data.username,
                    &data.password,
                )
                .await
                {
                    user
                } else {
                    info!("Failed to authenticate user {} with LDAP", data.username);
                    log_failed_login_attempt(&appstate.failed_logins, &data.username);
                    return Err(OriWebError::Authorization("user not found".into()));
                }
            } else {
                info!(
                    "User {} not found in DB and LDAP is disabled",
                    data.username
                );
                log_failed_login_attempt(&appstate.failed_logins, &data.username);
                return Err(OriWebError::Authorization("LDAP feature disabled".into()));
            }
        }
        Err(err) => {
            error!(
                "DB error when authenticating user {}: {}",
                data.username, err
            );
            return Err(OriWebError::DbError(err.to_string()));
        }
    };

    Session::delete_expired(&appstate.pool).await?;
    let session = Session::new(user.id.unwrap(), SessionState::PasswordVerified);
    session.save(&appstate.pool).await?;

    let max_age = match &appstate.config.session_lifetime {
        Some(seconds) => Duration::seconds(*seconds),
        None => Duration::days(7),
    };

    let auth_cookie = Cookie::build("defguard_session", session.id)
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .max_age(max_age)
        .finish();
    cookies.add(auth_cookie);

    info!("Authenticated user {}", data.username);
    if user.mfa_enabled {
        if let Some(mfa_info) = MFAInfo::for_user(&appstate.pool, &user).await? {
            Ok(ApiResponse {
                json: json!(mfa_info),
                status: Status::Created,
            })
        } else {
            Err(OriWebError::DbError("MFA info read error".into()))
        }
    } else {
        let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
        if let Some(openid_cookie) = cookies.get_private("known_sign_in") {
            debug!("Found openid session cookie.");
            Ok(ApiResponse {
                json: json!(AuthResponse {
                    user: user_info,
                    url: Some(openid_cookie.value().to_string())
                }),
                status: Status::Ok,
            })
        } else {
            Ok(ApiResponse {
                json: json!(AuthResponse {
                    user: user_info,
                    url: None,
                }),
                status: Status::Ok,
            })
        }
    }
}

/// Logout - forget the session cookie.
#[post("/auth/logout")]
pub async fn logout(
    cookies: &CookieJar<'_>,
    session: Session,
    appstate: &State<AppState>,
) -> ApiResult {
    // remove auth cookie
    cookies.remove(Cookie::named("defguard_session"));
    // remove stored session
    session.delete(&appstate.pool).await?;
    Ok(ApiResponse::default())
}

/// Enable MFA
#[put("/auth/mfa")]
pub async fn mfa_enable(
    session: Session,
    session_info: SessionInfo,
    appstate: &State<AppState>,
    cookies: &CookieJar<'_>,
) -> ApiResult {
    let mut user = session_info.user;
    debug!("Enabling MFA for user {}", user.username);
    user.enable_mfa(&appstate.pool).await?;
    if user.mfa_enabled {
        info!("Enabled MFA for user {}", user.username);
        cookies.remove(Cookie::named("defguard_sesssion"));
        session.delete(&appstate.pool).await?;
        debug!(
            "Removed auth session for user {} after enabling MFA",
            user.username
        );
        Ok(ApiResponse::default())
    } else {
        error!("Error enabling MFA for user {}", user.username);
        Err(OriWebError::Http(Status::NotModified))
    }
}

/// Disable MFA
#[delete("/auth/mfa")]
pub async fn mfa_disable(session_info: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let mut user = session_info.user;
    debug!("Disabling MFA for user {}", user.username);
    user.disable_mfa(&appstate.pool).await?;
    info!("Disabled MFA for user {}", user.username);
    Ok(ApiResponse::default())
}

/// Initialize WebAuthn registration
#[post("/auth/webauthn/init")]
pub async fn webauthn_init(mut session_info: SessionInfo, appstate: &State<AppState>) -> ApiResult {
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
                status: Status::Ok,
            })
        }
        Err(err) => Err(OriWebError::WebauthnRegistration(err.to_string())),
    }
}

/// Finish WebAuthn registration
#[post("/auth/webauthn/finish", format = "json", data = "<data>")]
pub async fn webauthn_finish(
    session: SessionInfo,
    appstate: &State<AppState>,
    data: Json<WebAuthnRegistration>,
) -> ApiResult {
    info!(
        "Finishing WebAuthn registration for user {}",
        session.user.username
    );
    let passkey_reg =
        session
            .session
            .get_passkey_registration()
            .ok_or(OriWebError::WebauthnRegistration(
                "Passkey registration session not found".into(),
            ))?;

    let webauth_reg = data.into_inner();

    let ccdj: CollectedClientData = serde_json::from_slice(
        webauth_reg.rpkc.response.client_data_json.as_ref(),
    )
    .map_err(|_| {
        OriWebError::WebauthnRegistration(
            "Failed to parse passkey registration request data".into(),
        )
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
            .map(|url| url.to_string())
            .collect::<Vec<String>>()
    );

    let passkey = appstate
        .webauthn
        .finish_passkey_registration(&webauth_reg.rpkc, &passkey_reg)
        .map_err(|err| OriWebError::WebauthnRegistration(err.to_string()))?;
    let mut user = User::find_by_id(&appstate.pool, session.session.user_id)
        .await?
        .ok_or(OriWebError::WebauthnRegistration("User not found".into()))?;
    let recovery_codes = RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
    let mut webauthn = WebAuthn::new(session.session.user_id, webauth_reg.name, &passkey)?;
    webauthn.save(&appstate.pool).await?;
    if user.mfa_method == MFAMethod::None {
        user.set_mfa_method(&appstate.pool, MFAMethod::Webauthn)
            .await?;
    }
    info!("Finished Webauthn registration for user {}", user.username);

    Ok(ApiResponse {
        json: json!(recovery_codes),
        status: Status::Ok,
    })
}

/// Start WebAuthn authentication
#[post("/auth/webauthn/start")]
pub async fn webauthn_start(mut session: Session, appstate: &State<AppState>) -> ApiResult {
    let passkeys = WebAuthn::passkeys_for_user(&appstate.pool, session.user_id).await?;

    match appstate.webauthn.start_passkey_authentication(&passkeys) {
        Ok((rcr, passkey_reg)) => {
            session
                .set_passkey_authentication(&appstate.pool, &passkey_reg)
                .await?;
            Ok(ApiResponse {
                json: json!(rcr),
                status: Status::Ok,
            })
        }
        Err(_err) => Err(OriWebError::Http(Status::BadRequest)),
    }
}

/// Finish WebAuthn authentication
#[post("/auth/webauthn", format = "json", data = "<pubkey>")]
pub async fn webauthn_end(
    mut session: Session,
    appstate: &State<AppState>,
    pubkey: Json<PublicKeyCredential>,
    cookies: &CookieJar<'_>,
) -> ApiResult {
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
                if let Some(openid_cookie) = cookies.get_private("known_sign_in") {
                    debug!("Found openid session cookie.");
                    Ok(ApiResponse {
                        json: json!(AuthResponse {
                            user: user_info,
                            url: Some(openid_cookie.value().to_string())
                        }),
                        status: Status::Ok,
                    })
                } else {
                    Ok(ApiResponse {
                        json: json!(AuthResponse {
                            user: user_info,
                            url: None,
                        }),
                        status: Status::Ok,
                    })
                }
            } else {
                Ok(ApiResponse::default())
            };
        }
    }
    Err(OriWebError::Http(Status::BadRequest))
}

/// Generate new TOTP secret
#[post("/auth/totp/init")]
pub async fn totp_secret(session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let mut user = session.user;
    debug!("Generating new TOTP secret for user {}", user.username);

    let secret = user.new_secret(&appstate.pool).await?;
    info!("Generated new TOTP secret for user {}", user.username);
    Ok(ApiResponse {
        json: json!(AuthTotp::new(secret)),
        status: Status::Ok,
    })
}

/// Enable TOTP
#[post("/auth/totp", format = "json", data = "<data>")]
pub async fn totp_enable(
    session: SessionInfo,
    appstate: &State<AppState>,
    data: Json<AuthCode>,
) -> ApiResult {
    let mut user = session.user;
    debug!("Enabling TOTP for user {}", user.username);
    if user.verify_code(data.code) {
        let recovery_codes = RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
        user.enable_totp(&appstate.pool).await?;
        if user.mfa_method == MFAMethod::None {
            user.set_mfa_method(&appstate.pool, MFAMethod::OneTimePassword)
                .await?;
        }
        info!("Enabled TOTP for user {}", user.username);
        Ok(ApiResponse {
            json: json!(recovery_codes),
            status: Status::Ok,
        })
    } else {
        Err(OriWebError::ObjectNotFound("Invalid TOTP code".into()))
    }
}

/// Disable TOTP
#[delete("/auth/totp")]
pub async fn totp_disable(session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let mut user = session.user;
    debug!("Disabling TOTP for user {}", user.username);
    user.disable_totp(&appstate.pool).await?;
    user.verify_mfa_state(&appstate.pool).await?;
    info!("Disabled TOTP for user {}", user.username);
    Ok(ApiResponse::default())
}

/// Validate one-time passcode
#[post("/auth/totp/verify", format = "json", data = "<data>")]
pub async fn totp_code(
    mut session: Session,
    appstate: &State<AppState>,
    data: Json<AuthCode>,
    cookies: &CookieJar<'_>,
) -> ApiResult {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        let username = user.username.clone();
        debug!("Verifying TOTP for user {}", username);
        if user.totp_enabled && user.verify_code(data.code) {
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
            info!("Verified TOTP for user {}", username);
            if let Some(openid_cookie) = cookies.get_private("known_sign_in") {
                debug!("Found openid session cookie.");
                Ok(ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: Some(openid_cookie.value().to_string())
                    }),
                    status: Status::Ok,
                })
            } else {
                Ok(ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: None,
                    }),
                    status: Status::Ok,
                })
            }
        } else {
            Err(OriWebError::Authorization("Invalid TOTP code".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound("Invalid user".into()))
    }
}
/// Start Web3 authentication
#[post("/auth/web3/start", format = "json", data = "<data>")]
pub async fn web3auth_start(
    mut session: Session,
    appstate: &State<AppState>,
    data: Json<WalletAddress>,
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
                status: Status::Ok,
            })
        }
        None => Err(OriWebError::DbError("cannot retrieve settings".into())),
    }
}

/// Finish Web3 authentication
#[post("/auth/web3", format = "json", data = "<signature>")]
pub async fn web3auth_end(
    mut session: Session,
    appstate: &State<AppState>,
    signature: Json<WalletSignature>,
    cookies: &CookieJar<'_>,
) -> ApiResult {
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
                                "User {} authenticated with wallet {}",
                                username, signature.address
                            );
                            if let Some(openid_cookie) = cookies.get_private("known_sign_in") {
                                debug!("Found openid session cookie.");
                                Ok(ApiResponse {
                                    json: json!(AuthResponse {
                                        user: user_info,
                                        url: Some(openid_cookie.value().to_string())
                                    }),
                                    status: Status::Ok,
                                })
                            } else {
                                Ok(ApiResponse {
                                    json: json!(AuthResponse {
                                        user: user_info,
                                        url: None,
                                    }),
                                    status: Status::Ok,
                                })
                            }
                        } else {
                            Ok(ApiResponse::default())
                        }
                    }
                    _ => Err(OriWebError::Authorization("Signature not verified".into())),
                };
            }
        }
    }
    Err(OriWebError::Http(Status::BadRequest))
}

/// Authenticate with a recovery code.
#[post("/auth/recovery", format = "json", data = "<recovery_code>")]
pub async fn recovery_code(
    mut session: Session,
    appstate: &State<AppState>,
    recovery_code: Json<RecoveryCode>,
    cookies: &CookieJar<'_>,
) -> ApiResult {
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
            info!("Authenticated user {} with recovery code", username);
            if let Some(openid_cookie) = cookies.get_private("known_sign_in") {
                debug!("Found openid session cookie.");
                return Ok(ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: Some(openid_cookie.value().to_string())
                    }),
                    status: Status::Ok,
                });
            } else {
                return Ok(ApiResponse {
                    json: json!(AuthResponse {
                        user: user_info,
                        url: None,
                    }),
                    status: Status::Ok,
                });
            }
        }
    }
    Err(OriWebError::Http(Status::Unauthorized))
}
