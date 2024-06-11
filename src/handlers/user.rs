use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use serde_json::json;

use super::{
    mail::{send_mfa_configured_email, EMAIL_PASSOWRD_RESET_START_SUBJECT},
    user_for_admin_or_self, AddUserData, ApiResponse, ApiResult, PasswordChange,
    PasswordChangeSelf, RecoveryCodes, StartEnrollmentRequest, Username, WalletChallenge,
    WalletChange, WalletSignature,
};
use crate::{
    appstate::AppState,
    auth::{SessionInfo, UserAdminRole},
    db::{
        models::enrollment::{Token, PASSWORD_RESET_TOKEN_TYPE},
        AppEvent, MFAMethod, OAuth2AuthorizedApp, Settings, User, UserDetails, UserInfo, Wallet,
        WebAuthn, WireguardNetwork,
    },
    error::WebError,
    ldap::utils::{ldap_add_user, ldap_change_password, ldap_delete_user, ldap_modify_user},
    mail::Mail,
    server_config, templates,
};

/// Verify the given username
///
/// To enable LDAP sync usernames need to avoid reserved characters.
/// Username requirements:
/// - 3 - 64 characters long
/// - lowercase or uppercase latin alphabet letters (A-Z, a-z)
/// - digits (0-9)
/// - starts with non-special character
/// - special characters: . - _
/// - no whitespaces
fn check_username(username: &str) -> Result<(), WebError> {
    // check length
    let length = username.len();
    if !(3..64).contains(&length) {
        return Err(WebError::Serialization(format!(
            "Username ({username}) has incorrect length"
        )));
    }

    // check first character is a letter or digit
    if let Some(first_char) = username.chars().next() {
        if !first_char.is_ascii_alphanumeric() {
            return Err(WebError::Serialization(
                "Username must not start with a special character".into(),
            ));
        }
    }

    // check if username contains only valid characters
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(WebError::Serialization(
            "Username contains invalid characters".into(),
        ));
    }

    Ok(())
}

pub(crate) fn check_password_strength(password: &str) -> Result<(), WebError> {
    if !(8..=128).contains(&password.len()) {
        return Err(WebError::Serialization("Incorrect password length".into()));
    }
    if !password.chars().any(|c| c.is_ascii_punctuation()) {
        return Err(WebError::Serialization(
            "No special characters in password".into(),
        ));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(WebError::Serialization("No numbers in password".into()));
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(WebError::Serialization(
            "No lowercase characters in password".into(),
        ));
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(WebError::Serialization(
            "No uppercase characters in password".into(),
        ));
    }
    Ok(())
}

pub async fn list_users(_role: UserAdminRole, State(appstate): State<AppState>) -> ApiResult {
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
    _role: UserAdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(user_data): Json<AddUserData>,
) -> ApiResult {
    let username = user_data.username.clone();
    debug!("User {} adding user {username}", session.user.username);

    // check username
    if let Err(err) = check_username(&username) {
        debug!("{err}");
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

    if let Some(password) = user_data.password {
        let _result = ldap_add_user(&appstate.pool, &user, &password).await;
    }

    let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
    appstate.trigger_action(AppEvent::UserCreated(user_info.clone()));
    info!("User {} added user {username}", session.user.username);
    if !user_info.enrolled {
        warn!("User {username} hasn't been enrolled yet. Please proceed with enrollment.");
    };
    Ok(ApiResponse {
        json: json!(&user_info),
        status: StatusCode::CREATED,
    })
}

// Trigger enrollment process manually
pub async fn start_enrollment(
    _role: UserAdminRole,
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

    let Some(user) = User::find_by_username(&appstate.pool, &username).await? else {
        return Err(WebError::ObjectNotFound(format!(
            "user {username} not found"
        )));
    };

    let mut transaction = appstate.pool.begin().await?;

    let config = server_config();
    let enrollment_token = user
        .start_enrollment(
            &mut transaction,
            &session.user,
            data.email,
            config.enrollment_token_timeout.as_secs(),
            config.enrollment_url.clone(),
            data.send_enrollment_notification,
            appstate.mail_tx.clone(),
        )
        .await?;

    transaction.commit().await?;

    Ok(ApiResponse {
        json: json!({"enrollment_token": enrollment_token, "enrollment_url":  config.enrollment_url.to_string()}),
        status: StatusCode::CREATED,
    })
}

pub async fn start_remote_desktop_configuration(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(data): Json<StartEnrollmentRequest>,
) -> ApiResult {
    debug!(
        "User {} starting enrollment for user {username}",
        session.user.username
    );

    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;

    // if email is None assume that email should be sent to enrolling user
    let email = match data.email {
        Some(email) => email,
        None => user.email.clone(),
    };

    let mut transaction = appstate.pool.begin().await?;

    let config = server_config();
    let enrollment_token = user
        .start_remote_desktop_configuration(
            &mut transaction,
            &session.user,
            Some(email),
            config.enrollment_token_timeout.as_secs(),
            config.enrollment_url.clone(),
            data.send_enrollment_notification,
            appstate.mail_tx.clone(),
        )
        .await?;

    transaction.commit().await?;

    Ok(ApiResponse {
        json: json!({"enrollment_token": enrollment_token, "enrollment_url":  config.enrollment_url.to_string()}),
        status: StatusCode::CREATED,
    })
}

pub async fn username_available(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<Username>,
) -> ApiResult {
    if let Err(err) = check_username(&data.username) {
        debug!("{err}");
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
    Json(mut user_info): Json<UserInfo>,
) -> ApiResult {
    debug!("User {} updating user {username}", session.user.username);
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Err(err) = check_username(&user_info.username) {
        debug!("Failed to check username {} {err}", user_info.username);
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let mut transaction = appstate.pool.begin().await?;

    // remove authorized apps if needed
    let request_app_ids: Vec<i64> = user_info
        .authorized_apps
        .iter()
        .map(|app| app.oauth2client_id)
        .collect();
    let db_apps = user.oauth2authorizedapps(&mut *transaction).await?;
    let removed_apps: Vec<i64> = db_apps
        .iter()
        .filter(|app| !request_app_ids.contains(&app.oauth2client_id))
        .map(|app| app.oauth2client_id)
        .collect();
    if !removed_apps.is_empty() {
        user.remove_oauth2_authorized_apps(&mut *transaction, &removed_apps)
            .await?;
    }
    if session.is_admin {
        // prevent admin from disabling himself
        if session.user.username == username {
            debug!("Admin {username} attempted to disable himself");
            return Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::BAD_REQUEST,
            });
        }

        // update VPN gateway config if user status or groups have changed
        if user_info
            .handle_user_groups(&mut transaction, &mut user)
            .await?
            || user_info
                .handle_status_change(&mut transaction, &mut user)
                .await?
        {
            let networks = WireguardNetwork::all(&mut *transaction).await?;
            for network in networks {
                let gateway_events = network.sync_allowed_devices(&mut transaction, None).await?;
                appstate.send_multiple_wireguard_events(gateway_events);
            }
        };
        user_info.into_user_all_fields(&mut user)?;
    } else {
        user_info.into_user_safe_fields(&mut user)?;
    }
    user.save(&mut *transaction).await?;

    // TODO: Reflect user status (active/disabled) modification in ldap
    let _result = ldap_modify_user(&appstate.pool, &username, &user).await;
    let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
    appstate.trigger_action(AppEvent::UserModified(user_info));

    transaction.commit().await?;

    info!("User {} updated user {username}", session.user.username);
    Ok(ApiResponse::default())
}

pub async fn delete_user(
    _role: UserAdminRole,
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
        let _result = ldap_delete_user(&appstate.pool, &username).await;
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

    let _ = ldap_change_password(&appstate.pool, &user.username, &data.new_password).await;

    info!("User {} changed password.", &user.username);

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

pub async fn change_password(
    _role: UserAdminRole,
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
        let _ = ldap_change_password(&appstate.pool, &username, &data.new_password).await;
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

pub async fn reset_password(
    _role: UserAdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
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

    let user = User::find_by_username(&appstate.pool, &username).await?;

    if let Some(user) = user {
        let mut transaction = appstate.pool.begin().await?;

        Token::delete_unused_user_password_reset_tokens(
            &mut transaction,
            user.id.expect("Missing user ID"),
        )
        .await?;

        let config = server_config();
        let enrollment = Token::new(
            user.id.expect("Missing user ID"),
            Some(session.user.id.expect("Missing admin ID")),
            Some(user.email.clone()),
            config.password_reset_token_timeout.as_secs(),
            Some(PASSWORD_RESET_TOKEN_TYPE.to_string()),
        );
        enrollment.save(&mut transaction).await?;

        let mail = Mail {
            to: user.email.clone(),
            subject: EMAIL_PASSOWRD_RESET_START_SUBJECT.into(),
            content: templates::email_password_reset_mail(
                config.enrollment_url.clone(),
                enrollment.id.clone().as_str(),
                None,
                None,
            )?,
            attachments: Vec::new(),
            result_tx: None,
        };

        let to = mail.to.clone();

        match &appstate.mail_tx.send(mail) {
            Ok(()) => {
                info!("Password reset email sent to {to}");
                Ok(())
            }
            Err(err) => {
                error!("Failed to send password reset email to {to} with error:\n{err}");
                Err(WebError::Serialization(format!(
                    "Could not send password reset email to user {username}"
                )))
            }
        }?;

        transaction.commit().await?;

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

/// Similar to [`models::WalletInfo`] but without `use_for_mfa`.
#[derive(Deserialize)]
pub struct WalletInfoShort {
    pub address: String,
    pub name: String,
    pub chain_id: i64,
}

pub async fn wallet_challenge(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Query(wallet_info): Query<WalletInfoShort>,
) -> ApiResult {
    debug!(
        "User {} generating wallet challenge for user {username}",
        session.user.username,
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;

    // check if address already exists
    let wallet = if let Some(wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), &wallet_info.address)
            .await?
    {
        if wallet.validation_timestamp.is_some() {
            return Err(WebError::ObjectNotFound("wrong address".into()));
        }
        wallet
    } else {
        let challenge_message =
            if let Some(settings) = Settings::find_by_id(&appstate.pool, 1).await? {
                Wallet::format_challenge(&wallet_info.address, &settings.challenge_template)
            } else {
                return Err(WebError::DbError("cannot retrieve settings".into()));
            };
        let mut wallet = Wallet::new_for_user(
            user.id.unwrap(),
            wallet_info.address,
            wallet_info.name,
            wallet_info.chain_id,
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
/// Currently only `use_for_mfa` flag can be set or unset.
pub async fn update_wallet(
    session: SessionInfo,
    Path((username, address)): Path<(String, String)>,
    State(appstate): State<AppState>,
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
                        // send notification email about enabled MFA
                        send_mfa_configured_email(
                            Some(&session.session),
                            &user,
                            &MFAMethod::Web3,
                            &appstate.mail_tx,
                        )?;
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
pub async fn delete_wallet(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path((username, address)): Path<(String, String)>,
) -> ApiResult {
    debug!(
        "User {} deleting wallet {address} for user {username}",
        session.user.username,
    );
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(wallet) =
        Wallet::find_by_user_and_address(&appstate.pool, user.id.unwrap(), &address).await?
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

pub async fn delete_security_key(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path((username, id)): Path<(String, i64)>,
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

pub async fn me(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    let user_info = UserInfo::from_user(&appstate.pool, &session.user).await?;
    Ok(ApiResponse {
        json: json!(user_info),
        status: StatusCode::OK,
    })
}

/// Delete Oauth token.
pub async fn delete_authorized_app(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path((username, oauth2client_id)): Path<(String, i64)>,
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
                "User {} deleted OAuth2 client {oauth2client_id} for user {username}",
                session.user.username,
            );
            Ok(ApiResponse::default())
        } else {
            Err(WebError::ObjectNotFound("Wrong app".into()))
        }
    } else {
        Err(WebError::ObjectNotFound("Authorized app not found".into()))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_username_validation() {
        // valid usernames
        assert_ok!(check_username("zenek34"));
        assert_ok!(check_username("zenekXXX__"));
        assert_ok!(check_username("first.last"));
        assert_ok!(check_username("First_Last"));
        assert_ok!(check_username("32zenek"));
        assert_ok!(check_username("32-zenek"));

        // invalid usernames
        assert_err!(check_username("a"));
        assert_err!(check_username("32"));
        assert_err!(check_username("a4"));
        assert_err!(check_username("__zenek"));
        assert_err!(check_username("zenek?"));
        assert_err!(check_username("MeMeMe!"));
        assert_err!(check_username(
            "averylongnameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        ));
    }
}
