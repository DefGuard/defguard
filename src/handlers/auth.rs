use super::{
    ApiResponse, ApiResult, Auth, AuthCode, AuthTotp, WalletSignature, WebAuthnRegistration,
};
use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::{MFAInfo, Session, SessionState, Settings, User, UserInfo, Wallet, WebAuthn},
    enterprise::ldap::utils::user_from_ldap,
    error::OriWebError,
    license::Features,
};
use rocket::{
    http::{Cookie, CookieJar, Status},
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
    data: Json<Auth>,
    cookies: &CookieJar<'_>,
) -> ApiResult {
    debug!("Authenticating user {}", data.username);
    let user = match User::find_by_username(&appstate.pool, &data.username).await {
        Ok(Some(user)) => match user.verify_password(&data.password) {
            Ok(_) => user,
            Err(err) => {
                info!("Failed to authenticate user {}: {}", data.username, err);
                return Err(OriWebError::Authorization(err.to_string()));
            }
        },
        Ok(None) => {
            error!("User not found {}", data.username);
            // create user from LDAP
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
                    return Err(OriWebError::Authorization("user not found".into()));
                }
            } else {
                return Err(OriWebError::Authorization("LDAP feature disabled".into()));
            }
        }
        Err(err) => {
            error!(
                "Error when trying to authenticate user {}: {}",
                data.username, err
            );
            return Err(OriWebError::DbError(err.to_string()));
        }
    };

    Session::delete_expired(&appstate.pool).await?;
    let session = Session::new(user.id.unwrap(), SessionState::PasswordVerified);
    session.save(&appstate.pool).await?;
    cookies.add(Cookie::new("session", session.id));

    info!("Authenticated user {}", data.username);
    // TODO: return MFA struct with enabled methods
    if user.mfa_enabled {
        let mfa_info = MFAInfo::for_user(&appstate.pool, &user).await?;
        Ok(ApiResponse {
            json: json!(mfa_info),
            status: Status::Created,
        })
    } else {
        let user_info = UserInfo::from_user(&appstate.pool, user).await?;
        Ok(ApiResponse {
            json: json!(user_info),
            status: Status::Ok,
        })
    }
}

#[post("/auth/logout")]
pub fn logout(cookies: &CookieJar<'_>) -> ApiResult {
    cookies.remove(Cookie::named("session"));
    Ok(ApiResponse::default())
}

/// Enable MFA
#[post("/auth/mfa")]
pub async fn mfa_enable(session_info: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let mut user = session_info.user;
    let recovery_codes = user.enable_mfa(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(recovery_codes),
        status: Status::Ok,
    })
}

/// Disable MFA
#[delete("/auth/mfa")]
pub async fn mfa_disable(session_info: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let mut user = session_info.user;
    user.disable_mfa(&appstate.pool).await?;
    Ok(ApiResponse::default())
}

/// Initialize WebAuthn registration
#[post("/auth/webauthn/init")]
pub async fn webauthn_init(mut session: Session, appstate: &State<AppState>) -> ApiResult {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        match appstate.webauthn.start_passkey_registration(
            Uuid::new_v4(),
            &user.email,
            &user.username,
            None,
        ) {
            Ok((ccr, passkey_reg)) => {
                session
                    .set_passkey_registration(&appstate.pool, &passkey_reg)
                    .await?;
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
            let mut webauthn = WebAuthn::new(session.user_id, webauth_reg.name, &passkey)?;
            webauthn.save(&appstate.pool).await?;
            return Ok(ApiResponse::default());
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
            return Ok(ApiResponse::default());
        }
    }
    Err(OriWebError::Http(Status::BadRequest))
}

/// Generate new TOTP secret
#[post("/auth/totp/init")]
pub async fn totp_secret(session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let mut user = session.user;

    let secret = user.new_secret(&appstate.pool).await?;
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
    if user.verify_code(data.code) {
        user.enable_totp(&appstate.pool).await?;
        Ok(ApiResponse::default())
    } else {
        Err(OriWebError::ObjectNotFound("Invalid TOTP code".into()))
    }
}

/// Disable TOTP
#[delete("/auth/totp")]
pub async fn totp_disable(session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let mut user = session.user;
    user.disable_totp(&appstate.pool).await?;
    Ok(ApiResponse::default())
}

/// Validate one-time passcode
#[post("/auth/totp/verify", format = "json", data = "<data>")]
pub async fn totp_code(
    mut session: Session,
    appstate: &State<AppState>,
    data: Json<AuthCode>,
) -> ApiResult {
    if let Some(user) = User::find_by_id(&appstate.pool, session.user_id).await? {
        if user.totp_enabled && user.verify_code(data.code) {
            session
                .set_state(&appstate.pool, SessionState::MultiFactorVerified)
                .await?;
            Ok(ApiResponse::default())
        } else {
            Err(OriWebError::Authorization("Invalid TOTP code".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound("Invalid user".into()))
    }
}

/// Start Web3 authentication
#[post("/auth/web3/start")]
pub async fn web3auth_start(mut session: Session, appstate: &State<AppState>) -> ApiResult {
    match Settings::find_by_id(&appstate.pool, 1).await? {
        Some(settings) => {
            session
                .set_web3_challenge(&appstate.pool, settings.challenge_template.clone())
                .await?;
            Ok(ApiResponse {
                json: json!({"challenge": settings.challenge_template}),
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
) -> ApiResult {
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
                        Ok(ApiResponse::default())
                    }
                    _ => Err(OriWebError::Authorization("Signature not verified".into())),
                };
            }
        }
    }
    Err(OriWebError::Http(Status::BadRequest))
}
