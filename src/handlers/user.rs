use super::{
    user_for_admin_or_self, AddUserData, ApiResponse, ApiResult, PasswordChange, Username,
    WalletChallenge, WalletChange, WalletSignature,
};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{AppEvent, Settings, User, UserInfo, Wallet, WebAuthn},
    enterprise::ldap::utils::{
        ldap_add_user, ldap_change_password, ldap_delete_user, ldap_modify_user,
    },
    error::OriWebError,
    license::Features,
};
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

#[get("/user", format = "json")]
pub async fn list_users(_session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    debug!("Listing users");
    let all_users = User::all(&appstate.pool).await?;
    let mut users: Vec<UserInfo> = Vec::with_capacity(all_users.len());
    for user in all_users {
        users.push(UserInfo::from_user(&appstate.pool, user).await?);
    }
    info!("Listed users");
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
    debug!("Retrieving user {}", username);
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    info!("Retrieved user {}", username);
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
) -> ApiResult {
    let user_data = data.into_inner();
    let password = user_data.password.clone();
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

// XXX: must ignore UserInfo.groups
#[put("/user/<username>", format = "json", data = "<data>")]
pub async fn modify_user(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    data: Json<UserInfo>,
) -> ApiResult {
    debug!("Modifing user {}", username);
    let mut user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    data.into_inner().into_user(&mut user);
    user.save(&appstate.pool).await?;
    if appstate.license.validate(&Features::Ldap) {
        let _result = ldap_modify_user(&appstate.config, username, &user).await;
    };
    info!("Modified user {}", username);
    let user_info = UserInfo::from_user(&appstate.pool, user).await?;
    appstate.trigger_action(AppEvent::UserModified(user_info));
    Ok(ApiResponse::default())
}

#[delete("/user/<username>")]
pub async fn delete_user(
    _admin: AdminRole,
    appstate: &State<AppState>,
    username: &str,
) -> ApiResult {
    debug!("Deleting user {}", username);
    match User::find_by_username(&appstate.pool, username).await? {
        Some(user) => {
            user.delete(&appstate.pool).await?;
            if appstate.license.validate(&Features::Ldap) {
                let _result = ldap_delete_user(&appstate.config, username).await;
            };
            info!("Deleted user {}", username);
            appstate.trigger_action(AppEvent::UserDeleted(username.into()));
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
    debug!("Changing password for user {}", username);
    let mut user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    user.set_password(&data.new_password);
    user.save(&appstate.pool).await?;
    if appstate.license.validate(&Features::Ldap) {
        let _result = ldap_change_password(&appstate.config, username, &data.new_password).await;
    }
    info!("Password changed for user {}", username);
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
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;

    // check if address already exists
    let wallet =
        match Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), address).await? {
            Some(wallet) => {
                if wallet.validation_timestamp.is_some() {
                    return Err(OriWebError::ObjectNotFound("wrong address".into()));
                }
                wallet
            }
            None => {
                let challenge_message = match Settings::find_by_id(&appstate.pool, 1).await? {
                    Some(settings) => settings.challenge_template,
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
            Ok(ApiResponse::default())
        } else {
            Err(OriWebError::ObjectNotFound("wrong address".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound("wallet not found".into()))
    }
}

#[put("/user/<username>/wallet/<address>", format = "json", data = "<data>")]
pub async fn update_wallet(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    address: &str,
    data: Json<WalletChange>,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    if let Some(mut wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), address).await?
    {
        if Some(wallet.user_id) == user.id {
            wallet.use_for_mfa = data.use_for_mfa;
            wallet.save(&appstate.pool).await?;
            Ok(ApiResponse::default())
        } else {
            Err(OriWebError::ObjectNotFound("wrong wallet".into()))
        }
    } else {
        Err(OriWebError::ObjectNotFound("wallet not found".into()))
    }
}

#[delete("/user/<username>/wallet/<address>")]
pub async fn delete_wallet(
    session: SessionInfo,
    appstate: &State<AppState>,
    username: &str,
    address: &str,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    if let Some(wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), address).await?
    {
        if Some(wallet.user_id) == user.id {
            wallet.delete(&appstate.pool).await?;
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
    let user = user_for_admin_or_self(&appstate.pool, &session, username).await?;
    if let Some(webauthn) = WebAuthn::find_by_id(&appstate.pool, id).await? {
        if Some(webauthn.user_id) == user.id {
            webauthn.delete(&appstate.pool).await?;
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
