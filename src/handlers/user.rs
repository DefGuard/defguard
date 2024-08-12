use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use serde_json::json;
use utoipa::ToSchema;

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
pub fn check_username(username: &str) -> Result<(), WebError> {
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

/// List of all users
///
/// Retrives list of users.
///
/// # Returns
/// Returns list of `UserInfo` objects or `WebError` if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/user",
    responses(
        (status = 200, description = "List of all users.", body = [UserInfo], example = json!(
        [
            {
                "authorized_apps": [],
                "email": "name@email.com",
                "email_mfa_enabled": false,
                "enrolled": true,
                "first_name": "first_name",
                "groups": [
                    "group"
                ],
                "id": 1,
                "is_active": true,
                "last_name": "last_name",
                "mfa_enabled": false,
                "mfa_method": "None",
                "phone": null,
                "totp_enabled": false,
                "username": "username"
            }
        ])),
        (status = 401, description = "Unauthorized to list all users.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to list all users.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable return list of users.", body = ApiResponse, example = json!({"msg": "Internal error"}))
    )
)]
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

/// Get user
///
/// Return a user based on provided username parameter.
///
/// # Returns
/// Returns `UserDetails` object or `WebError` if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/user/{username}",
    params(
        ("username" = String, description = "name of a user"),
    ),
    responses(
        (status = 200, description = "Return details about user.", body = UserDetails, example = json!(
            {
                "devices": [
                    {
                        "created": "date",
                        "id": 1,
                        "name": "name",
                        "networks": [
                            {
                                "device_wireguard_ip": "1.1.1.1",
                                "is_active": false,
                                "last_connected_at": null,
                                "last_connected_ip": null,
                                "last_connected_location": null,
                                "network_gateway_ip": "0.0.0.0",
                                "network_id": 1,
                                "network_name": "TestNet"
                            }
                        ],
                        "user_id": 1,
                        "wireguard_pubkey": "wireguard_pubkey"
                    }
                ],
                "security_keys": [],
                "user": {
                    "authorized_apps": [],
                    "email": "name@email.com",
                    "email_mfa_enabled": false,
                    "enrolled": true,
                    "first_name": "first_name",
                    "groups": [
                        "group"
                    ],
                    "id": 1,
                    "is_active": true,
                    "last_name": "last_name",
                    "mfa_enabled": false,
                    "mfa_method": "None",
                    "phone": null,
                    "totp_enabled": false,
                    "username": "username"
                },
                "wallets": []
            }
        )),
        (status = 401, description = "Unauthorized to return details about user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to return details about user.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to return user details.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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

/// Add user
///
/// Add a new user based on `AddUserData` object.
///
/// # Returns
/// Returns `UserInfo` object or `WebError` if error occurs.
#[utoipa::path(
    post,
    path = "/api/v1/user",
    request_body = AddUserData,
    responses(
        (status = 201, description = "Add a new user.", body = UserInfo, example = json!(
            {
                "authorized_apps": [],
                "email": "name@email.com",
                "email_mfa_enabled": false,
                "enrolled": true,
                "first_name": "first_name",
                "groups": [
                    "admin"
                ],
                "id": 1,
                "is_active": true,
                "last_name": "last_name",
                "mfa_enabled": false,
                "mfa_method": "None",
                "phone": null,
                "totp_enabled": false,
                "username": "username"
            }
        )),
        (status = 400, description = "Bad request, invalid user data.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to create a user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to create a user.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to create a user.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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
        debug!("Username {username} rejected: {err}");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }
    // check if email doesn't already exist
    if User::find_by_email(&appstate.pool, &user_data.email)
        .await?
        .is_some()
    {
        debug!("User with email {} already exists", user_data.email);
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

/// Trigger enrollment process manually
///
/// Allows admin to start new enrollment for user that is provided as a parameter in endpoint.
///
/// Thanks to this endpoint you are able to trigger manually enrollment process, where after finishing you receive an enrollment token.
///
/// `Enrollment token` allows to start the process of gaining access to the company infrastructure `(The enrollment token is valid for 24 hours)`. On the other hand, enrollment url allows the user to access the enrollment form via the web browser or perform the enrollment through the desktop client.
///
/// Optionally this endpoint can send an email notification to the user about the enrollment.
/// # Returns
/// Returns json with `enrollment token` and `enrollment url` or `WebError` if error occurs.
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/start_enrollment",
    request_body = StartEnrollmentRequest,
    responses(
        (status = 201, description = "Trigger enrollment process manually.", body = ApiResponse, example = json!({"enrollment_token": "your_enrollment_token", "enrollment_url": "your_enrollment_token"})),
        (status = 400, description = "Bad request, invalid enrollment request.", body = ApiResponse, example = json!({"msg": "Email notification is enabled, but email was not provided"})),
        (status = 401, description = "Unauthorized to start enrollment.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to start enrollment.", body = Json, example = json!({"msg": "access denied"})),
        (status = 404, description = "Provided user does not exist.", body = Json, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to start enrollment.", body = Json, example = json!({"msg": "unexpected error"}))
    )
)]
pub async fn start_enrollment(
    _role: UserAdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(data): Json<StartEnrollmentRequest>,
) -> ApiResult {
    debug!(
        "User {} has started a new enrollment request.",
        session.user.username
    );

    // validate request
    if data.send_enrollment_notification && data.email.is_none() {
        error!(
            "Email notification is enabled for user {}, but email was not provided",
            session.user.username
        );
        return Err(WebError::BadRequest(
            "Email notification is enabled, but email was not provided".into(),
        ));
    }

    debug!(
        "Search for the user {} in database to get started with enrollment process.",
        username
    );
    let Some(user) = User::find_by_username(&appstate.pool, &username).await? else {
        error!("User {username} couldn't be found, enrollment aborted");
        return Err(WebError::ObjectNotFound(format!(
            "user {username} not found"
        )));
    };

    debug!("Create a new database transaction to save a new enrollment token into the database.");
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

    debug!("Try to submit transaction to save the enrollment token into the databse.");
    transaction.commit().await?;
    debug!("Transaction submitted.");

    info!(
        "The enrollment process for {} has ended with success.",
        session.user.username
    );
    debug!(
        "Enrollment token {}, enrollment url {}",
        enrollment_token,
        config.enrollment_url.to_string()
    );

    Ok(ApiResponse {
        json: json!({"enrollment_token": enrollment_token, "enrollment_url": config.enrollment_url.to_string()}),
        status: StatusCode::CREATED,
    })
}

/// Start remote desktop configuration
///
/// Allows admin to start new remote desktop configuration for user that is provided as a parameter in endpoint.
///
/// Thanks to this endpoint you are able to receive a new desktop client configuration or update an existing one. Users need the configuration to connect to the company infrastrcture.
///
/// `Enrollment token` allows to start the process of gaining access to the company infrastructure `(The enrollment token is valid for 24 hours)`. On the other hand, enrollment url allows the user to access the enrollment form via the web browser or perform the enrollment through the desktop client.
///
/// Optionally this endpoint can send an email notification to the user about the enrollment.```
/// # Returns
/// Returns json with `enrollment token` and `enrollment url` or `WebError` if error occurs.
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/start_desktop",
    request_body = StartEnrollmentRequest,
    responses(
        (status = 201, description = "Trigger enrollment process manually.", body = Json, example = json!({"enrollment_token": "your_enrollment_token", "enrollment_url": "your_enrollment_token"})),
        (status = 400, description = "Bad request, invalid enrollment request.", body = Json, example = json!({"msg": "Email notification is enabled, but email was not provided"})),
        (status = 401, description = "Unauthorized to start remote desktop configuration.", body = Json, example = json!({"msg": "Can't create desktop configuration enrollment token for disabled user <username>"})),
        (status = 404, description = "Provided user does not exist.", body = Json, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to start remote desktop configuration.", body = Json, example = json!({"msg": "unexpected error"}))
    )
)]
pub async fn start_remote_desktop_configuration(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(data): Json<StartEnrollmentRequest>,
) -> ApiResult {
    debug!(
        "User {} has started a new desktop activation request.",
        session.user.username
    );

    debug!("Verify that the user from the current session is an admin or only peforms desktop activation for self.");
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    debug!("Successfully fetched user data: {user:?}");

    // if email is None assume that email should be sent to enrolling user
    let email = match data.email {
        Some(email) => email,
        None => user.email.clone(),
    };

    debug!("Create a new database transaction to save a desktop configuration token into the database.");
    let mut transaction = appstate.pool.begin().await?;

    debug!(
        "Generating a new desktop activation token by {}.",
        session.user.username
    );
    let config = server_config();
    let desktop_configuration_token = user
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

    debug!("Try to submit transaction to save the desktop configuration token into the databse.");
    transaction.commit().await?;
    debug!("Transaction submitted.");

    info!(
        "User {} added a new desktop activation.",
        session.user.username
    );
    debug!(
        "Desktop configuration token {}, desktop configuration url {}",
        desktop_configuration_token,
        config.enrollment_url.to_string()
    );

    Ok(ApiResponse {
        json: json!({"enrollment_token": desktop_configuration_token, "enrollment_url":  config.enrollment_url.to_string()}),
        status: StatusCode::CREATED,
    })
}

/// Verify if the user is available
///
/// Check if user is available by provided `Username` object.
/// Username is unique so database returns only single user or nothing.
///
/// # Returns
/// Returns only status code 200 if user is available or `WebError` if error occurs.
///
/// `Please take notice that if user exists in database, endpoint will return status code 400.`
#[utoipa::path(
    post,
    path = "/api/v1/user/available",
    request_body = Json<Username>,
    responses(
        (status = 200, description = "Provided username is available to use.", body = ApiResponse, example = json!({})),
        (status = 400, description = "Bad request, provided username is not available or username is invalid.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to check is username available.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to check is username available.", body = ApiResponse,  example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to check is username available.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
pub async fn username_available(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<Username>,
) -> ApiResult {
    if let Err(err) = check_username(&data.username) {
        debug!("Username {} rejected: {err}", data.username);
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    };
    let status = match User::find_by_username(&appstate.pool, &data.username).await? {
        Some(_) => {
            debug!("Username {} is not available", data.username);
            StatusCode::BAD_REQUEST
        }
        None => StatusCode::OK,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

/// Modify user
///
/// Update users data, it can remove authorized apps and active/deactivate ldap status if needed.
/// Endpoint is able to disable a user, but `admin cannot disable himself`.
///
/// # Returns
/// If erorr occurs, endpoint will return `WebError` object.
#[utoipa::path(
    put,
    path = "/api/v1/user/{username}",
    params(
        ("username" = String, description = "name of a user"),
    ),
    request_body = Json<UserInfo>,
    responses(
        (status = 200, description = "User has been updated."),
        (status = 400, description = "Bad request, unable to change user data. Verify user data that you want to update.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to modify user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to modify user.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
pub async fn modify_user(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(mut user_info): Json<UserInfo>,
) -> ApiResult {
    debug!("User {} updating user {username}", session.user.username);
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Err(err) = check_username(&user_info.username) {
        debug!("Username {} rejected: {err}", user_info.username);
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
        if session.user.username == username && !user_info.is_active {
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
            debug!(
                "User {} has started a new enrollment request.",
                session.user.username
            );
            let networks = WireguardNetwork::all(&mut *transaction).await?;
            for network in networks {
                let gateway_events = network.sync_allowed_devices(&mut transaction, None).await?;
                appstate.send_multiple_wireguard_events(gateway_events);
            }
            info!("Allowed network devices of {username} synced");
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

/// Delete user
///
/// Endpoint helps you delete a user, but `you can't delete yourself as a administrator`.
///
/// # Returns
/// If erorr occurs, endpoint will return `WebError` object.
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}",
    params(
        ("username" = String, description = "name of a user"),
    ),
    responses(
        (status = 200, description = "User has been deleted."),
        (status = 400, description = "Bad request, unable to delete user.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to delete user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to delete user.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "User does not exist with username: <username>", body = ApiResponse, example = json!({"msg": "User <username> not found"})),
        (status = 500, description = "Unable to delete user.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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

/// Change your own password
///
/// Change your own password, it could return error if password is not strong enough.
///
/// # Returns
/// If erorr occurs, endpoint will return `WebError` object.
#[utoipa::path(
    put,
    path = "/api/v1/user/change_password",
    request_body = Json<PasswordChangeSelf>,
    responses(
        (status = 200, description = "Pasword has been changed.", body = ApiResponse, example = json!({})),
        (status = 400, description = "Bad request, provided passwords are not same or new password does not satisfy requirements.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to change password.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to change your password", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
pub async fn change_self_password(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<PasswordChangeSelf>,
) -> ApiResult {
    debug!("User {} is changing his password.", session.user.username);
    let mut user = session.user;
    if user.verify_password(&data.old_password).is_err() {
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    if let Err(err) = check_password_strength(&data.new_password) {
        debug!("User {} password change failed: {err}", user.username);
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    user.set_password(&data.new_password);
    user.save(&appstate.pool).await?;

    let _ = ldap_change_password(&appstate.pool, &user.username, &data.new_password).await;

    info!("User {} changed his password.", &user.username);

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

/// Change user password
///
/// Change user password, it could return error if password is not strong enough.
///
/// `This endpoint doesn't allow you to change your own password. Go to: /api/v1/user/change_password.`
///
/// # Returns
/// If erorr occurs, endpoint will return `WebError` object.
#[utoipa::path(
    put,
    path = "/api/v1/user/{username}/password",
    params(
        ("username" = String, description = "name of a user"),
    ),
    request_body = Json<PasswordChange>,
    responses(
        (status = 200, description = "Pasword has been changed.", body = ApiResponse, example = json!({})),
        (status = 400, description = "Bad request, password does not satisfy requirements. This endpoint does not change your own password.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to change password.", body = Json, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to change user password.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Cannot change user password that does not exist.", body = ApiResponse, example = json!({})),
        (status = 500, description = "Unable to change user password", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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
        debug!("Cannot change own ({username}) password with this endpoint.");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    if let Err(err) = check_password_strength(&data.new_password) {
        debug!("Password for user {username} not strong enough: {err}");
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }
    if let Err(err) = check_username(&username) {
        debug!("Invalid username ({username}): {err}");
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
        debug!("Can't change password for user {username}, user not found");
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        })
    }
}

/// Reset user password
///
/// Reset user password, it will send a new enrollment to the user's email.
///
/// `This endpoint doesn't allow you to reset your own password.`
///
/// # Returns
/// If erorr occurs, endpoint will return `WebError` object.
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/reset_password",
    params(
        ("username" = String, description = "name of a user"),
    ),
    responses(
        (status = 200, description = "Successfully reset user password."),
        (status = 400, description = "Bad request, this endpoint does not change your own password.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to change password.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to change user password.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Cannot reset user password that does not exist.", body = ApiResponse, example = json!({})),
        (status = 500, description = "Unable to send reset password to email", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
pub async fn reset_password(
    _role: UserAdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    debug!(
        "Admin {} resetting password for user {username}",
        session.user.username,
    );

    if session.user.username == username {
        debug!("Cannot reset own ({username}) password with this endpoint.");
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
                info!("Password reset email for {username} sent to {to}");
                Ok(())
            }
            Err(err) => {
                error!(
                    "Failed to send password reset email for {username} to {to} with error: {err}"
                );
                Err(WebError::Serialization(format!(
                    "Could not send password reset email to user {username}"
                )))
            }
        }?;

        transaction.commit().await?;

        info!(
            "Admin {} reset password for user {username}",
            session.user.username
        );
        Ok(ApiResponse::default())
    } else {
        debug!("Can't reset password for user {username}, user not found");
        Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::NOT_FOUND,
        })
    }
}

/// Similar to [`models::WalletInfo`] but without `use_for_mfa`.
#[derive(Deserialize, ToSchema)]
pub struct WalletInfoShort {
    pub address: String,
    pub name: String,
    pub chain_id: i64,
}

/// Wallet challenge
///
/// Endpoint allows to generate a wallet challenge for ownership verification.
///
/// # Returns
/// Returns `WalletChallenge` object or `WebError` object if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/user/{username}/challenge",
    params(
        ("username" = String, description = "name of a user"),
    ),
    responses(
        (status = 200, description = "Return successfully wallet challenge details.", body = WalletChallenge, example = json!(
            {
                "id": 1,
                "message": "message"
            }
        )),
        (status = 401, description = "Unauthorized to return wallet challenge.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Wrong address or wallet challenge is alredy validated.", body = ApiResponse, example = json!({"msg": "wrong address"})),
        (status = 500, description = "Cannot retrive settings.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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
            error!(
                "Can't generate wallet challange for user {username}, the wallet {} is already validated",
                wallet_info.address
                );
            return Err(WebError::ObjectNotFound("wrong address".into()));
        }
        wallet
    } else {
        let challenge_message =
            if let Some(settings) = Settings::find_by_id(&appstate.pool, 1).await? {
                Wallet::format_challenge(&wallet_info.address, &settings.challenge_template)
            } else {
                error!("Cannot retrieve settings");
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

/// Set wallet
///
/// Set a new signature to user wallet by providing `WalletSignature` object.
///
/// # Returns
/// It returns `WebError` object if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/user/{username}/wallet",
    params(
        ("username" = String, description = "name of a user"),
    ),
    responses(
        (status = 200, description = "Successfully set wallet signature."),
        (status = 401, description = "Unauthorized to set a new signature.", body = Json, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to set new signature to wallet.", body = Json, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Incorrect wallet signature or address, can't set new signature for user.", body = ApiResponse, example = json!({"msg": "wallet not found"})),
        (status = 500, description = "Cannot set a new wallet signature", body = Json, example = json!({"msg": "Internal server error"}))
    )
)]
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
            error!(
                "User {} failed to set wallet signature for user {username}, wallet ({}) signature {} is invalid",
                session.user.username, wallet_info.address, wallet_info.signature
            );
            Err(WebError::ObjectNotFound("wrong address".into()))
        }
    } else {
        error!(
            "User {} failed to set wallet signature for user {username}, address {} not found",
            session.user.username, wallet_info.address
        );
        Err(WebError::ObjectNotFound("wallet not found".into()))
    }
}

/// Change wallet.
///
/// Updates user wallet.
/// Currently only `use_for_mfa` flag can be set or unset.
///
/// # Returns
/// Returns `RecoveryCodes` object or `WebError` object if error occurs.
#[utoipa::path(
    put,
    path = "/api/v1/user/{username}/wallet/{address}",
    params(
        ("username" = String, description = "name of a user"),
        ("address" = String, description = "address of a user portfel")
    ),
    request_body = Json<WalletChange>,
    responses(
        (status = 200, description = "Successfully updated user's wallet.", body = RecoveryCodes, example = json!({"codes": "[]"})),
        (status = 401, description = "Unauthorized to udpate user wallet.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to update user wallet.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Incorrect wallet, can't update user wallet.", body = ApiResponse, example = json!({"msg": "wallet not found"})),
        (status = 500, description = "Cannot udpate user wallet.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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
            error!(
                "User {} failed to update wallet {address} for user {username} (id: {:?}), the owner id is {}",
                session.user.username, user.id, wallet.user_id
            );
            Err(WebError::ObjectNotFound("wrong wallet".into()))
        }
    } else {
        error!(
            "User {} failed to update wallet {address} for user {username}, wallet not found",
            session.user.username
        );
        Err(WebError::ObjectNotFound("wallet not found".into()))
    }
}

/// Delete wallet.
///
/// Endpoint helps you to delete user wallet.
///
/// # Returns
/// Returns `WebError` object if error occurs.
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/wallet/{address}",
    params(
        ("username" = String, description = "name of a user"),
        ("address" = String, description = "address of a user portfel")
    ),
    responses(
        (status = 200, description = "Successfully deleted user's wallet."),
        (status = 401, description = "Unauthorized to delete user wallet.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to delete user wallet.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Incorrect wallet, can't delete user wallet.", body = ApiResponse, example = json!({"msg": "wallet not found"})),
        (status = 500, description = "Cannot delete user wallet.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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
            error!(
                "User {} failed to delete wallet {address} for user {username} (id: {:?}), the owner id is {}",
                session.user.username, user.id, wallet.user_id
            );
            Err(WebError::ObjectNotFound("wrong wallet".into()))
        }
    } else {
        error!(
            "User {} failed to delete wallet {address} for user {username}, wallet not found",
            session.user.username
        );
        Err(WebError::ObjectNotFound("wallet not found".into()))
    }
}

/// Delete security key
///
/// Delete Webauthn security key that allows users to authenticate.
///
/// # Returns
/// Returns `WebError` object if error occurs.
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/security_key/{id}",
    params(
        ("username" = String, description = "name of a user"),
        ("id" = i64, description = "id of security key that could point to passkey")
    ),
    responses(
        (status = 200, description = "Successfully deleted security key."),
        (status = 401, description = "Unauthorized to delete security key.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to delete security key.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Incorrect authorized app, not found.", body = ApiResponse, example = json!({"msg": "security key not found"})),
        (status = 500, description = "Cannot delete authorized app.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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
            error!(
                "User {} failed to delete security key {id} for user {username} (id: {:?}), the owner id is {}",
                session.user.username, user.id, webauthn.user_id
            );
            Err(WebError::ObjectNotFound("wrong security key".into()))
        }
    } else {
        error!(
            "User {} failed to delete security key {id} for user {username}, security key not found",
            session.user.username
        );
        Err(WebError::ObjectNotFound("security key not found".into()))
    }
}

/// Returns your data
///
/// Endpoint returns the data associated with the current session user```
///
/// # Returns
/// Returns `UserInfo` object or `WebError` object if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/me",
    responses(
        (status = 200, description = "Returns your own data.", body = UserInfo, example = json!(
            {
                "authorized_apps": [],
                "email": "name@email.com",
                "email_mfa_enabled": false,
                "enrolled": true,
                "first_name": "first_name",
                "groups": [
                    "group"
                ],
                "id": 1,
                "is_active": true,
                "last_name": "last_name",
                "mfa_enabled": false,
                "mfa_method": "None",
                "phone": null,
                "totp_enabled": false,
                "username": "username"
            }
        )),
        (status = 401, description = "Unauthorized return own user data.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Cannot retrive own user data.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
pub async fn me(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    let user_info = UserInfo::from_user(&appstate.pool, &session.user).await?;
    Ok(ApiResponse {
        json: json!(user_info),
        status: StatusCode::OK,
    })
}

/// Delete Oauth token.
///
/// Endpoint helps your to delete authorized application by `OAuth2` id.
///
/// # Returns
/// Returns `WebError` object if error occurs.
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/oauth_app/{oauth2client_id}",
    params(
        ("username" = String, description = "name of a user"),
        ("oauth2client_id" = i64, description = "id of OAuth2 client")
    ),
    responses(
        (status = 200, description = "Successfully deleted authorized app."),
        (status = 401, description = "Unauthorized to delete authorized app.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to delete authorized app.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Incorrect authorized app, not found.", body = ApiResponse, example = json!({"msg": "Authorized app not found"})),
        (status = 500, description = "Cannot delete authorized app.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    )
)]
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
            error!(
                "User {} failed to delete OAuth2 client {oauth2client_id} for user {username} (id: {:?}), the app owner id is {}",
                session.user.username, user.id, app.user_id
            );
            Err(WebError::ObjectNotFound("Wrong app".into()))
        }
    } else {
        error!(
            "User {} failed to delete OAuth2 client {oauth2client_id} for user {username}, authorized app not found",
            session.user.username
        );
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
