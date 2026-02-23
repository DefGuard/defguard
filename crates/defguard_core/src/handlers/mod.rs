use axum::{
    Json,
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_client_ip::InsecureClientIp;
use axum_extra::{TypedHeader, headers::UserAgent};
use defguard_common::{
    db::{
        Id, NoId,
        models::{Device, User},
    },
    types::user_info::UserInfo,
};
use defguard_static_ip::error::StaticIpError;
use serde_json::{Value, json};
use sqlx::PgPool;
use utoipa::ToSchema;
use webauthn_rs::prelude::RegisterPublicKeyCredential;

use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::WebHook,
    enterprise::{db::models::acl::AclError, license::LicenseError},
    error::WebError,
    events::ApiRequestContext,
};

pub(crate) mod activity_log;
pub(crate) mod app_info;
pub mod auth;
pub mod component_setup;
pub(crate) mod forward_auth;
pub mod gateway;
pub(crate) mod group;
pub(crate) mod location_stats;
pub mod mail;
pub mod network_devices;
pub mod openid_clients;
pub mod openid_flow;
pub(crate) mod pagination;
pub mod proxy;
pub mod settings;
pub(crate) mod ssh_authorized_keys;
pub(crate) mod static_ips;
pub(crate) mod support;
pub(crate) mod updates;
pub mod user;
pub(crate) mod webhooks;
pub mod wireguard;
pub mod worker;
pub(crate) mod yubikey;

pub static SESSION_COOKIE_NAME: &str = "defguard_session";
pub(crate) static SIGN_IN_COOKIE_NAME: &str = "defguard_sign_in";
pub(crate) const SIGN_IN_COOKIE_MAX_AGE: time::Duration = time::Duration::minutes(10);
pub(crate) const DEFAULT_API_PAGE_SIZE: u32 = 50;

#[derive(Default, ToSchema)]
pub struct ApiResponse {
    json: Value,
    #[schema(value_type = u16)]
    status: StatusCode,
}

impl ApiResponse {
    /// Build a new [`ApiResponse`].
    #[must_use]
    pub fn new(json: Value, status: StatusCode) -> Self {
        Self { json, status }
    }

    /// Response with `json` set to "{}", and a status code.
    #[must_use]
    pub fn with_status(status: StatusCode) -> Self {
        Self {
            json: Value::Object(serde_json::Map::new()),
            status,
        }
    }

    /// Response with serializable value for JSON, and a status code.
    #[must_use]
    pub fn json<T: serde::Serialize>(value: T, status: StatusCode) -> Self {
        let json = serde_json::to_value(value).expect("Failed to convert value to JSON");
        Self { json, status }
    }
}

impl From<WebError> for ApiResponse {
    fn from(web_error: WebError) -> ApiResponse {
        match web_error {
            WebError::ObjectNotFound(msg) => {
                ApiResponse::new(json!({"msg": msg}), StatusCode::NOT_FOUND)
            }
            WebError::ObjectAlreadyExists(msg) => {
                ApiResponse::new(json!({"msg": msg}), StatusCode::CONFLICT)
            }
            WebError::Authorization(msg) => {
                error!(msg);
                ApiResponse::new(json!({"msg": msg}), StatusCode::UNAUTHORIZED)
            }
            WebError::Authentication => ApiResponse::with_status(StatusCode::UNAUTHORIZED),
            WebError::Forbidden(msg) => {
                error!(msg);
                ApiResponse::new(json!({"msg": msg}), StatusCode::FORBIDDEN)
            }
            WebError::DbError(_)
            | WebError::Grpc(_)
            | WebError::WebauthnRegistration(_)
            | WebError::Serialization(_)
            | WebError::ModelError(_)
            | WebError::EmailMfa(_)
            | WebError::ClientIpError
            | WebError::FirewallError(_)
            | WebError::ApiEventChannelError(_)
            | WebError::ActivityLogStreamError(_)
            | WebError::UrlParseError(_)
            | WebError::CertificateError(_) => {
                error!("{web_error}");
                ApiResponse::new(
                    json!({"msg": "Internal server error"}),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
            WebError::StaticIpError(err) => match err {
                StaticIpError::InvalidIpAssignment(err) => {
                    ApiResponse::new(json!({"msg": err.to_string()}), StatusCode::BAD_REQUEST)
                }
                StaticIpError::NetworkNotFound(_) | StaticIpError::DeviceNotInNetwork(_, _) => {
                    error!("{err}");
                    ApiResponse::new(json!({"msg": err.to_string()}), StatusCode::BAD_REQUEST)
                }
                StaticIpError::SqlxError(_) => {
                    error!("{err}");
                    ApiResponse::new(
                        json!({"msg": "Internal server error"}),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
            },
            WebError::AclError(err) => match err {
                AclError::ParseIntError(_)
                | AclError::IpNetworkError(_)
                | AclError::AddrParseError(_)
                | AclError::InvalidRelationError(_)
                | AclError::InvalidPortsFormat(_) => ApiResponse::new(
                    json!({"msg": "Unprocessable entity"}),
                    StatusCode::UNPROCESSABLE_ENTITY,
                ),
                AclError::InvalidIpRangeError(err) => ApiResponse::new(
                    json!({"msg": format!("Invalid IP range: {err}")}),
                    StatusCode::UNPROCESSABLE_ENTITY,
                ),
                AclError::RuleNotFoundError(id) => ApiResponse::new(
                    json!({"msg": format!("Rule {id} not found")}),
                    StatusCode::NOT_FOUND,
                ),
                AclError::RuleAlreadyAppliedError(id) => ApiResponse::new(
                    json!({"msg": format!("Rule {id} already applied")}),
                    StatusCode::BAD_REQUEST,
                ),
                AclError::AliasNotFoundError(id) => ApiResponse::new(
                    json!({"msg": format!("Alias {id} not found")}),
                    StatusCode::NOT_FOUND,
                ),
                AclError::AliasAlreadyAppliedError(id) => ApiResponse::new(
                    json!({"msg": format!("Alias {id} already applied")}),
                    StatusCode::BAD_REQUEST,
                ),
                AclError::AliasUsedByRulesError(id) => ApiResponse::new(
                    json!({"msg": format!("Alias {id} is used by some existing ACL rules")}),
                    StatusCode::BAD_REQUEST,
                ),
                AclError::DbError(_) | AclError::FirewallError(_) => {
                    error!("{err}");
                    ApiResponse::new(
                        json!({"msg": "Internal server error"}),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                }
                AclError::CannotModifyDeletedRuleError(id) => ApiResponse::new(
                    json!({"msg": format!("Cannot modify deleted ACL rule {id}")}),
                    StatusCode::BAD_REQUEST,
                ),
                AclError::CannotUseModifiedAliasInRuleError(alias_ids) => ApiResponse::new(
                    json!({"msg": format!("Cannot use modified alias in ACL rule {alias_ids:?}")}),
                    StatusCode::BAD_REQUEST,
                ),
            },
            WebError::Http(status) => {
                error!("{status}");
                ApiResponse::new(
                    json!({"msg": status.canonical_reason().unwrap_or_default()}),
                    status,
                )
            }
            WebError::TooManyLoginAttempts(_) => ApiResponse::new(
                json!({"msg": "Too many login attempts"}),
                StatusCode::TOO_MANY_REQUESTS,
            ),
            WebError::PubkeyValidation(msg)
            | WebError::PubkeyExists(msg)
            | WebError::BadRequest(msg) => {
                error!(msg);
                ApiResponse::new(json!({"msg": msg}), StatusCode::BAD_REQUEST)
            }
            WebError::TemplateError(err) => {
                error!("Template error: {err}");
                ApiResponse::new(
                    json!({"msg": "Internal server error"}),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
            WebError::LicenseError(err) => match err {
                LicenseError::DecodeError(msg) | LicenseError::InvalidLicense(msg) => {
                    warn!(msg);
                    ApiResponse::new(json!({"msg": msg}), StatusCode::BAD_REQUEST)
                }
                LicenseError::SignatureMismatch => {
                    let msg = "License signature doesn't match its content";
                    warn!(msg);
                    ApiResponse::new(json!({"msg": msg}), StatusCode::BAD_REQUEST)
                }
                LicenseError::InvalidSignature => {
                    let msg = "License signature is malformed and couldn't be read";
                    warn!(msg);
                    ApiResponse::new(json!({"msg": msg}), StatusCode::BAD_REQUEST)
                }
                LicenseError::LicenseNotFound => {
                    let msg = "License not found";
                    warn!(msg);
                    ApiResponse::new(json!({"msg": msg}), StatusCode::NOT_FOUND)
                }
                _ => {
                    error!("License error: {err}");
                    ApiResponse::new(
                        json!({"msg": "Internal server error"}),
                        StatusCode::FORBIDDEN,
                    )
                }
            },
        }
    }
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        ApiResponse::from(self).into_response()
    }
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        let mut response = Json(self.json).into_response();
        *response.status_mut() = self.status;
        response
    }
}

pub type ApiResult = Result<ApiResponse, WebError>;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct Auth {
    username: String,
    password: String,
}

impl Auth {
    #[must_use]
    pub fn new<S: Into<String>>(username: S, password: S) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AuthTotp {
    pub secret: String,
}

impl AuthTotp {
    #[must_use]
    pub fn new<S: Into<String>>(secret: S) -> Self {
        Self {
            secret: secret.into(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AuthCode {
    code: String,
}

impl AuthCode {
    #[must_use]
    pub fn new<S: Into<String>>(code: S) -> Self {
        Self { code: code.into() }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct GroupInfo {
    pub id: Id,
    pub name: String,
    pub members: Vec<String>,
    pub vpn_locations: Vec<String>,
    pub is_admin: bool,
}

impl GroupInfo {
    #[must_use]
    pub fn new<S: Into<String>>(
        id: Id,
        name: S,
        members: Vec<String>,
        vpn_locations: Vec<String>,
        is_admin: bool,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            members,
            vpn_locations,
            is_admin,
        }
    }
}

/// Dedicated `GroupInfo` variant for group modification operations.
#[derive(Deserialize, Serialize, ToSchema)]
pub struct EditGroupInfo {
    pub name: String,
    pub members: Vec<String>,
    pub is_admin: bool,
}

impl EditGroupInfo {
    #[must_use]
    pub fn new<S: Into<String>>(name: S, members: Vec<String>, is_admin: bool) -> Self {
        Self {
            name: name.into(),
            members,
            is_admin,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct Username {
    pub username: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct AddUserData {
    pub username: String,
    pub last_name: String,
    pub first_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub password: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct StartEnrollmentRequest {
    #[serde(default)]
    pub send_enrollment_notification: bool,
    pub email: Option<String>,
    pub token_expiration_time: Option<String>,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct PasswordChangeSelf {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct PasswordChange {
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct WebAuthnRegistration {
    pub name: String,
    pub rpkc: RegisterPublicKeyCredential,
}

#[derive(Deserialize)]
pub struct RecoveryCode {
    code: String,
}

#[derive(Serialize)]
pub struct RecoveryCodes {
    codes: Option<Vec<String>>,
}

impl RecoveryCodes {
    #[must_use]
    pub fn new(codes: Option<Vec<String>>) -> Self {
        Self { codes }
    }
}

#[derive(Deserialize)]
pub struct WebHookData {
    pub url: String,
    pub description: String,
    pub token: String,
    pub enabled: bool,
    pub on_user_created: bool,
    pub on_user_deleted: bool,
    pub on_user_modified: bool,
    pub on_hwkey_provision: bool,
}

impl From<WebHookData> for WebHook {
    fn from(data: WebHookData) -> Self {
        Self {
            id: NoId,
            url: data.url,
            description: data.description,
            token: data.token,
            enabled: data.enabled,
            on_user_created: data.on_user_created,
            on_user_deleted: data.on_user_deleted,
            on_user_modified: data.on_user_modified,
            on_hwkey_provision: data.on_hwkey_provision,
        }
    }
}

/// Return type needed for knowing if a user came from OpenID flow.
/// If so, fill in the optional URL field to redirect him later.
#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: UserInfo,
    pub url: Option<String>,
}

/// Try to fetch [`User`] if the username is of the currently logged in user, or
/// the logged in user is an admin.
pub async fn user_for_admin_or_self(
    pool: &PgPool,
    session: &SessionInfo,
    username: &str,
) -> Result<User<Id>, WebError> {
    if session.user.username == username || session.is_admin {
        debug!(
            "The user meets one or both of these conditions: \
            1) the user from the current session has admin privileges, \
            2) the user performs this operation on themself."
        );
        if let Some(user) = User::find_by_username(pool, username).await? {
            debug!("User {} has been found in database.", user.username);
            Ok(user)
        } else {
            debug!("User with {username} does not exist in database.");
            Err(WebError::ObjectNotFound(format!(
                "user {username} not found"
            )))
        }
    } else {
        debug!(
            "User from the current session doesn't have enough privileges to do this operation."
        );
        Err(WebError::Forbidden("requires privileged access".into()))
    }
}

/// Try to fetch [`Device'] if the device.id is of the currently logged in user, or
/// the logged in user is an admin.
pub async fn device_for_admin_or_self<'e, E: sqlx::PgExecutor<'e>>(
    executor: E,
    session: &SessionInfo,
    id: Id,
) -> Result<Device<Id>, WebError> {
    let fetch = if session.is_admin {
        Device::find_by_id(executor, id).await
    } else {
        Device::find_by_id_and_username(executor, id, &session.user.username).await
    }?;

    match fetch {
        Some(device) => Ok(device),
        None => Err(WebError::ObjectNotFound(format!(
            "device id {id} not found"
        ))),
    }
}

impl<S> FromRequestParts<S> for ApiRequestContext
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(user_agent) = TypedHeader::<UserAgent>::from_request_parts(parts, state)
            .await
            .map_err(|_| WebError::BadRequest("Missing UserAgent header".to_string()))?;
        let InsecureClientIp(insecure_ip) = InsecureClientIp::from_request_parts(parts, state)
            .await
            .map_err(|_| WebError::BadRequest("Missing client IP".to_string()))?;
        let session = if let Some(cached) = parts.extensions.get::<SessionInfo>() {
            cached.clone()
        } else {
            SessionInfo::from_request_parts(parts, state).await?
        };

        // Store session info into request extensions so future extractors can use it
        parts.extensions.insert(session.clone());
        Ok(ApiRequestContext::new(
            session.user.id,
            session.user.username,
            insecure_ip,
            user_agent.to_string(),
        ))
    }
}
