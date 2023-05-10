use super::{
    user_for_admin_or_self, AddUserData, ApiResponse, ApiResult, PasswordChange, RecoveryCodes,
    Username, WalletChallenge, WalletChange, WalletSignature,
};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{AppEvent, MFAMethod, OAuth2AuthorizedApp, Settings, User, UserInfo, Wallet, WebAuthn},
    error::OriWebError,
    ldap::utils::{ldap_add_user, ldap_change_password, ldap_delete_user, ldap_modify_user},
    license::Features,
};
use log::debug;
use regex::Regex;
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};

/// Verify the given username consists of all ASCII digits or lowercase characters.
fn check_username(username: &str) -> Result<(), OriWebError> {
    if username
        .chars()
        .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
    {
        Ok(())
    } else {
        Err(OriWebError::IncorrectUsername(username.into()))
    }
}

fn check_password_strength(password: &str) -> Result<(), OriWebError> {
    let password_length = password.len();
    let special_chars_expression = Regex::new(r#"[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?~]"#).unwrap();
    let numbers_expression = Regex::new(r"[0-9]").unwrap();
    let lowercase_expression = Regex::new(r"[a-z]").unwrap();
    let uppercase_expression = Regex::new(r"[A-Z]").unwrap();
    if password_length < 8 || password_length > 32 {
        return Err(OriWebError::Serialization("Incorrect password length".into()));
    }
    if !special_chars_expression.is_match(password) {
        return Err(OriWebError::Serialization("No special characters in password".into()));
    }
    if !numbers_expression.is_match(password) {
        return Err(OriWebError::Serialization("No numbers in password".into()))
    }
    if !lowercase_expression.is_match(password) {
        return Err(OriWebError::Serialization("No lowercase characters in password".into()));
    }
    if !uppercase_expression.is_match(password) {
        return Err(OriWebError::Serialization("No uppercase characters in password".into()))
    }
    Ok(())
}

#[get("/user", format = "json")]
pub async fn list_users(_admin: AdminRole, appstate: &State<AppState>) -> ApiResult {
    let all_users = User::all(&appstate.pool).await?;
    let mut users: Vec<UserInfo> = Vec::with_capacity(all_users.len());
    for user in all_users {
        users.push(UserInfo::from_user(&appstate.pool, user).await?);
    }
    Ok(ApiResponse {
        json: json!(users),
        status: Status::Ok,
    })
}

#[get("/user/<username>", format = "json")]
pub async fn get_user(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    let user_info = UserInfo::from_user(&appstate.pool, user).await?;
    Ok(ApiResponse {
        json: json!(user_info),
        status: Status::Ok,
    })
}

#[post("/user", format = "json", data = "<data>")]
pub async fn add_user(
    _admin: AdminRole,
    appstate: &State<AppState>,
    data: Json<AddUserData>,
    session: SessionInfo,
) -> ApiResult {
    let username = data.username.clone();
    debug!("User {} adding user {}", session.user.username, username);
    let user_data = data.into_inner();
    let password = user_data.password.clone();
    if let Err(err) = check_password_strength(&password) {
        debug!("Pasword not strong enough: {}", err);
        return Ok(ApiResponse { json: json!({}), status: Status::BadRequest });
    }
    check_username(&user_data.username)?;
    let mut user = User::new(
        user_data.username,
        &user_data.password,
        user_data.last_name,
        user_data.first_name,
        user_data.email,
        Some(user_data.phone),
    );
    user.save(&appstate.pool).await?;
    if appstate.license.validate(&Features::Ldap) {
        let _result = ldap_add_user(&appstate.config, &user, &password).await;
    };
    let user_info = UserInfo::from_user(&appstate.pool, user).await?;
    appstate.trigger_action(AppEvent::UserCreated(user_info));
    info!("User {} added user {}", session.user.username, username);
    Ok(ApiResponse {
        json: json!({}),
        status: Status::Created,
    })
}

#[post("/user/available", format = "json", data = "<data>")]
pub async fn username_available(
    _session: SessionInfo,
    appstate: &State<AppState>,
    data: Json<Username>,
) -> ApiResult {
    check_username(&data.username)?;
    let status = match User::find_by_username(&appstate.pool, &data.username).await? {
        Some(_) => Status::BadRequest,
        None => Status::Ok,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[put("/user/<username>", format = "json", data = "<data>")]
pub async fn modify_user(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    data: Json<UserInfo>,
) -> ApiResult {
    debug!("User {} updating user {}", session.user.username, username);
    let mut user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    let user_info = data.into_inner();
    if session.is_admin {
        user_info
            .into_user_all_fields(&appstate.pool, &mut user)
            .await?;
    } else {
        user_info.into_user_safe_fields(&mut user).await?;
    }
    user.save(&appstate.pool).await?;

    if appstate.license.validate(&Features::Ldap) {
        let _result = ldap_modify_user(&appstate.config, username, &user).await;
    };
    let user_info = UserInfo::from_user(&appstate.pool, user).await?;
    appstate.trigger_action(AppEvent::UserModified(user_info));
    info!("User {} updated user {}", session.user.username, username);
    Ok(ApiResponse::default())
}

#[delete("/user/<username>")]
pub async fn delete_user(
    _admin: AdminRole,
    appstate: &State<AppState>,
    username: &str,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} deleting user {}", session.user.username, username);
    match User::find_by_username(&appstate.pool, username).await? {
        Some(user) => {
            user.delete(&appstate.pool).await?;
            if appstate.license.validate(&Features::Ldap) {
                let _result = ldap_delete_user(&appstate.config, username).await;
            };
            appstate.trigger_action(AppEvent::UserDeleted(username.into()));
            info!("User {} deleted user {}", session.user.username, username);
            Ok(ApiResponse::default())
        }
        None => {
            error!("User {} not found", username);
            Err(OriWebError::ObjectNotFound(format!(
                "User {} not found",
                username
            )))
        }
    }
}

#[put("/user/<username>/password", format = "json", data = "<data>")]
pub async fn change_password(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    data: Json<PasswordChange>,
) -> ApiResult {
    debug!(
        "User {} changing password for user {}",
        session.user.username, username
    );
    let password = &data.new_password.clone();
    if let Err(err) = check_password_strength(&password) {
        debug!("Pasword not strong enough: {}", err);
        return Ok(ApiResponse { json: json!({}), status: Status::BadRequest });
    }
    let mut user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    user.set_password(&data.new_password);
    user.save(&appstate.pool).await?;
    if appstate.license.validate(&Features::Ldap) {
        let _result = ldap_change_password(&appstate.config, username, &data.new_password).await;
    }
    info!(
        "User {} changed password for user {}",
        session.user.username, username
    );
    Ok(ApiResponse::default())
}

#[get("/user/<username>/challenge?<address>&<name>&<chain_id>")]
pub async fn wallet_challenge(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    address: &str,
    name: &str,
    chain_id: i64,
) -> ApiResult {
    debug!(
        "User {} generating wallet challenge for user {}",
        session.user.username, username
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;

    // check if address already exists
    let wallet = match Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), address)
        .await?
    {
        Some(wallet) => {
            if wallet.validation_timestamp.is_some() {
                return Err(OriWebError::ObjectNotFound("wrong address".into()));
            }
            wallet
        }
        None => {
            let challenge_message = match Settings::find_by_id(&appstate.pool, 1).await? {
                Some(settings) => Wallet::format_challenge(address, &settings.challenge_template),
                None => return Err(OriWebError::DbError("cannot retrieve settings".into())),
            };
            let mut wallet = Wallet::new_for_user(
                user.id.unwrap(),
                address.into(),
                name.into(),
                chain_id,
                challenge_message,
            );
            wallet.save(&appstate.pool).await?;
            wallet
        }
    };

    info!(
        "User {} generated wallet challenge for user {}",
        session.user.username, username
    );
    Ok(ApiResponse {
        json: json!(WalletChallenge {
            id: wallet.id.unwrap(),
            message: wallet.challenge_message
        }),
        status: Status::Ok,
    })
}

#[put("/user/<username>/wallet", format = "json", data = "<data>")]
pub async fn set_wallet(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    data: Json<WalletSignature>,
) -> ApiResult {
    debug!(
        "User {} setting wallet signature for user {}",
        session.user.username, username
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    let wallet_info = data.into_inner();
    if let Some(mut wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), &wallet_info.address)
            .await?
    {
        if wallet.validate_signature(&wallet_info.signature).is_ok() {
            wallet
                .set_signature(&appstate.pool, &wallet_info.signature)
                .await?;
            info!(
                "User {} set wallet signature for user {}",
                session.user.username, username
            );
            Ok(ApiResponse::default())
        } else {
            Err(OriWebError::ObjectNotFound("wrong address".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound("wallet not found".into()))
    }
}

/// Change wallet.
/// Currenly only `use_for_mfa` flag can be set or unset.
#[put("/user/<username>/wallet/<address>", format = "json", data = "<data>")]
pub async fn update_wallet(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    address: &str,
    data: Json<WalletChange>,
) -> ApiResult {
    debug!(
        "User {} updating wallet {} for user {}",
        session.user.username, address, username
    );
    let mut user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    if let Some(mut wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), address).await?
    {
        if Some(wallet.user_id) == user.id {
            wallet.use_for_mfa = data.use_for_mfa;
            let recovery_codes = if data.use_for_mfa {
                user.set_mfa_method(&appstate.pool, MFAMethod::Web3).await?;
                user.get_recovery_codes(&appstate.pool).await?
            } else {
                None
            };
            wallet.save(&appstate.pool).await?;
            info!(
                "User {} updated wallet {} for user {}",
                session.user.username, address, username
            );
            Ok(ApiResponse {
                json: json!(RecoveryCodes::new(recovery_codes)),
                status: Status::Ok,
            })
        } else {
            Err(OriWebError::ObjectNotFound("wrong wallet".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound("wallet not found".into()))
    }
}

/// Delete wallet.
#[delete("/user/<username>/wallet/<address>")]
pub async fn delete_wallet(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    address: &str,
) -> ApiResult {
    debug!(
        "User {} deleting wallet {} for user {}",
        session.user.username, address, username
    );
    let mut user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    if let Some(wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), address).await?
    {
        if Some(wallet.user_id) == user.id {
            wallet.delete(&appstate.pool).await?;
            user.verify_mfa_state(&appstate.pool).await?;
            info!(
                "User {} deleted wallet {} for user {}",
                session.user.username, address, username
            );
            Ok(ApiResponse::default())
        } else {
            Err(OriWebError::ObjectNotFound("wrong wallet".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound("wallet not found".into()))
    }
}

#[delete("/user/<username>/security_key/<id>")]
pub async fn delete_security_key(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    id: i64,
) -> ApiResult {
    debug!(
        "User {} deleting security key {} for user {}",
        session.user.username, id, username
    );
    let mut user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    if let Some(webauthn) = WebAuthn::find_by_id(&appstate.pool, id).await? {
        if Some(webauthn.user_id) == user.id {
            webauthn.delete(&appstate.pool).await?;
            user.verify_mfa_state(&appstate.pool).await?;
            info!(
                "User {} deleted security key {} for user {}",
                session.user.username, id, username
            );
            Ok(ApiResponse::default())
        } else {
            Err(OriWebError::ObjectNotFound("wrong security key".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound("security key not found".into()))
    }
}

#[get("/me", format = "json")]
pub async fn me(session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let user_info = UserInfo::from_user(&appstate.pool, session.user).await?;
    Ok(ApiResponse {
        json: json!(user_info),
        status: Status::Ok,
    })
}

/// Delete Oauth token.
#[delete("/user/<username>/oauth_app/<oauth2client_id>")]
pub async fn delete_authorized_app(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    oauth2client_id: i64,
) -> ApiResult {
    debug!(
        "User {} deleting OAuth2 client {} for user {}",
        session.user.username, oauth2client_id, username
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    if let Some(app) = OAuth2AuthorizedApp::find_by_user_and_oauth2client_id(
        &appstate.pool,
        user.id.unwrap(),
        oauth2client_id,
    )
    .await?
    {
        if Some(app.user_id) == user.id {
            app.delete(&appstate.pool).await?;
            info!(
                "User {} deleted OAuth2 client {} for user {}",
                session.user.username, oauth2client_id, username
            );
            Ok(ApiResponse::default())
        } else {
            Err(OriWebError::ObjectNotFound("Wrong app".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound(
            "Authorized app not found".into(),
        ))
    }
}
