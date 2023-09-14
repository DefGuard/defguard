use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use regex::Regex;
use serde_json::json;

use super::{
    user_for_admin_or_self, AddUserData, ApiResponse, ApiResult, PasswordChange,
    PasswordChangeSelf, RecoveryCodes, StartEnrollmentRequest, Username, WalletChallenge,
    WalletChange, WalletSignature,
};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{
        AppEvent, MFAMethod, OAuth2AuthorizedApp, Settings, User, UserDetails, UserInfo, Wallet,
        WebAuthn,
    },
    error::WebError,
    ldap::utils::{ldap_add_user, ldap_change_password, ldap_delete_user, ldap_modify_user},
};

/// Verify the given username
fn check_username(username: &str) -> Result<(), WebError> {
    let length = username.len();
    if !(3..64).contains(&length) {
        return Err(WebError::Serialization(format!(
            "Username ({username}) has incorrect length"
        )));
    }

    if let Some(first_char) = username.chars().next() {
        if first_char.is_ascii_digit() {
            return Err(WebError::Serialization(
                "Username must not start with a digit".into(),
            ));
        }
    }

    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(WebError::Serialization(
            "Username is not in lowercase".into(),
        ));
    }
    Ok(())
}

pub(crate) fn check_password_strength(password: &str) -> Result<(), WebError> {
    let password_length = password.len();
    let special_chars_expression = Regex::new(r#"[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>/?~]"#).unwrap();
    let numbers_expression = Regex::new(r"[0-9]").unwrap();
    let lowercase_expression = Regex::new(r"[a-z]").unwrap();
    let uppercase_expression = Regex::new(r"[A-Z]").unwrap();
    if !(8..=128).contains(&password_length) {
        return Err(WebError::Serialization("Incorrect password length".into()));
    }
    if !special_chars_expression.is_match(password) {
        return Err(WebError::Serialization(
            "No special characters in password".into(),
        ));
    }
    if !numbers_expression.is_match(password) {
        return Err(WebError::Serialization("No numbers in password".into()));
    }
    if !lowercase_expression.is_match(password) {
        return Err(WebError::Serialization(
            "No lowercase characters in password".into(),
        ));
    }
    if !uppercase_expression.is_match(password) {
        return Err(WebError::Serialization(
            "No uppercase characters in password".into(),
        ));
    }
    Ok(())
}

pub async fn list_users(_admin: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    let all_users = User::all(&appstate.pool).await?;
    let mut users: Vec<UserInfo> = Vec::with_capacity(all_users.len());
    for user in all_users {
        users.push(UserInfo::from_user(&appstate.pool, &user).await?);
    }
    Ok(ApiResponse {
        json: json!(users),
        status: StatusCode::OK,
    })
}

pub async fn get_user(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    let user_details = UserDetails::from_user(&appstate.pool, &user).await?;
    Ok(ApiResponse {
        json: json!(user_details),
        status: StatusCode::OK,
    })
}

pub async fn add_user(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(user_data): Json<AddUserData>,
) -> ApiResult {
    let username = user_data.username.clone();
    debug!("User {} adding user {username}", session.user.username);

    // check username
    if let Err(err) = check_username(&username) {
        debug!("{}", err);
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }
    let password = match &user_data.password {
        Some(password) => {
            // check password strength
            if let Err(err) = check_password_strength(password) {
                debug!("Password not strong enough: {err}");
                return Ok(ApiResponse {
                    json: json!({}),
                    status: StatusCode::BAD_REQUEST,
                });
            }
            Some(password.as_str())
        }
        None => None,
    };

    // create new user
    let mut user = User::new(
        user_data.username,
        password,
        user_data.last_name,
        user_data.first_name,
        user_data.email,
        user_data.phone,
    );
    user.save(&appstate.pool).await?;

    // add LDAP user
    // FIXME: check for LDAP being enabled
    if true {
        if let Some(password) = user_data.password {
            let _result = ldap_add_user(&appstate.config, &user, &password).await;
        }
    };

    let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
    appstate.trigger_action(AppEvent::UserCreated(user_info.clone()));
    info!("User {} added user {username}", session.user.username);
    if !user.has_password() {
        warn!("User {username} is not active yet. Please proceed with enrollment.");
    };
    Ok(ApiResponse {
        json: json!(&user_info),
        status: StatusCode::CREATED,
    })
}

// Trigger enrollment process manually
pub async fn start_enrollment(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(data): Json<StartEnrollmentRequest>,
) -> ApiResult {
    debug!(
        "User {} starting enrollment for user {username}",
        session.user.username
    );

    // validate request
    if data.send_enrollment_notification && data.email.is_none() {
        return Err(WebError::BadRequest(
            "Email notification is enabled, but email was not provided".into(),
        ));
    }

    let user = match User::find_by_username(&appstate.pool, &username).await? {
        Some(user) => Ok(user),
        None => Err(WebError::ObjectNotFound(format!(
            "user {username} not found"
        ))),
    }?;

    let mut transaction = appstate.pool.begin().await?;

    let enrollment_token = user
        .start_enrollment(
            &mut transaction,
            &session.user,
            data.email.clone(),
            appstate.config.enrollment_token_timeout.as_secs(),
            appstate.config.enrollment_url.clone(),
            data.send_enrollment_notification,
            appstate.mail_tx.clone(),
        )
        .await?;

    transaction.commit().await?;

    Ok(ApiResponse {
        json: json!({"enrollment_token": enrollment_token, "enrollment_url":  appstate.config.enrollment_url.to_string()}),
        status: StatusCode::CREATED,
    })
}

pub async fn username_available(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<Username>,
) -> ApiResult {
    if let Err(err) = check_username(&data.username) {
        debug!("{}", err);
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    };
    let status = match User::find_by_username(&appstate.pool, &data.username).await? {
        Some(_) => StatusCode::BAD_REQUEST,
        None => StatusCode::OK,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

pub async fn modify_user(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(user_info): Json<UserInfo>,
) -> ApiResult {
    debug!("User {} updating user {username}", session.user.username);
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Err(err) = check_username(&user_info.username) {
        debug!("{}", err);
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }
    // remove authorized apps if needed
    let request_app_ids: Vec<i64> = user_info
        .authorized_apps
        .iter()
        .map(|app| app.oauth2client_id)
        .collect();
    let db_apps = user.oauth2authorizedapps(&appstate.pool).await?;
    let removed_apps: Vec<i64> = db_apps
        .iter()
        .filter(|app| !request_app_ids.contains(&app.oauth2client_id))
        .map(|app| app.oauth2client_id)
        .collect();
    if !removed_apps.is_empty() {
        user.remove_oauth2_authorized_apps(&appstate.pool, &removed_apps)
            .await?;
    }
    if session.is_admin {
        user_info
            .into_user_all_fields(&appstate.pool, &mut user)
            .await?;
    } else {
        user_info.into_user_safe_fields(&mut user).await?;
    }
    user.save(&appstate.pool).await?;

    // FIXME: check for LDAP being enabled
    if true {
        let _result = ldap_modify_user(&appstate.config, &username, &user).await;
    }
    let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
    appstate.trigger_action(AppEvent::UserModified(user_info));
    info!("User {} updated user {username}", session.user.username);
    Ok(ApiResponse::default())
}

pub async fn delete_user(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} deleting user {username}", session.user.username);
    if session.user.username == username {
        debug!("User {username} attempted to delete himself");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }
    if let Some(user) = User::find_by_username(&appstate.pool, &username).await? {
        user.delete(&appstate.pool).await?;
        // FIXME: check for LDAP being enabled
        if true {
            let _result = ldap_delete_user(&appstate.config, &username).await;
        }
        appstate.trigger_action(AppEvent::UserDeleted(username.clone()));
        info!("User {} deleted user {}", session.user.username, &username);
        Ok(ApiResponse::default())
    } else {
        error!("User {username} not found");
        Err(WebError::ObjectNotFound(format!(
            "User {username} not found"
        )))
    }
}

pub async fn change_self_password(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<PasswordChangeSelf>,
) -> ApiResult {
    let mut user = session.user;
    if user.verify_password(&data.old_password).is_err() {
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    if check_password_strength(&data.new_password).is_err() {
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    user.set_password(&data.new_password);
    user.save(&appstate.pool).await?;

    // FIXME: check for LDAP being enabled
    if true {
        let _result =
            ldap_change_password(&appstate.config, &user.username, &data.new_password).await;
    }

    info!("User {} changed password.", &user.username);

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

pub async fn change_password(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(data): Json<PasswordChange>,
) -> ApiResult {
    debug!(
        "Admin {} changing password for user {username}",
        session.user.username,
    );

    if session.user.username == username {
        debug!("Cannot change own password with this endpoint.");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    if let Err(err) = check_password_strength(&data.new_password) {
        debug!("Pasword not strong enough: {err}");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }
    if let Err(err) = check_username(&username) {
        debug!("Invalid Username: {err}");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let user = User::find_by_username(&appstate.pool, &username).await?;

    if let Some(mut user) = user {
        user.set_password(&data.new_password);
        user.save(&appstate.pool).await?;
        // FIXME: check for LDAP being enabled
        if true {
            let _result =
                ldap_change_password(&appstate.config, &username, &data.new_password).await;
        }
        info!(
            "Admin {} changed password for user {username}",
            session.user.username
        );
        Ok(ApiResponse::default())
    } else {
        debug!("User not found");
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        })
    }
}

// #[get("/user/<username>/challenge?<address>&<name>&<chain_id>")]
pub async fn wallet_challenge(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    address: &str,
    name: &str,
    chain_id: i64,
) -> ApiResult {
    debug!(
        "User {} generating wallet challenge for user {username}",
        session.user.username,
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;

    // check if address already exists
    let wallet = if let Some(wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), address).await?
    {
        if wallet.validation_timestamp.is_some() {
            return Err(WebError::ObjectNotFound("wrong address".into()));
        }
        wallet
    } else {
        let challenge_message =
            if let Some(settings) = Settings::find_by_id(&appstate.pool, 1).await? {
                Wallet::format_challenge(address, &settings.challenge_template)
            } else {
                return Err(WebError::DbError("cannot retrieve settings".into()));
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
    };

    info!(
        "User {} generated wallet challenge for user {username}",
        session.user.username
    );
    Ok(ApiResponse {
        json: json!(WalletChallenge {
            id: wallet.id.unwrap(),
            message: wallet.challenge_message
        }),
        status: StatusCode::OK,
    })
}

// #[put("/user/<username>/wallet", format = "json", data = "<data>")]
pub async fn set_wallet(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(wallet_info): Json<WalletSignature>,
) -> ApiResult {
    debug!(
        "User {} setting wallet signature for user {username}",
        session.user.username
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(mut wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), &wallet_info.address)
            .await?
    {
        if wallet.validate_signature(&wallet_info.signature).is_ok() {
            wallet
                .set_signature(&appstate.pool, &wallet_info.signature)
                .await?;
            info!(
                "User {} set wallet signature for user {username}",
                session.user.username,
            );
            Ok(ApiResponse::default())
        } else {
            Err(WebError::ObjectNotFound("wrong address".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("wallet not found".into()))
    }
}

/// Change wallet.
/// Currenly only `use_for_mfa` flag can be set or unset.
// #[put("/user/<username>/wallet/<address>", format = "json", data = "<data>")]
pub async fn update_wallet(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Path(address): Path<String>,
    Json(data): Json<WalletChange>,
) -> ApiResult {
    debug!(
        "User {} updating wallet {address} for user {username}",
        session.user.username,
    );
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(mut wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), &address).await?
    {
        if Some(wallet.user_id) == user.id {
            let mfa_change = wallet.use_for_mfa != data.use_for_mfa;
            wallet.use_for_mfa = data.use_for_mfa;
            wallet.save(&appstate.pool).await?;
            if mfa_change {
                if data.use_for_mfa {
                    debug!("Wallet {} MFA flag enabled", wallet.address);
                    if !user.mfa_enabled {
                        user.set_mfa_method(&appstate.pool, MFAMethod::Web3).await?;
                        let recovery_codes = user.get_recovery_codes(&appstate.pool).await?;
                        info!("User {} MFA enabled", username);
                        info!(
                            "User {} updated wallet {address} for user {username}",
                            session.user.username,
                        );
                        return Ok(ApiResponse {
                            json: json!(RecoveryCodes::new(recovery_codes)),
                            status: StatusCode::OK,
                        });
                    }
                } else {
                    debug!("Wallet {} MFA flag removed", wallet.address);
                    user.verify_mfa_state(&appstate.pool).await?;
                }
            }
            info!(
                "User {} updated wallet {address} for user {username}",
                session.user.username,
            );
            Ok(ApiResponse::default())
        } else {
            Err(WebError::ObjectNotFound("wrong wallet".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("wallet not found".into()))
    }
}

/// Delete wallet.
// #[delete("/user/<username>/wallet/<address>")]
pub async fn delete_wallet(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    address: &str,
) -> ApiResult {
    debug!(
        "User {} deleting wallet {address} for user {username}",
        session.user.username,
    );
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), address).await?
    {
        if Some(wallet.user_id) == user.id {
            wallet.delete(&appstate.pool).await?;
            user.verify_mfa_state(&appstate.pool).await?;
            info!(
                "User {} deleted wallet {address} for user {username}",
                session.user.username,
            );
            Ok(ApiResponse::default())
        } else {
            Err(WebError::ObjectNotFound("wrong wallet".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("wallet not found".into()))
    }
}

// #[delete("/user/<username>/security_key/<id>")]
pub async fn delete_security_key(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    id: i64,
) -> ApiResult {
    debug!(
        "User {} deleting security key {id} for user {username}",
        session.user.username,
    );
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(webauthn) = WebAuthn::find_by_id(&appstate.pool, id).await? {
        if Some(webauthn.user_id) == user.id {
            webauthn.delete(&appstate.pool).await?;
            user.verify_mfa_state(&appstate.pool).await?;
            info!(
                "User {} deleted security key {id} for user {username}",
                session.user.username,
            );
            Ok(ApiResponse::default())
        } else {
            Err(WebError::ObjectNotFound("wrong security key".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("security key not found".into()))
    }
}

// #[get("/me", format = "json")]
pub async fn me(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    let user_info = UserInfo::from_user(&appstate.pool, &session.user).await?;
    Ok(ApiResponse {
        json: json!(user_info),
        status: StatusCode::OK,
    })
}

/// Delete Oauth token.
// #[delete("/user/<username>/oauth_app/<oauth2client_id>")]
pub async fn delete_authorized_app(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    oauth2client_id: i64,
) -> ApiResult {
    debug!(
        "User {} deleting OAuth2 client {oauth2client_id} for user {username}",
        session.user.username,
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
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
            Err(WebError::ObjectNotFound("Wrong app".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("Authorized app not found".into()))
    }
}
