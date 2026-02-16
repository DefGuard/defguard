use std::collections::HashSet;

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use defguard_common::{
    db::{
        Id,
        models::{
            BiometricAuth, OAuth2AuthorizedApp, Settings, User, WebAuthn, device::UserDevice,
            user::SecurityKey,
        },
    },
    types::{group_diff::GroupDiff, user_info::UserInfo},
};
use defguard_mail::{Mail, templates};
use humantime::parse_duration;
use serde_json::json;
use sqlx::{Error as SqlxError, PgPool};
use utoipa::ToSchema;

use super::{
    AddUserData, ApiResponse, ApiResult, PasswordChange, PasswordChangeSelf,
    StartEnrollmentRequest, Username, mail::EMAIL_PASSWORD_RESET_START_SUBJECT,
    user_for_admin_or_self,
};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{
        AppEvent,
        models::enrollment::{PASSWORD_RESET_TOKEN_TYPE, Token},
    },
    enrollment_management::{start_desktop_configuration, start_user_enrollment},
    enterprise::{
        db::models::api_tokens::ApiToken,
        handlers::CanManageDevices,
        ldap::{
            model::{ldap_sync_allowed_for_user, maybe_update_rdn},
            utils::{
                ldap_add_user, ldap_add_user_to_groups, ldap_change_password, ldap_delete_user,
                ldap_handle_user_modify, ldap_remove_user_from_groups, ldap_update_user_state,
            },
        },
        license::get_cached_license,
        limits::{get_counts, update_counts},
    },
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    is_valid_phone_number, server_config,
    user_management::{delete_user_and_cleanup_devices, sync_allowed_user_devices},
};

/// The maximum length for the commonName (CN) attribute in LDAP schemas is commonly set to 64
/// characters according to the X.520 standard and many LDAP implementations like Active Directory.
pub(crate) const MAX_USERNAME_CHARS: usize = 64;

/// Verify the given username
///
/// To enable LDAP sync usernames need to avoid reserved characters.
/// Username requirements:
/// - 1 - MAX_USERNAME_CHARS characters long
/// - lowercase or uppercase latin alphabet letters (A-Z, a-z)
/// - digits (0-9)
/// - starts with non-special character
/// - special characters: . - _
/// - no whitespaces
pub fn check_username(username: &str) -> Result<(), WebError> {
    // check length
    let length = username.len();
    if !(1..MAX_USERNAME_CHARS).contains(&length) {
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

pub fn check_password_strength(password: &str) -> Result<(), WebError> {
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

// Full user info with related objects
#[derive(Deserialize, Serialize, Debug, ToSchema)]
pub struct UserDetails {
    pub user: UserInfo,
    #[serde(default)]
    pub devices: Vec<UserDevice>,
    pub biometric_enabled_devices: Vec<i64>,
    #[serde(default)]
    pub security_keys: Vec<SecurityKey>,
}

impl UserDetails {
    pub async fn from_user(pool: &PgPool, user: &User<Id>) -> Result<Self, SqlxError> {
        let devices = user.user_devices(pool).await?;
        let security_keys = user.security_keys(pool).await?;
        let biometric_enabled_devices = BiometricAuth::find_by_user_id(pool, user.id)
            .await?
            .iter()
            .map(|a| a.device_id)
            .collect::<Vec<_>>();
        Ok(Self {
            user: UserInfo::from_user(pool, user).await?,
            devices,
            security_keys,
            biometric_enabled_devices,
        })
    }
}

/// List of all users
///
/// Retrieves list of users.
///
/// # Returns
/// - List of `UserInfo` objects.
///
/// - `WebError` if error occurs
#[utoipa::path(
    get,
    path = "/api/v1/user",
    responses(
        (status = 200, description = "List of all users.", body = [UserInfo], example = json!(
        [
            {
              "authorized_apps": [],
                "email": "mail@mail",
                "email_mfa_enabled": false,
                "enrolled": true,
                "first_name": "first_name",
                "groups": [
                  "admin"
                ],
                "id": 1,
                "is_active": true,
                "is_admin": true,
                "last_name": "last_name",
                "ldap_pass_requires_change": false,
                "mfa_enabled": false,
                "mfa_method": "None",
                "phone": null,
                "totp_enabled": false,
                "username": "admin"
            }
        ])),
        (status = 401, description = "Unauthorized to list all users.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to list all users.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable return list of users.", body = ApiResponse, example = json!({"msg": "Internal error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn list_users(_role: AdminRole, State(appstate): State<AppState>) -> ApiResult {
    let all_users = User::all(&appstate.pool).await?;
    let mut users: Vec<UserInfo> = Vec::with_capacity(all_users.len());
    for user in all_users {
        users.push(UserInfo::from_user(&appstate.pool, &user).await?);
    }
    Ok(ApiResponse::json(users, StatusCode::OK))
}

/// Get user
///
/// Return a user based on provided username parameter.
///
/// # Returns
/// - `UserDetails` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    get,
    path = "/api/v1/user/{username}",
    params(
        ("username" = String, description = "Name of a user"),
    ),
    responses(
        (status = 200, description = "Return details about user.", body = UserDetails, example = json!(
            {
              "biometric_enabled_devices": [],
              "devices": [],
              "security_keys": [],
              "user": {
                "authorized_apps": [],
                "email": "mail@defguard.net",
                "email_mfa_enabled": false,
                "enrolled": true,
                "first_name": "first_name",
                "groups": [],
                "id": 2,
                "is_active": true,
                "is_admin": false,
                "last_name": "last_name",
                "ldap_pass_requires_change": false,
                "mfa_enabled": false,
                "mfa_method": "None",
                "phone": "000000000",
                "totp_enabled": false,
                "username": "username"
              }
            }
        )),
        (status = 401, description = "Unauthorized to return details about user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to return details about user.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to return user details.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn get_user(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    let user_details = UserDetails::from_user(&appstate.pool, &user).await?;
    Ok(ApiResponse::json(user_details, StatusCode::OK))
}

/// Add user
///
/// Add a new user based on `AddUserData` object.
///
/// # Returns
/// - `UserInfo` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    post,
    path = "/api/v1/user",
    request_body = AddUserData,
    responses(
        (status = 201, description = "Add a new user.", body = UserInfo, example = json!(
           {
              "authorized_apps": [],
              "email": "mail@mail",
              "email_mfa_enabled": false,
              "enrolled": true,
              "first_name": "first_name",
              "groups": [],
              "id": 3,
              "is_active": true,
              "is_admin": false,
              "last_name": "last_name",
              "ldap_pass_requires_change": false,
              "mfa_enabled": false,
              "mfa_method": "None",
              "phone": "000000000",
              "totp_enabled": false,
              "username": "new_user"
            }
        )),
        (status = 400, description = "Bad request, invalid user data.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to create a user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to create a user.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to create a user.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn add_user(
    _role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(user_data): Json<AddUserData>,
) -> ApiResult {
    let username = user_data.username.clone();
    debug!("User {} adding user {username}", session.user.username);

    // check if adding new user will go over limits
    let user_count = get_counts().user();

    if get_cached_license()
        .as_ref()
        .and_then(|l| l.limits.as_ref())
        .is_some_and(|l| l.users == user_count)
    {
        error!("Adding user {username} blocked! License limit reached.");
        return Ok(WebError::Forbidden("License limit reached.".into()).into());
    }

    // check username
    if let Err(err) = check_username(&username) {
        debug!("Username {username} rejected: {err}");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    // check if email doesn't already exist
    if User::find_by_email(&appstate.pool, &user_data.email)
        .await?
        .is_some()
    {
        debug!("User with email {} already exists", user_data.email);
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    // check phone number
    if let Some(ref phone) = user_data.phone {
        if !is_valid_phone_number(phone) {
            debug!("Invalid phone number for new user {username}: {phone}");
            return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
        }
    }

    let password = match &user_data.password {
        Some(password) => {
            // check password strength
            if let Err(err) = check_password_strength(password) {
                debug!("Password not strong enough: {err}");
                return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
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
    )
    .save(&appstate.pool)
    .await?;
    update_counts(&appstate.pool).await?;

    if let Some(password) = user_data.password {
        ldap_add_user(&mut user, Some(&password), &appstate.pool).await;
    }

    let user_info = UserInfo::from_user(&appstate.pool, &user).await?;
    appstate.trigger_action(AppEvent::UserCreated(user_info.clone()));
    info!("User {} added user {username}", session.user.username);
    if !user_info.enrolled {
        warn!("User {username} hasn't been enrolled yet. Please proceed with enrollment.");
    }
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserAdded { user }),
    })?;
    Ok(ApiResponse::json(&user_info, StatusCode::CREATED))
}

/// Trigger enrollment process manually
///
/// Allows admin to start new enrollment for user that is provided as a parameter in endpoint.
///
/// Thanks to this endpoint you are able to trigger manually enrollment process, where after finishing you receive an enrollment token.
///
/// **Enrollment token** allows to start the process of gaining access to the company infrastructure **(The enrollment token is valid for 24 hours)**.
///
/// On the other hand, enrollment url allows the user to access the enrollment form via the web browser or perform the enrollment through the desktop client.
///
/// Optionally this endpoint can send an email notification to the user about the enrollment.
///
/// # Returns
/// - JSON with `enrollment_token` and `enrollment_url`
///
/// - `WebError` if error occurs
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/start_enrollment",
    request_body = StartEnrollmentRequest,
    responses(
        (status = 201, description = "Trigger enrollment process manually.", body = ApiResponse, example = json!({"enrollment_token": "your_enrollment_token", "enrollment_url": "your_enrollment_token"})),
        (status = 400, description = "Bad request, invalid enrollment request.", body = ApiResponse, example = json!({"msg": "Email notification is enabled, but email was not provided"})),
        (status = 401, description = "Unauthorized to start enrollment.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to start enrollment.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Provided user does not exist.", body = ApiResponse, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to start enrollment.", body = ApiResponse, example = json!({"msg": "unexpected error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn start_enrollment(
    _role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(data): Json<StartEnrollmentRequest>,
) -> ApiResult {
    debug!(
        "User {} creating enrollment token for user {username}.",
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
    let Some(mut user) = User::find_by_username(&appstate.pool, &username).await? else {
        error!("User {username} couldn't be found, enrollment aborted");
        return Err(WebError::ObjectNotFound(format!(
            "user {username} not found"
        )));
    };

    debug!("Create a new database transaction to save a new enrollment token into the database.");
    let mut transaction = appstate.pool.begin().await?;

    // try to parse token expiration time if provided
    let config = server_config();
    let token_expiration_time_seconds = match data.token_expiration_time {
        Some(time) => parse_duration(&time)
            .map_err(|err| {
                error!("Failed to parse token expiration time {time}: {err}");
                WebError::BadRequest("Failed to parse token expiration time".to_owned())
            })?
            .as_secs(),
        None => config.enrollment_token_timeout.as_secs(),
    };

    let settings: Settings = Settings::get_current_settings();
    let public_proxy_url = settings.proxy_public_url()?;

    let enrollment_token = start_user_enrollment(
        &mut user,
        &mut transaction,
        &session.user,
        data.email,
        token_expiration_time_seconds,
        public_proxy_url.clone(),
        data.send_enrollment_notification,
    )
    .await?;

    debug!("Try to commit transaction to save the enrollment token into the database.");
    transaction.commit().await?;
    debug!("Transaction committed.");

    info!(
        "User {} created enrollment token for user {username}.",
        session.user.username
    );
    debug!(
        "Enrollment token {}, enrollment url {}",
        enrollment_token,
        public_proxy_url.to_string()
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::EnrollmentTokenAdded { user }),
    })?;

    Ok(ApiResponse::new(
        json!({"enrollment_token": enrollment_token, "enrollment_url": public_proxy_url.to_string()}),
        StatusCode::CREATED,
    ))
}

/// Start remote desktop configuration
///
/// Allows admin to start new remote desktop configuration for user that is provided as a parameter in endpoint.
///
/// Thanks to this endpoint you are able to receive a new desktop client configuration or update an existing one. Users need the configuration to connect to the company infrastrcture.
///
/// `Enrollment token` allows to start the process of gaining access to the company infrastructure **(The enrollment token is valid for 24 hours)**.
///
/// On the other hand, enrollment url allows the user to access the enrollment form via the web browser or perform the enrollment through the desktop client.
///
/// Optionally this endpoint can send an email notification to the user about the enrollment.
///
/// # Returns
/// - JSON with `enrollment_token` and `enrollment_url`
///
/// - `WebError` if error occurs
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/start_desktop",
    request_body = StartEnrollmentRequest,
    responses(
        (status = 201, description = "Trigger enrollment process manually.", body = ApiResponse, example = json!({"enrollment_token": "your_enrollment_token", "enrollment_url": "your_enrollment_token"})),
        (status = 400, description = "Bad request, invalid enrollment request.", body = ApiResponse, example = json!({"msg": "Email notification is enabled, but email was not provided"})),
        (status = 401, description = "Unauthorized to start remote desktop configuration.", body = ApiResponse, example = json!({"msg": "Can't create desktop configuration enrollment token for disabled user <username>"})),
        (status = 404, description = "Provided user does not exist.", body = ApiResponse, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to start remote desktop configuration.", body = ApiResponse, example = json!({"msg": "unexpected error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn start_remote_desktop_configuration(
    _can_manage_devices: CanManageDevices,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(data): Json<StartEnrollmentRequest>,
) -> ApiResult {
    debug!(
        "User {} has started a new desktop activation for {username}.",
        session.user.username
    );

    debug!(
        "Verify that the user from the current session is an admin or only peforms desktop activation for self."
    );
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    debug!("Successfully fetched user data: {user:?}");

    // if email is None assume that email should be sent to enrolling user
    let email = match data.email {
        Some(email) => email,
        None => user.email.clone(),
    };

    debug!(
        "Create a new database transaction to save a desktop configuration token into the database."
    );
    let mut transaction = appstate.pool.begin().await?;

    debug!(
        "Generating a new desktop activation token by {}.",
        session.user.username
    );
    let config = server_config();
    let settings = Settings::get_current_settings();
    let public_proxy_url = settings.proxy_public_url()?;
    let desktop_configuration_token = start_desktop_configuration(
        &user,
        &mut transaction,
        &session.user,
        Some(email),
        config.enrollment_token_timeout.as_secs(),
        public_proxy_url.clone(),
        data.send_enrollment_notification,
        None,
    )
    .await?;

    debug!("Try to submit transaction to save the desktop configuration token into the databse.");
    transaction.commit().await?;
    debug!("Transaction submitted.");

    info!(
        "User {} started a new desktop activation.",
        session.user.username
    );
    debug!(
        "Desktop configuration token {}, desktop configuration url {}",
        desktop_configuration_token,
        public_proxy_url.to_string()
    );
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::ClientConfigurationTokenAdded { user }),
    })?;

    Ok(ApiResponse::new(
        json!({"enrollment_token": desktop_configuration_token, "enrollment_url":  public_proxy_url.to_string()}),
        StatusCode::CREATED,
    ))
}

/// Verify if the user is available
///
/// Check if user is available by provided `Username` object.
/// Username is unique so database returns only single user or nothing.
///
/// # Returns
/// - `200` if the user is available
///
/// - `WebError` if error occurs
///
/// **Please take notice that if user exists in database, endpoint will return status code 400.**
#[utoipa::path(
    post,
    path = "/api/v1/user/available",
    request_body = Username,
    responses(
        (status = 200, description = "Provided username is available to use.", body = ApiResponse, example = json!({})),
        (status = 400, description = "Bad request, provided username is not available or username is invalid.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to check is username available.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to check is username available.", body = ApiResponse,  example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to check is username available.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn username_available(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<Username>,
) -> ApiResult {
    if let Err(err) = check_username(&data.username) {
        debug!("Username {} rejected: {err}", data.username);
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }
    let status = match User::find_by_username(&appstate.pool, &data.username).await? {
        Some(_) => {
            debug!("Username {} is not available", data.username);
            StatusCode::BAD_REQUEST
        }
        None => StatusCode::OK,
    };
    Ok(ApiResponse::with_status(status))
}

/// Modify user
///
/// Update user's data basing on `UserInfo` object, it can also remove/add authorized apps and groups assigned to user.
///
/// Endpoint is able to disable a user, but **admin cannot disable himself**.
///
/// Disabling a user can be done by setting `is_active` to `false`.
///
///
/// # Returns
/// - empty JSON
///
/// - `WebError` if error occurs
#[utoipa::path(
    put,
    path = "/api/v1/user/{username}",
    params(
        ("username" = String, description = "Name of a user"),
    ),
    request_body = UserInfo,
    responses(
        (status = 200, description = "User has been updated."),
        (status = 400, description = "Bad request, unable to change user data. Verify user data that you want to update.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to modify user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to modify user.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn modify_user(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(user_info): Json<UserInfo>,
) -> ApiResult {
    debug!("User {} updating user {username}", session.user.username);
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    let groups_before = UserInfo::from_user(&appstate.pool, &user).await?.groups;

    // store user before mods
    let before = user.clone();
    let old_username = user.username.clone();
    if let Err(err) = check_username(&user_info.username) {
        debug!("Username {} rejected: {err}", user_info.username);
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    // check phone number
    if let Some(ref phone) = user_info.phone {
        if !is_valid_phone_number(phone) {
            debug!("Invalid phone number for user {username}: {phone}");
            return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
        }
    }

    let status_changing = user_info.is_active != user.is_active;

    let mut transaction = appstate.pool.begin().await?;
    let ldap_sync_allowed = ldap_sync_allowed_for_user(&user, &mut *transaction).await?;

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
    let mut group_diff = GroupDiff::default();
    if session.is_admin {
        // prevent admin from disabling himself
        if session.user.username == username && !user_info.is_active {
            debug!("Admin {username} attempted to disable himself");
            return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
        }

        // update VPN gateway config if user status or groups have changed
        group_diff = user_info
            .handle_user_groups(&mut transaction, &mut user)
            .await?;
        if group_diff.changed()
            || user_info
                .handle_status_change(&mut transaction, &mut user)
                .await?
        {
            debug!(
                "User {} changed {username} groups or status, syncing allowed network devices.",
                session.user.username
            );
            sync_allowed_user_devices(&user, &mut transaction, &appstate.wireguard_tx).await?;
        }

        // remove API tokens when deactivating a user
        if before.is_active && !user.is_active {
            let api_tokens = ApiToken::find_by_user_id(&mut *transaction, user.id).await?;
            for token in api_tokens {
                token.delete(&mut *transaction).await?;
            }
        }

        user_info.into_user_all_fields(&mut user)?;
    } else {
        user_info.into_user_safe_fields(&mut user)?;
    }
    user.save(&mut *transaction).await?;
    transaction.commit().await?;
    let user_info = UserInfo::from_user(&appstate.pool, &user).await?;

    if ldap_sync_allowed {
        ldap_handle_user_modify(&old_username, &mut user, &appstate.pool).await;
    }

    maybe_update_rdn(&mut user);
    user.save(&appstate.pool).await?;

    Box::pin(ldap_update_user_state(&mut user, &appstate.pool)).await;

    if group_diff.changed() || status_changing {
        if !group_diff.added.is_empty() {
            ldap_add_user_to_groups(
                &user,
                group_diff
                    .added
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<&str>>(),
                &appstate.pool,
            )
            .await;
        }

        if !group_diff.removed.is_empty() {
            ldap_remove_user_from_groups(
                &user,
                group_diff
                    .removed
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<&str>>(),
                &appstate.pool,
            )
            .await;
        }
    }

    appstate.trigger_action(AppEvent::UserModified(user_info.clone()));
    let groups_after = user_info.groups.clone();
    info!("User {} updated user {username}", session.user.username);

    let set_groups_before: HashSet<_> = groups_before.iter().collect();
    let set_groups_after: HashSet<_> = groups_after.iter().collect();

    if set_groups_before != set_groups_after {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::UserGroupsModified {
                user: user.clone(),
                before: groups_before,
                after: groups_after,
            }),
        })?;
    }

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::UserModified {
            before,
            after: user,
        }),
    })?;
    Ok(ApiResponse::default())
}

/// Delete user
///
/// Deletes user, however, **you can't delete yourself as an administrator**.
///
/// # Returns
/// - `WebError` if error occurs
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}",
    params(
        ("username" = String, description = "Name of a user"),
    ),
    responses(
        (status = 200, description = "User has been deleted."),
        (status = 400, description = "Bad request, unable to delete user.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to delete user.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to delete user.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "User does not exist with username: <username>", body = ApiResponse, example = json!({"msg": "User <username> not found"})),
        (status = 500, description = "Unable to delete user.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn delete_user(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    session: SessionInfo,
    context: ApiRequestContext,
) -> ApiResult {
    debug!("User {} deleting user {username}", session.user.username);
    if session.user.username == username {
        debug!("User {username} attempted to delete himself");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }
    if let Some(user) = User::find_by_username(&appstate.pool, &username).await? {
        // Get rid of all devices of the deleted user from networks first
        debug!(
            "User {} deleted user {username}, purging their network devices across all networks.",
            session.user.username
        );
        let mut transaction = appstate.pool.begin().await?;
        let user_for_ldap = if ldap_sync_allowed_for_user(&user, &mut *transaction).await? {
            Some(user.clone().as_noid())
        } else {
            None
        };
        delete_user_and_cleanup_devices(user.clone(), &mut transaction, &appstate.wireguard_tx)
            .await?;

        appstate.trigger_action(AppEvent::UserDeleted(username.clone()));
        transaction.commit().await?;
        update_counts(&appstate.pool).await?;
        if let Some(user_for_ldap) = user_for_ldap {
            ldap_delete_user(&user_for_ldap, &appstate.pool).await;
        }

        info!("User {} deleted user {}", session.user.username, &username);
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::UserRemoved { user }),
        })?;
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
/// Changes your own password basing on `PasswordChangeSelf` object.
///
/// It can return error if password is not strong enough.
///
/// # Returns
/// - `WebError` if error occurs
#[utoipa::path(
    put,
    path = "/api/v1/user/change_password",
    request_body = PasswordChangeSelf,
    responses(
        (status = 200, description = "Pasword has been changed.", body = ApiResponse, example = json!({})),
        (status = 400, description = "Bad request, provided passwords are not same or new password does not satisfy requirements.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to change password.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to change your password", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn change_self_password(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<PasswordChangeSelf>,
) -> ApiResult {
    debug!("User {} is changing his password.", session.user.username);
    let mut user = session.user;
    if user.verify_password(&data.old_password).is_err() {
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    if let Err(err) = check_password_strength(&data.new_password) {
        debug!("User {} password change failed: {err}", user.username);
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    user.set_password(&data.new_password);
    user.save(&appstate.pool).await?;

    ldap_change_password(&mut user, &data.new_password, &appstate.pool).await;

    info!("User {} changed his password.", &user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::PasswordChanged),
    })?;

    Ok(ApiResponse::with_status(StatusCode::OK))
}

/// Change user password
///
/// Change user password basing on `PasswordChange` object, it can return error if password is not strong enough.
///
/// This endpoint doesn't allow you to **change your own** password.
///
/// If you want to change your own password please go to: `/api/v1/user/change_password`.
///
/// # Returns
/// - `WebError` if error occurs
#[utoipa::path(
    put,
    path = "/api/v1/user/{username}/password",
    params(
        ("username" = String, description = "Name of a user"),
    ),
    request_body = PasswordChange,
    responses(
        (status = 200, description = "Password has been changed.", body = ApiResponse, example = json!({})),
        (status = 400, description = "Bad request, password does not satisfy requirements. This endpoint does not change your own password.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to change password.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to change user password.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Cannot change user password that does not exist.", body = ApiResponse, example = json!({})),
        (status = 500, description = "Unable to change user password", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn change_password(
    _role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
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
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    if let Err(err) = check_password_strength(&data.new_password) {
        debug!("Password for user {username} not strong enough: {err}");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }
    if let Err(err) = check_username(&username) {
        debug!("Invalid username ({username}): {err}");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    let user = User::find_by_username(&appstate.pool, &username).await?;

    if let Some(mut user) = user {
        user.set_password(&data.new_password);
        user.save(&appstate.pool).await?;
        ldap_change_password(&mut user, &data.new_password, &appstate.pool).await;
        info!(
            "Admin {} changed password for user {username}",
            session.user.username
        );
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::PasswordChangedByAdmin { user }),
        })?;
        Ok(ApiResponse::default())
    } else {
        debug!("Can't change password for user {username}, user not found");
        Ok(ApiResponse::with_status(StatusCode::NOT_FOUND))
    }
}

/// Reset user password
///
/// Reset user password, it will send a new enrollment token to the user's email.
///
/// **This endpoint doesn't allow you to reset your own password.**
///
/// # Returns
/// - `WebError` if error occurs
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/reset_password",
    params(
        ("username" = String, description = "Name of a user"),
    ),
    responses(
        (status = 200, description = "Successfully reset user password."),
        (status = 400, description = "Bad request, this endpoint does not change your own password.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to change password.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to change user password.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Cannot reset user password that does not exist.", body = ApiResponse, example = json!({})),
        (status = 500, description = "Unable to send reset password to email", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn reset_password(
    _role: AdminRole,
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    debug!(
        "Admin {} resetting password for user {username}",
        session.user.username,
    );

    if session.user.username == username {
        debug!("Cannot reset own ({username}) password with this endpoint.");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    let user = User::find_by_username(&appstate.pool, &username).await?;

    if let Some(user) = user {
        let mut transaction = appstate.pool.begin().await?;

        Token::delete_unused_user_password_reset_tokens(&mut transaction, user.id).await?;

        let config = server_config();
        let enrollment = Token::new(
            user.id,
            Some(session.user.id),
            Some(user.email.clone()),
            config.password_reset_token_timeout.as_secs(),
            Some(PASSWORD_RESET_TOKEN_TYPE.to_string()),
        );
        enrollment.save(&mut *transaction).await?;
        let settings = Settings::get_current_settings();
        let public_proxy_url = settings.proxy_public_url()?;

        let result = Mail::new(
            user.email.clone(),
            EMAIL_PASSWORD_RESET_START_SUBJECT,
            templates::email_password_reset_mail(
                public_proxy_url,
                enrollment.id.clone().as_str(),
                None,
                None,
            )?,
        )
        .send()
        .await;

        let to = &user.email;
        match result {
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
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::PasswordReset { user }),
        })?;
        Ok(ApiResponse::default())
    } else {
        debug!("Can't reset password for user {username}, user not found");
        Ok(ApiResponse::with_status(StatusCode::NOT_FOUND))
    }
}

/// Delete security key
///
/// Delete WebAuthn security key that allows users to authenticate.
///
/// # Returns
/// - `WebError` if error occurs
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/security_key/{id}",
    params(
        ("username" = String, description = "Name of a user"),
        ("id" = i64, description = "ID of security key that could point to passkey")
    ),
    responses(
        (status = 200, description = "Successfully deleted security key."),
        (status = 401, description = "Unauthorized to delete security key.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to delete security key.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Incorrect authorized app, not found.", body = ApiResponse, example = json!({"msg": "security key not found"})),
        (status = 500, description = "Cannot delete authorized app.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn delete_security_key(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path((username, id)): Path<(String, i64)>,
) -> ApiResult {
    debug!(
        "User {} deleting security key {id} for user {username}",
        session.user.username,
    );
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    if let Some(webauthn) = WebAuthn::find_by_id(&appstate.pool, id).await? {
        if webauthn.user_id == user.id {
            webauthn.clone().delete(&appstate.pool).await?;
            user.verify_mfa_state(&appstate.pool).await?;
            info!(
                "User {} deleted security key {id} for user {username}",
                session.user.username,
            );
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::MfaSecurityKeyRemoved { key: webauthn }),
            })?;
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
/// Endpoint returns the data associated with the current session user
///
/// # Returns
/// - `UserInfo` object
///
/// - `WebError` if error occurs
#[utoipa::path(
    get,
    path = "/api/v1/me",
    responses(
        (status = 200, description = "Returns your own data.", body = UserInfo, example = json!(
            {
                  "authorized_apps": [],
                  "email": "mail@mail",
                  "email_mfa_enabled": false,
                  "enrolled": true,
                  "first_name": "first_name",
                  "groups": [
                    "admin"
                  ],
                  "id": 1,
                  "is_active": true,
                  "is_admin": true,
                  "last_name": "last_name",
                  "ldap_pass_requires_change": false,
                  "mfa_enabled": false,
                  "mfa_method": "None",
                  "phone": 000_000_000,
                  "totp_enabled": false,
                  "username": "username"
                }
        )),
        (status = 401, description = "Unauthorized return own user data.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Cannot retrieve own user data.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn me(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    let user_info = UserInfo::from_user(&appstate.pool, &session.user).await?;
    Ok(ApiResponse::json(user_info, StatusCode::OK))
}

/// Delete OAuth token.
///
/// Deletes an authorized application by `OAuth2` ID.
///
/// # Returns
/// - `WebError` if error occurs
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/oauth_app/{oauth2client_id}",
    params(
        ("username" = String, description = "Name of a user"),
        ("oauth2client_id" = i64, description = "id of OAuth2 client")
    ),
    responses(
        (status = 200, description = "Successfully deleted authorized app."),
        (status = 401, description = "Unauthorized to delete authorized app.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to delete authorized app.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Incorrect authorized app, not found.", body = ApiResponse, example = json!({"msg": "Authorized app not found"})),
        (status = 500, description = "Cannot delete authorized app.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
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
        user.id,
        oauth2client_id,
    )
    .await?
    {
        if app.user_id == user.id {
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
    use claims::{assert_err, assert_ok};

    use super::*;

    #[test]
    fn test_username_validation() {
        // valid usernames
        assert_ok!(check_username("zenek34"));
        assert_ok!(check_username("zenekXXX__"));
        assert_ok!(check_username("first.last"));
        assert_ok!(check_username("First_Last"));
        assert_ok!(check_username("32zenek"));
        assert_ok!(check_username("32-zenek"));
        assert_ok!(check_username("a"));
        assert_ok!(check_username("32"));
        assert_ok!(check_username("a4"));

        // invalid usernames
        assert_err!(check_username("__zenek"));
        assert_err!(check_username("zenek?"));
        assert_err!(check_username("MeMeMe!"));
        assert_err!(check_username(
            "averylongnameeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
        ));
    }
}
