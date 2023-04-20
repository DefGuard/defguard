use super::{
    ApiResponse, ApiResult, Auth, AuthCode, AuthResponse, AuthTotp, RecoveryCode, RecoveryCodes,
    WalletAddress, WalletSignature, WebAuthnRegistration,
};
use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::{MFAInfo, MFAMethod, Session, SessionState, Settings, User, UserInfo, Wallet, WebAuthn},
    error::OriWebError,
    ldap::utils::user_from_ldap,
    license::Features,
};
use rocket::{
    http::{Cookie, CookieJar, SameSite, Status},
    serde::json::{serde_json::json, Json},
    State,
};
use sqlx::types::Uuid;
use webauthn_rs::prelude::PublicKeyCredential;

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
    data.username = data.username.to_lowercase();
    let user = match User::find_by_username(&appstate.pool, &data.username).await {
        Ok(Some(user)) => match user.verify_password(&data.password) {
            Ok(_) => user,
            Err(err) => {
                info!("Failed to authenticate user {}: {}", data.username, err);
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
                    return Err(OriWebError::Authorization("user not found".into()));
                }
            } else {
                info!(
                    "User {} not found in DB and LDAP is disabled",
                    data.username
                );
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

    let auth_cookie = Cookie::build("defguard_session", session.id)
        .http_only(true)
        .same_site(SameSite::None)
        .finish();
    cookies.add(auth_cookie);

    info!("Authenticated user {}", data.username);
    if user.mfa_enabled {
        let mfa_info = MFAInfo::for_user(&appstate.pool, &user).await?;
        Ok(ApiResponse {
            json: json!(mfa_info),
            status: Status::Created,
        })
    } else {
        let user_info = UserInfo::from_user(&appstate.pool, user).await?;
        if let Some(openid_cookie) = cookies.get("known_sign_in") {
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
pub fn logout(cookies: &CookieJar<'_>) -> ApiResult {
    cookies.remove(Cookie::named("defguard_session"));
    Ok(ApiResponse::default())
}

/// Enable MFA
#[put("/auth/mfa")]
pub async fn mfa_enable(session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let mut user = session.user;
    debug!("Enabling MFA for user {}", user.username);
    user.enable_mfa(&appstate.pool).await?;
    if user.mfa_enabled {
        info!("Enabled MFA for user {}", user.username);
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
pub async fn webauthn_init(
    _session: SessionInfo,
    mut session: Session,
    appstate: &State<AppState>,
) -> ApiResult {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        debug!(
            "Initializing WebAuthn registration for user {}",
            user.username
        );
        // passkeys to exclude
        let passkeys = WebAuthn::passkeys_for_user(&appstate.pool, session.user_id).await?;
        match appstate.webauthn.start_passkey_registration(
            Uuid::new_v4(),
            &user.email,
            &user.username,
            Some(passkeys.iter().map(|key| key.cred_id().clone()).collect()),
        ) {
            Ok((ccr, passkey_reg)) => {
                session
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
            Err(_err) => Err(OriWebError::Http(Status::BadRequest)),
        }
    } else {
        Err(OriWebError::ObjectNotFound("invalid user".into()))
    }
}

/// Finish WebAuthn registration
#[post("/auth/webauthn/finish", format = "json", data = "<data>")]
pub async fn webauthn_finish(
    session: Session,
    appstate: &State<AppState>,
    data: Json<WebAuthnRegistration>,
) -> ApiResult {
    if let Some(passkey_reg) = session.get_passkey_registration() {
        let webauth_reg = data.into_inner();
        if let Ok(passkey) = appstate
            .webauthn
            .finish_passkey_registration(&webauth_reg.rpkc, &passkey_reg)
        {
            if let Some(mut user) = User::find_by_id(&appstate.pool, session.user_id).await? {
                user.set_mfa_method(&appstate.pool, MFAMethod::Webauthn)
                    .await?;
                let recovery_codes =
                    RecoveryCodes::new(user.get_recovery_codes(&appstate.pool).await?);
                let mut webauthn = WebAuthn::new(session.user_id, webauth_reg.name, &passkey)?;
                webauthn.save(&appstate.pool).await?;
                info!("Finished Webauthn registration for user {}", user.username);
                return Ok(ApiResponse {
                    json: json!(recovery_codes),
                    status: Status::Ok,
                });
            }
        }
    }
    Err(OriWebError::Http(Status::BadRequest))
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
                let user_info = UserInfo::from_user(&appstate.pool, user).await?;
                if let Some(openid_cookie) = cookies.get("known_sign_in") {
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
        user.set_mfa_method(&appstate.pool, MFAMethod::OneTimePassword)
            .await?;
        user.enable_totp(&appstate.pool).await?;
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
            let user_info = UserInfo::from_user(&appstate.pool, user).await?;
            info!("Verified TOTP for user {}", username);
            if let Some(openid_cookie) = cookies.get("known_sign_in") {
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
                            let user_info = UserInfo::from_user(&appstate.pool, user).await?;
                            info!(
                                "User {} authenticated with wallet {}",
                                username, signature.address
                            );
                            if let Some(openid_cookie) = cookies.get("known_sign_in") {
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
            let user_info = UserInfo::from_user(&appstate.pool, user).await?;
            info!("Authenticated user {} with recovery code", username);
            if let Some(openid_cookie) = cookies.get("known_sign_in") {
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
