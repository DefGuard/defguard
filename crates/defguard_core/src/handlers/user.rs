use std::{collections::HashSet, fmt};

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use axum_extra::extract::Query;
use defguard_common::{
    db::{
        Id,
        models::{BiometricAuth, OAuth2AuthorizedApp, Settings, User, WebAuthn, user::SecurityKey},
    },
    types::{group_diff::GroupDiff, user_info::UserInfo},
};
use humantime::parse_duration;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Postgres, QueryBuilder, Type};
use thiserror::Error;
use utoipa::ToSchema;

use super::{
    AddUserData, ApiErrorResponse, ApiResponse, ApiResult, PasswordChange, PasswordChangeSelf,
    StartEnrollmentRequest, Username, user_for_admin_or_self,
};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{
        AppEvent,
        models::enrollment::{PASSWORD_RESET_TOKEN_TYPE, Token},
    },
    enrollment_management::{
        send_enrollment_invitation, start_desktop_configuration, start_user_enrollment,
    },
    enterprise::{
        db::models::{api_tokens::ApiToken, openid_provider::OpenIdProvider},
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
    handlers::pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
    is_valid_phone_number,
    mail::templates,
    user_management::{delete_user_and_cleanup_devices, disable_user, sync_allowed_user_devices},
};

#[derive(Deserialize, ToSchema)]
pub(crate) struct BulkUserOperationRequest {
    pub users: Vec<Id>,
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct BulkStartEnrollmentRequest {
    pub users: Vec<Id>,
    /// Whether to send enrollment email to each user (uses user's stored email).
    #[serde(default)]
    pub send_enrollment_notification: bool,
    /// Token expiration override in humantime format, for example `24h`. Falls back to the
    /// system setting.
    pub token_expiration_time: Option<String>,
}

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

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ValidationError(pub String);

pub fn check_username(username: &str) -> Result<(), ValidationError> {
    // check length
    let length = username.len();
    if !(1..MAX_USERNAME_CHARS).contains(&length) {
        return Err(ValidationError(format!(
            "Username ({username}) has incorrect length"
        )));
    }

    // check first character is a letter or digit
    if let Some(first_char) = username.chars().next()
        && !first_char.is_ascii_alphanumeric()
    {
        return Err(ValidationError(
            "Username must not start with a special character".into(),
        ));
    }

    // check if username contains only valid characters
    if !username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
    {
        return Err(ValidationError(
            "Username contains invalid characters".into(),
        ));
    }

    Ok(())
}
pub fn check_password_strength(password: &str) -> Result<(), ValidationError> {
    if !(8..=128).contains(&password.len()) {
        return Err(ValidationError("Incorrect password length".into()));
    }
    if !password.chars().any(|c| c.is_ascii_punctuation()) {
        return Err(ValidationError("No special characters in password".into()));
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(ValidationError("No numbers in password".into()));
    }
    if !password.chars().any(|c| c.is_ascii_lowercase()) {
        return Err(ValidationError(
            "No lowercase characters in password".into(),
        ));
    }
    if !password.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(ValidationError(
            "No uppercase characters in password".into(),
        ));
    }
    Ok(())
}

// Full user info with related objects
#[derive(Deserialize, Serialize, ToSchema)]
pub struct UserDetails {
    pub user: UserInfo,
    pub biometric_enabled_devices: Vec<Id>,
    #[serde(default)]
    pub security_keys: Vec<SecurityKey>,
}

impl UserDetails {
    pub(crate) async fn from_user(
        pool: &PgPool,
        user: User<Id>,
        oidc_disable_password_management: bool,
    ) -> sqlx::Result<Self> {
        let security_keys = user.security_keys(pool).await?;
        let biometric_enabled_devices = BiometricAuth::find_by_user_id(pool, user.id)
            .await?
            .iter()
            .map(|a| a.device_id)
            .collect::<Vec<_>>();
        Ok(Self {
            user: UserInfo::from_user(pool, user, oidc_disable_password_management).await?,
            security_keys,
            biometric_enabled_devices,
        })
    }
}

/// List of all users
///
/// Query params for sorting the user list.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct SortParams {
    #[serde(default)]
    pub sort_by: SortKey,
    #[serde(default)]
    pub sort_order: SortOrder,
}

#[derive(Debug, Deserialize, Type, Serialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortKey {
    Username,
    #[default]
    Name,
    Email,
}

impl fmt::Display for SortKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Username => "u.username",
            Self::Name => "u.first_name",
            Self::Email => "u.email",
        })
    }
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Default, Type)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        })
    }
}

/// Query params for filtering the user list.
#[derive(Debug, Deserialize, Default)]
pub struct UserFilterParams {
    /// Filter users by group membership (OR logic - user in any listed group).
    #[serde(default)]
    pub groups: Vec<String>,
    /// Filter users with no group memberships. When combined with `groups`, returns the union
    /// (users in the specified groups OR users with no groups).
    #[serde(default)]
    pub no_group: bool,
    /// Free-text search across username, first_name, last_name, and email.
    pub search: Option<String>,
}

/// List users
#[utoipa::path(
    get,
    path = "/api/v1/user",
    tag = "user",
    params(
        ("page" = Option<u32>, Query, description = "Page number. Defaults to 1."),
        ("per_page" = Option<u32>, Query, description = "Number of items per page, from 1 to 100. Defaults to 50."),
        ("groups" = Option<Vec<String>>, Query, description = "Filter by group names. Returns users belonging to any of the given groups."),
        ("no_group" = Option<bool>, Query, description = "Filter users with no group membership. Combined with `groups`, returns users in the given groups and users with no group."),
        ("search" = Option<String>, Query, description = "Free-text search across username, first name, last name, and email."),
        ("sort_by" = Option<SortKey>, Query, description = "Sort key: `name`, `username`, or `email`. Defaults to `name`."),
        ("sort_order" = Option<SortOrder>, Query, description = "Sort direction: `asc` or `desc`. Defaults to `asc`."),
    ),
    responses(
        (status = 200, description = "Paginated list of users.", body = PaginatedApiResponse<UserInfo>, example = json!(
        {
            "data": [
                {
                    "authorized_apps": [],
                    "devices": [],
                    "email": "jane@example.com",
                    "email_mfa_enabled": false,
                    "enrolled": true,
                    "first_name": "Jane",
                    "groups": [
                      "admin"
                    ],
                    "has_non_mfa_location_access": false,
                    "has_non_posture_location_access": false,
                    "id": 1,
                    "is_active": true,
                    "is_admin": true,
                    "last_name": "Doe",
                    "ldap_pass_requires_change": false,
                    "mfa_enabled": false,
                    "mfa_method": "None",
                    "name": "Jane Doe",
                    "password_management_disabled": false,
                    "phone": null,
                    "totp_enabled": false,
                    "username": "jane"
                }
            ],
            "pagination": {
                "current_page": 1,
                "page_size": 50,
                "total_items": 1,
                "total_pages": 1,
                "next_page": null
            }
        })),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to list users.", body = ApiErrorResponse, example = json!({"msg": "Internal error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_users(
    _role: AdminRole,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
    filters: Query<UserFilterParams>,
    sorting: Query<SortParams>,
) -> PaginatedApiResult<UserInfo> {
    let pagination = pagination.0;
    let filters = filters.0;
    let sorting = sorting.0;

    debug!("Listing users with filters: {filters:?} and sorting {sorting:?}");

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT u.id, u.username, u.password_hash, u.last_name, u.first_name, u.email, \
        u.phone, u.mfa_enabled, u.totp_enabled, u.email_mfa_enabled, \
        u.totp_secret, u.email_mfa_secret, u.mfa_method, u.recovery_codes, \
        u.is_active, u.openid_sub, \
        u.from_ldap, u.ldap_pass_randomized, u.ldap_rdn, u.ldap_user_path, \
        u.ldap_remote_enrollment_completed, u.enrollment_pending \
        FROM \"user\" u WHERE 1=1 ",
    );

    apply_filters(&mut query_builder, &filters);
    apply_sorting(&mut query_builder, &sorting);

    query_builder
        .push(" LIMIT ")
        .push_bind(i64::from(pagination.per_page()));
    query_builder
        .push(" OFFSET ")
        .push_bind(i64::from(pagination.offset()));

    let all_users = query_builder
        .build_query_as::<User<Id>>()
        .fetch_all(&appstate.pool)
        .await?;

    let mut count_query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*) FROM \"user\" u WHERE 1=1 ");
    apply_filters(&mut count_query_builder, &filters);
    let count: i64 = count_query_builder
        .build_query_scalar()
        .fetch_one(&appstate.pool)
        .await?;

    // Map [`User`] to [`UserInfo`].
    // TODO: too many queries – optimise.
    let oidc_disable_password_management =
        OpenIdProvider::current_disables_password_management(&appstate.pool).await?;
    let mut users = Vec::with_capacity(all_users.len());
    for user in all_users {
        users.push(
            UserInfo::from_user(&appstate.pool, user, oidc_disable_password_management).await?,
        );
    }

    info!("Listed users");

    Ok(PaginatedApiResponse::new(users, pagination, count as u32))
}

/// Adds optional filtering statements to SQL query based on request query params.
fn apply_filters(query_builder: &mut QueryBuilder<Postgres>, filters: &UserFilterParams) {
    debug!("Applying query filters: {filters:?}");

    let has_no_group = filters.no_group;
    let has_groups = !filters.groups.is_empty();

    if has_no_group && has_groups {
        query_builder.push(
            " AND (NOT EXISTS (SELECT 1 FROM group_user \
            WHERE group_user.user_id = u.id) \
            OR EXISTS (SELECT 1 FROM group_user gu \
            INNER JOIN \"group\" g ON gu.group_id = g.id \
            WHERE gu.user_id = u.id AND g.name = ANY(",
        );
        query_builder.push_bind(filters.groups.clone());
        query_builder.push("))) ");
    } else if has_no_group {
        query_builder.push(
            " AND NOT EXISTS (SELECT 1 FROM group_user \
            WHERE group_user.user_id = u.id) ",
        );
    } else if has_groups {
        query_builder.push(
            " AND EXISTS (SELECT 1 FROM group_user gu \
            INNER JOIN \"group\" g ON gu.group_id = g.id \
            WHERE gu.user_id = u.id AND g.name = ANY(",
        );
        query_builder.push_bind(filters.groups.clone());
        query_builder.push(")) ");
    }

    if let Some(search_term) = &filters.search {
        query_builder
            .push(
                " AND CONCAT(u.username, ' ', u.first_name, ' ', u.last_name, ' ', u.email) ILIKE ",
            )
            .push_bind(format!("%{search_term}%"))
            .push(" ");
    }
}

/// Adds ORDER BY clause to SQL query based on request query params.
fn apply_sorting(query_builder: &mut QueryBuilder<Postgres>, sorting: &SortParams) {
    debug!("Applying query sorting: {sorting:?}");

    query_builder
        .push(" ORDER BY ")
        .push(sorting.sort_by.to_string())
        .push(" ")
        .push(sorting.sort_order.to_string());

    if matches!(sorting.sort_by, SortKey::Name) {
        query_builder
            .push(", u.last_name ")
            .push(sorting.sort_order.to_string());
    }

    query_builder
        .push(", u.id ")
        .push(sorting.sort_order.to_string());
}

/// Get a user
#[utoipa::path(
    get,
    path = "/api/v1/user/{username}",
    tag = "user",
    params(
        ("username" = String, description = "Name of the user."),
    ),
    responses(
        (status = 200, description = "User details.", body = UserDetails, example = json!(
            {
              "biometric_enabled_devices": [],
              "security_keys": [],
              "user": {
                "authorized_apps": [],
                "devices": [],
                "email": "jdoe@example.com",
                "email_mfa_enabled": false,
                "enrolled": true,
                "first_name": "John",
                "groups": [],
                "has_non_mfa_location_access": false,
                "has_non_posture_location_access": false,
                "id": 2,
                "is_active": true,
                "is_admin": false,
                "last_name": "Doe",
                "ldap_pass_requires_change": false,
                "mfa_enabled": false,
                "mfa_method": "None",
                "name": "John Doe",
                "password_management_disabled": false,
                "phone": "+48123456789",
                "totp_enabled": false,
                "username": "jdoe"
              }
            }
        )),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to get user.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn get_user(
    session: SessionInfo,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
) -> ApiResult {
    let user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    let oidc_disable_password_management =
        OpenIdProvider::current_disables_password_management(&appstate.pool).await?;
    let user_details =
        UserDetails::from_user(&appstate.pool, user, oidc_disable_password_management).await?;
    Ok(ApiResponse::json(user_details, StatusCode::OK))
}

/// Create a user
#[utoipa::path(
    post,
    path = "/api/v1/user",
    tag = "user",
    request_body(content = AddUserData, description = "Leave `password` out to enroll the user instead.", example = json!({"username": "jdoe", "first_name": "John", "last_name": "Doe", "email": "jdoe@example.com", "phone": "+48123456789"})),
    responses(
        (status = 201, description = "User created.", body = UserInfo, example = json!(
           {
              "authorized_apps": [],
              "devices": [],
              "email": "jdoe@example.com",
              "email_mfa_enabled": false,
              "enrolled": true,
              "first_name": "John",
              "groups": [],
              "has_non_mfa_location_access": false,
              "has_non_posture_location_access": false,
              "id": 3,
              "is_active": true,
              "is_admin": false,
              "last_name": "Doe",
              "ldap_pass_requires_change": false,
              "mfa_enabled": false,
              "mfa_method": "None",
              "name": "John Doe",
              "password_management_disabled": false,
              "phone": "+48123456789",
              "totp_enabled": false,
              "username": "jdoe"
            }
        )),
        (status = 400, description = "Invalid user data."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to create user.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn add_user(
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
        .is_some_and(|l| user_count >= l.users)
    {
        error!("Adding user {username} blocked! License limit reached.");
        return Ok(WebError::Forbidden("License limit reached").into());
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
    if let Some(ref phone) = user_data.phone
        && !is_valid_phone_number(phone)
    {
        debug!("Invalid phone number for new user {username}: {phone}");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
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
        ldap_add_user(
            &mut user,
            Some(&password),
            &appstate.pool,
            &appstate.ldap_tx,
        )
        .await;
    }

    let oidc_disable_password_management =
        OpenIdProvider::current_disables_password_management(&appstate.pool).await?;
    let user_info = UserInfo::from_user(
        &appstate.pool,
        user.clone(),
        oidc_disable_password_management,
    )
    .await?;
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

/// Start enrollment for a user
///
/// Returns an enrollment token, valid for 24 hours, and the URL the user opens to finish
/// enrollment in a browser or in the desktop client. The user can also be notified by email.
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/start_enrollment",
    tag = "user",
    params(
        ("username" = String, Path, description = "Name of the user."),
    ),
    request_body = StartEnrollmentRequest,
    responses(
        (status = 201, description = "Enrollment token and URL.", body = Object, example = json!({"enrollment_token": "5nT2xK9wQpR7vL1yZbH3cD8fG5aQeJmU", "enrollment_url": "https://vpn.example.com/enrollment"})),
        (status = 400, description = "Invalid enrollment request.", body = ApiErrorResponse, example = json!({"msg": "Email notification is enabled, but email was not provided"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to start enrollment.", body = ApiErrorResponse, example = json!({"msg": "unexpected error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn start_enrollment(
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
    let settings = Settings::get_current_settings();
    let token_expiration_time_seconds = match data.token_expiration_time {
        Some(time) => parse_duration(&time)
            .map_err(|err| {
                error!("Failed to parse token expiration time {time}: {err}");
                WebError::BadRequest("Failed to parse token expiration time".to_owned())
            })?
            .as_secs(),
        None => settings.enrollment_token_timeout().as_secs(),
    };

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
    debug!("Enrollment url {}", public_proxy_url.to_string());
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
/// Creates or updates the desktop client configuration of the user. Returns an enrollment
/// token, valid for 24 hours, and the URL the user opens to finish the setup. The user can
/// also be notified by email.
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/start_desktop",
    tag = "user",
    params(
        ("username" = String, Path, description = "Name of the user."),
    ),
    request_body = StartEnrollmentRequest,
    responses(
        (status = 201, description = "Enrollment token and URL.", body = Object, example = json!({"enrollment_token": "5nT2xK9wQpR7vL1yZbH3cD8fG5aQeJmU", "enrollment_url": "https://vpn.example.com/enrollment"})),
        (status = 400, description = "Invalid enrollment request.", body = ApiErrorResponse, example = json!({"msg": "Email notification is enabled, but email was not provided"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Can't create desktop configuration enrollment token for disabled user <username>"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to start remote desktop configuration.", body = ApiErrorResponse, example = json!({"msg": "unexpected error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn start_remote_desktop_configuration(
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
    let settings = Settings::get_current_settings();
    let public_proxy_url = settings.proxy_public_url()?;
    let desktop_configuration_token = start_desktop_configuration(
        &user,
        &mut transaction,
        &session.user,
        Some(email),
        settings.enrollment_token_timeout().as_secs(),
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
    debug!("Desktop configuration url {}", public_proxy_url.to_string());
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::ClientConfigurationTokenAdded { user }),
    })?;

    Ok(ApiResponse::new(
        json!({"enrollment_token": desktop_configuration_token, "enrollment_url":  public_proxy_url.to_string()}),
        StatusCode::CREATED,
    ))
}

/// Check whether a username is available
#[utoipa::path(
    post,
    path = "/api/v1/user/available",
    tag = "user",
    request_body = Username,
    responses(
        (status = 200, description = "Username is available."),
        (status = 400, description = "Username is invalid or already taken."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse,  example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to check username.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn username_available(
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

/// Update a user
///
/// Can also add or remove the user's groups and authorized apps. Set `is_active` to
/// `false` to disable the user. An admin cannot disable their own account.
#[utoipa::path(
    put,
    path = "/api/v1/user/{username}",
    tag = "user",
    params(
        ("username" = String, description = "Name of the user."),
    ),
    request_body = UserInfo,
    responses(
        (status = 200, description = "User updated."),
        (status = 400, description = "Invalid user data."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "user <username> not found"})),
        (status = 500, description = "Unable to update user.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn modify_user(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Path(username): Path<String>,
    Json(user_info): Json<UserInfo>,
) -> ApiResult {
    debug!("User {} updating user {username}", session.user.username);
    let mut user = user_for_admin_or_self(&appstate.pool, &session, &username).await?;
    let oidc_disable_password_management =
        OpenIdProvider::current_disables_password_management(&appstate.pool).await?;
    let groups_before = UserInfo::from_user(
        &appstate.pool,
        user.clone(),
        oidc_disable_password_management,
    )
    .await?
    .groups;

    // store user before mods
    let before = user.clone();
    let old_username = user.username.clone();
    if let Err(err) = check_username(&user_info.username) {
        debug!("Username {} rejected: {err}", user_info.username);
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    // check phone number
    if let Some(ref phone) = user_info.phone
        && !is_valid_phone_number(phone)
    {
        debug!("Invalid phone number for user {username}: {phone}");
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    let status_changing = user_info.is_active != user.is_active;

    let mut transaction = appstate.pool.begin().await?;
    let ldap_sync_allowed = ldap_sync_allowed_for_user(&user, &mut *transaction).await?;

    // remove authorized apps if needed
    let request_app_ids: Vec<Id> = user_info
        .authorized_apps
        .iter()
        .map(|app| app.oauth2client_id)
        .collect();
    let db_apps = user.oauth2authorizedapps(&mut *transaction).await?;
    let removed_apps: Vec<Id> = db_apps
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

        // check if re-enabling a disabled user will go over license limits
        if !user.is_active && user_info.is_active {
            let user_count = get_counts().user();
            let user_limit = get_cached_license()
                .as_ref()
                .and_then(|l| l.limits.as_ref())
                .map(|l| l.users);

            if let Some(limit) = user_limit
                && user_count >= limit
            {
                error!("Enabling user {username} blocked. License limit reached.");
                return Ok(WebError::LicenseLimitReached(format!(
                    "Cannot enable user {username}: license user limit reached ({user_count}/{limit})"
                ))
                .into());
            }
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
            sync_allowed_user_devices(&user, &mut transaction, &appstate.gateway_tx).await?;
        }

        // remove API tokens when deactivating a user
        if before.is_active && !user.is_active {
            let api_tokens = ApiToken::find_by_user_id(&mut *transaction, user.id).await?;
            for token in api_tokens {
                token.delete(&mut *transaction).await?;
            }
        }
    }

    let updating_self = session.user.username == user.username;
    user_info.handle_update_user_fields(&mut user, session.is_admin, updating_self);

    user.save(&mut *transaction).await?;
    transaction.commit().await?;
    if status_changing {
        update_counts(&appstate.pool).await?;
    }
    let user_info = UserInfo::from_user(
        &appstate.pool,
        user.clone(),
        oidc_disable_password_management,
    )
    .await?;

    if ldap_sync_allowed {
        ldap_handle_user_modify(
            &old_username,
            &mut user,
            &appstate.pool,
            &appstate.gateway_tx,
            &appstate.ldap_tx,
        )
        .await;
    }

    maybe_update_rdn(&mut user);
    user.save(&appstate.pool).await?;

    Box::pin(ldap_update_user_state(
        &mut user,
        &appstate.pool,
        &appstate.gateway_tx,
        &appstate.ldap_tx,
    ))
    .await;

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
                &appstate.ldap_tx,
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
                &appstate.ldap_tx,
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
        context: context.clone(),
        event: Box::new(ApiEventType::UserModified {
            before,
            after: user.clone(),
        }),
    })?;

    if status_changing {
        let event = if user.is_active {
            ApiEventType::UserEnabled { user }
        } else {
            ApiEventType::UserDisabled { user }
        };
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(event),
        })?;
    }

    Ok(ApiResponse::default())
}

/// Delete a user
///
/// You cannot delete your own account.
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}",
    tag = "user",
    params(
        ("username" = String, description = "Name of the user."),
    ),
    responses(
        (status = 200, description = "User deleted."),
        (status = 400, description = "You cannot delete your own account."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "User <username> not found"})),
        (status = 500, description = "Unable to delete user.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_user(
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
        delete_user_and_cleanup_devices(user.clone(), &mut transaction, &appstate.gateway_tx)
            .await?;

        appstate.trigger_action(AppEvent::UserDeleted(username.clone()));
        transaction.commit().await?;
        update_counts(&appstate.pool).await?;
        if let Some(user_for_ldap) = user_for_ldap {
            ldap_delete_user(&user_for_ldap, &appstate.pool, &appstate.ldap_tx).await;
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

/// Loads the current settings and configured OIDC provider to determine whether password
/// management (set/change/reset) is disabled for `user`.
async fn user_password_management_disabled(pool: &PgPool, user: &User<Id>) -> sqlx::Result<bool> {
    let settings = Settings::get_current_settings();
    let oidc_disabled = OpenIdProvider::current_disables_password_management(pool).await?;
    let is_admin = user.is_admin(pool).await?;
    Ok(user.password_management_disabled(is_admin, &settings, oidc_disabled))
}

/// Change your own password
///
/// Fails when the new password is not strong enough.
#[utoipa::path(
    put,
    path = "/api/v1/user/change_password",
    tag = "user",
    request_body = PasswordChangeSelf,
    responses(
        (status = 200, description = "Password changed."),
        (status = 400, description = "Passwords do not match, or the new password does not satisfy the requirements."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Password management is disabled for this user.", body = ApiErrorResponse, example = json!({"msg": "Password management is disabled for this user"})),
        (status = 500, description = "Unable to change your password.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn change_self_password(
    session: SessionInfo,
    context: ApiRequestContext,
    State(appstate): State<AppState>,
    Json(data): Json<PasswordChangeSelf>,
) -> ApiResult {
    debug!("User {} is changing his password.", session.user.username);
    let mut user = session.user;

    if user_password_management_disabled(&appstate.pool, &user).await? {
        debug!("Password management disabled for user {}", user.username);
        return Ok(ApiResponse::new(
            json!({"msg": "Password management is disabled for this user"}),
            StatusCode::FORBIDDEN,
        ));
    }

    if user.verify_password(&data.old_password).is_err() {
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    if let Err(err) = check_password_strength(&data.new_password) {
        debug!("User {} password change failed: {err}", user.username);
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    user.set_password(&data.new_password);
    user.save(&appstate.pool).await?;

    let session_id = session.session.id;

    user.logout_all_sessions_except(&appstate.pool, &session_id)
        .await?;

    ldap_change_password(
        &mut user,
        &data.new_password,
        &appstate.pool,
        &appstate.ldap_tx,
    )
    .await;

    info!("User {} changed his password.", &user.username);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::PasswordChanged),
    })?;

    Ok(ApiResponse::with_status(StatusCode::OK))
}

/// Change the password of a user
///
/// Fails when the new password is not strong enough. Cannot be used to change your own
/// password, use `PUT /api/v1/user/change_password` for that.
#[utoipa::path(
    put,
    path = "/api/v1/user/{username}/password",
    tag = "user",
    params(
        ("username" = String, description = "Name of the user."),
    ),
    request_body = PasswordChange,
    responses(
        (status = 200, description = "Password changed."),
        (status = 400, description = "Password does not satisfy the requirements, or the request targets your own account."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "User not found."),
        (status = 500, description = "Unable to change user password.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn change_password(
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
        if user_password_management_disabled(&appstate.pool, &user).await? {
            debug!("Password management disabled for user {username}");
            return Ok(ApiResponse::new(
                json!({"msg": "Password management is disabled for this user"}),
                StatusCode::FORBIDDEN,
            ));
        }

        user.set_password(&data.new_password);
        user.save(&appstate.pool).await?;
        user.logout_all_sessions(&appstate.pool).await?;
        ldap_change_password(
            &mut user,
            &data.new_password,
            &appstate.pool,
            &appstate.ldap_tx,
        )
        .await;
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

/// Send a password reset email to a user
///
/// Sends a new enrollment token to the user's email. You cannot reset your own password
/// this way.
#[utoipa::path(
    post,
    path = "/api/v1/user/{username}/reset_password",
    tag = "user",
    params(
        ("username" = String, description = "Name of the user."),
    ),
    responses(
        (status = 200, description = "Password reset email sent."),
        (status = 400, description = "This endpoint does not reset your own password."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "User not found."),
        (status = 500, description = "Unable to send password reset email.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn reset_password(
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
        if user_password_management_disabled(&appstate.pool, &user).await? {
            debug!("Password management disabled for user {username}");
            return Ok(ApiResponse::new(
                json!({"msg": "Password management is disabled for this user"}),
                StatusCode::FORBIDDEN,
            ));
        }

        let mut transaction = appstate.pool.begin().await?;

        Token::delete_unused_user_password_reset_tokens(&mut transaction, user.id).await?;

        let settings = Settings::get_current_settings();
        let enrollment = Token::new(
            user.id,
            Some(session.user.id),
            Some(user.email.clone()),
            settings.password_reset_token_timeout().as_secs(),
            Some(PASSWORD_RESET_TOKEN_TYPE.to_owned()),
        );
        enrollment.save(&mut *transaction).await?;
        let public_proxy_url = settings.proxy_public_url()?;

        templates::password_reset_mail(
            &user.email,
            &mut transaction,
            public_proxy_url,
            enrollment.id.clone().as_str(),
            None,
            None,
        )
        .await?;

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

/// Delete a security key of a user
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/security_key/{id}",
    tag = "user",
    params(
        ("username" = String, description = "Name of the user."),
        ("id" = i64, description = "ID of the security key.")
    ),
    responses(
        (status = 200, description = "Security key deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Security key not found.", body = ApiErrorResponse, example = json!({"msg": "wrong security key"})),
        (status = 500, description = "Unable to delete security key.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_security_key(
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

/// Get the currently authenticated user
#[utoipa::path(
    get,
    path = "/api/v1/me",
    tag = "user",
    responses(
        (status = 200, description = "Your own account details.", body = UserInfo, example = json!(
            {
                  "authorized_apps": [],
                  "devices": [],
                  "email": "jane@example.com",
                  "email_mfa_enabled": false,
                  "enrolled": true,
                  "first_name": "Jane",
                  "groups": [
                    "admin"
                  ],
                  "has_non_mfa_location_access": false,
                  "has_non_posture_location_access": false,
                  "id": 1,
                  "is_active": true,
                  "is_admin": true,
                  "last_name": "Doe",
                  "ldap_pass_requires_change": false,
                  "mfa_enabled": false,
                  "mfa_method": "None",
                  "name": "Jane Doe",
                  "password_management_disabled": false,
                  "phone": "+48123456789",
                  "totp_enabled": false,
                  "username": "jane"
                }
        )),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Unable to get your own account details.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn me(session: SessionInfo, State(appstate): State<AppState>) -> ApiResult {
    let oidc_disable_password_management =
        OpenIdProvider::current_disables_password_management(&appstate.pool).await?;
    let user_info = UserInfo::from_user(
        &appstate.pool,
        session.user,
        oidc_disable_password_management,
    )
    .await?;
    Ok(ApiResponse::json(user_info, StatusCode::OK))
}

/// Delete an authorized OAuth2 application of a user
#[utoipa::path(
    delete,
    path = "/api/v1/user/{username}/oauth_app/{oauth2client_id}",
    tag = "user",
    params(
        ("username" = String, description = "Name of the user."),
        ("oauth2client_id" = i64, description = "ID of the OAuth2 client.")
    ),
    responses(
        (status = 200, description = "Authorized OAuth2 application deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges or the request must target your own account.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Authorized OAuth2 application not found.", body = ApiErrorResponse, example = json!({"msg": "Authorized app not found"})),
        (status = 500, description = "Unable to delete authorized OAuth2 application.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_authorized_app(
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

/// Bulk disable users
///
/// The request is rejected when any of the given IDs does not exist or is your own.
#[utoipa::path(
    post,
    path = "/api/v1/user/bulk-disable",
    tag = "user",
    request_body(content = BulkUserOperationRequest, example = json!({"users": [1, 4, 6, 23, 35]})),
    responses(
        (status = 200, description = "Users disabled."),
        (status = 400, description = "The list contains unknown user IDs or your own account.", body = ApiErrorResponse, example = json!({"msg": "Request contained users that don't exist in db."})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to disable users.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn bulk_disable_users(
    _role: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(mut data): Json<BulkUserOperationRequest>,
) -> ApiResult {
    debug!(
        "User {} bulk-disabling {} user(s)",
        session.user.username,
        data.users.len()
    );

    if data.users.contains(&session.user.id) {
        debug!(
            "User {} attempted to disable themselves via bulk-disable",
            session.user.username
        );
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    data.users.sort_unstable();
    data.users.dedup();

    let users = User::find_by_ids(&appstate.pool, &data.users).await?;

    if users.len() != data.users.len() {
        return Err(WebError::BadRequest(
            "Request contained users that don't exist in db.".into(),
        ));
    }

    let mut events = Vec::with_capacity(users.len());
    let mut transaction = appstate.pool.begin().await?;
    for user in users {
        if !user.is_active {
            continue;
        }
        let before = user.clone();
        let mut user_to_disable = user;

        // remove API tokens when deactivating a user (mirrors modify_user)
        let api_tokens = ApiToken::find_by_user_id(&mut *transaction, user_to_disable.id).await?;
        for token in api_tokens {
            token.delete(&mut *transaction).await?;
        }

        disable_user(&mut user_to_disable, &mut transaction, &appstate.gateway_tx).await?;
        events.push((before, user_to_disable));
    }
    transaction.commit().await?;

    for (_, user) in &mut events {
        Box::pin(ldap_update_user_state(
            user,
            &appstate.pool,
            &appstate.gateway_tx,
            &appstate.ldap_tx,
        ))
        .await;
    }

    info!(
        "User {} bulk-disabled {} user(s)",
        session.user.username,
        events.len()
    );
    for (before, after) in events {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::UserModified { before, after }),
        })?;
    }

    Ok(ApiResponse::default())
}

/// Bulk enable users
///
/// The request is rejected when any of the given IDs does not exist.
#[utoipa::path(
    post,
    path = "/api/v1/user/bulk-enable",
    tag = "user",
    request_body(content = BulkUserOperationRequest, example = json!({"users": [1, 4, 6, 23, 35]})),
    responses(
        (status = 200, description = "Users enabled."),
        (status = 400, description = "The list contains unknown user IDs.", body = ApiErrorResponse, example = json!({"msg": "Request contained users that don't exist in db."})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to enable users.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn bulk_enable_users(
    _role: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(mut data): Json<BulkUserOperationRequest>,
) -> ApiResult {
    debug!(
        "User {} bulk-enabling {} user(s)",
        session.user.username,
        data.users.len()
    );

    data.users.sort_unstable();
    data.users.dedup();

    let users = User::find_by_ids(&appstate.pool, &data.users).await?;

    if users.len() != data.users.len() {
        return Err(WebError::BadRequest(
            "Request contained users that don't exist in db.".into(),
        ));
    }

    // check if enabling the requested users will go over license limits
    let to_enable_count = users.iter().filter(|user| !user.is_active).count() as u32;
    if to_enable_count > 0 {
        let user_count = get_counts().user();
        let user_limit = get_cached_license()
            .as_ref()
            .and_then(|l| l.limits.as_ref())
            .map(|l| l.users);

        if let Some(limit) = user_limit
            && user_count + to_enable_count > limit
        {
            error!(
                "User {} bulk-enabling users blocked! License limit reached.",
                session.user.username
            );
            return Ok(WebError::LicenseLimitReached(format!(
                "Cannot enable {to_enable_count} user(s): license user limit reached \
                ({user_count}/{limit})"
            ))
            .into());
        }
    }

    let mut events = Vec::with_capacity(users.len());
    let mut transaction = appstate.pool.begin().await?;
    for user in users {
        if user.is_active {
            continue;
        }
        let before = user.clone();
        let mut user_to_enable = user;
        user_to_enable.is_active = true;
        user_to_enable.save(&mut *transaction).await?;
        sync_allowed_user_devices(&user_to_enable, &mut transaction, &appstate.gateway_tx).await?;
        events.push((before, user_to_enable));
    }
    transaction.commit().await?;
    if to_enable_count > 0 {
        update_counts(&appstate.pool).await?;
    }

    for (_, user) in &mut events {
        Box::pin(ldap_update_user_state(
            user,
            &appstate.pool,
            &appstate.gateway_tx,
            &appstate.ldap_tx,
        ))
        .await;
    }

    info!(
        "User {} bulk-enabled {} user(s)",
        session.user.username,
        events.len()
    );
    for (before, after) in events {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::UserModified { before, after }),
        })?;
    }

    Ok(ApiResponse::default())
}

/// Bulk delete users
///
/// The request is rejected when any of the given IDs does not exist or is your own.
#[utoipa::path(
    post,
    path = "/api/v1/user/bulk-delete",
    tag = "user",
    request_body(content = BulkUserOperationRequest, example = json!({"users": [1, 4, 6, 23, 35]})),
    responses(
        (status = 200, description = "Users deleted."),
        (status = 400, description = "The list contains unknown user IDs or your own account.", body = ApiErrorResponse, example = json!({"msg": "Request contained users that don't exist in db."})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to delete users.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn bulk_delete_users(
    _role: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(mut data): Json<BulkUserOperationRequest>,
) -> ApiResult {
    debug!(
        "User {} bulk-deleting {} user(s)",
        session.user.username,
        data.users.len()
    );

    if data.users.contains(&session.user.id) {
        debug!(
            "User {} attempted to delete themselves via bulk-delete",
            session.user.username
        );
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    data.users.sort_unstable();
    data.users.dedup();

    let users = User::find_by_ids(&appstate.pool, &data.users).await?;

    if users.len() != data.users.len() {
        return Err(WebError::BadRequest(
            "Request contained users that don't exist in db.".into(),
        ));
    }

    let mut transaction = appstate.pool.begin().await?;
    let mut ldap_targets = Vec::new();
    let mut removed_usernames = Vec::new();
    let mut removed_users = Vec::new();
    for user in users {
        let username = user.username.clone();
        let user_for_ldap = if ldap_sync_allowed_for_user(&user, &mut *transaction).await? {
            Some(user.clone().as_noid())
        } else {
            None
        };
        delete_user_and_cleanup_devices(user.clone(), &mut transaction, &appstate.gateway_tx)
            .await?;
        if let Some(noid_user) = user_for_ldap {
            ldap_targets.push(noid_user);
        }
        removed_usernames.push(username);
        removed_users.push(user);
    }
    transaction.commit().await?;
    update_counts(&appstate.pool).await?;

    for username in &removed_usernames {
        appstate.trigger_action(AppEvent::UserDeleted(username.clone()));
    }
    for noid_user in &ldap_targets {
        ldap_delete_user(noid_user, &appstate.pool, &appstate.ldap_tx).await;
    }

    info!(
        "User {} bulk-deleted {} user(s)",
        session.user.username,
        removed_users.len()
    );
    for user in removed_users {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::UserRemoved { user }),
        })?;
    }

    Ok(ApiResponse::default())
}

/// Bulk start user enrollment
///
/// Disabled users are skipped and counted in the `skipped` response field. Already
/// enrolled users are enrolled again. The request is rejected when any of the given IDs
/// does not exist or is your own.
#[utoipa::path(
    post,
    path = "/api/v1/user/bulk-start-enrollment",
    tag = "user",
    request_body(content = BulkStartEnrollmentRequest, example = json!({"users": [1, 4, 6, 23, 35], "send_enrollment_notification": true, "token_expiration_time": "24h"})),
    responses(
        (status = 200, description = "Enrollment started.", body = Object, example = json!({"started": 3, "skipped": 1})),
        (status = 400, description = "The list contains unknown user IDs or your own account.", body = ApiErrorResponse, example = json!({"msg": "Request contained users that don't exist in db."})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to start enrollments.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn bulk_start_enrollment(
    _role: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(mut data): Json<BulkStartEnrollmentRequest>,
) -> ApiResult {
    debug!(
        "User {} bulk-starting enrollment for {} user(s)",
        session.user.username,
        data.users.len()
    );

    if data.users.contains(&session.user.id) {
        debug!(
            "User {} attempted to start their own enrollment via bulk-start-enrollment",
            session.user.username
        );
        return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
    }

    data.users.sort_unstable();
    data.users.dedup();

    let users = User::find_by_ids(&appstate.pool, &data.users).await?;

    if users.len() != data.users.len() {
        return Err(WebError::BadRequest(
            "Request contained users that don't exist in db.".into(),
        ));
    }

    let settings = Settings::get_current_settings();
    let token_expiration_time_seconds = match data.token_expiration_time {
        Some(time) => parse_duration(&time)
            .map_err(|err| {
                error!("Failed to parse token expiration time {time}: {err}");
                WebError::BadRequest("Failed to parse token expiration time".to_owned())
            })?
            .as_secs(),
        None => settings.enrollment_token_timeout().as_secs(),
    };
    let public_proxy_url = settings.proxy_public_url()?;

    let mut started = Vec::with_capacity(users.len());
    let mut pending_notifications: Vec<(String, String)> = Vec::new();
    let mut skipped = 0;
    let mut transaction = appstate.pool.begin().await?;
    for mut user in users {
        if !user.is_active {
            debug!("Skipping bulk enrollment for {}: disabled", user.username);
            skipped += 1;
            continue;
        }
        let email = if data.send_enrollment_notification {
            Some(user.email.clone())
        } else {
            None
        };
        let token_id = start_user_enrollment(
            &mut user,
            &mut transaction,
            &session.user,
            email.clone(),
            token_expiration_time_seconds,
            public_proxy_url.clone(),
            false, // notifications sent post-commit to avoid email/DB state mismatch
        )
        .await?;
        if let Some(addr) = email {
            pending_notifications.push((token_id, addr));
        }
        started.push(user);
    }
    transaction.commit().await?;

    info!(
        "User {} bulk-started enrollment for {} user(s) ({} skipped)",
        session.user.username,
        started.len(),
        skipped
    );
    for (token_id, email) in pending_notifications {
        send_enrollment_invitation(&token_id, &email, &appstate.pool, public_proxy_url.clone())
            .await;
    }
    for user in started.iter().cloned() {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::EnrollmentTokenAdded { user }),
        })?;
    }

    Ok(ApiResponse::new(
        json!({ "started": started.len(), "skipped": skipped }),
        StatusCode::OK,
    ))
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
