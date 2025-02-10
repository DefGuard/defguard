use axum::{
    http::{HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use utoipa::ToSchema;
use webauthn_rs::prelude::RegisterPublicKeyCredential;

#[cfg(feature = "wireguard")]
use crate::db::Device;
use crate::{
    auth::SessionInfo,
    db::{Id, NoId, User, UserInfo, WebHook},
    enterprise::license::LicenseError,
    error::WebError,
    VERSION,
};

pub(crate) mod app_info;
pub(crate) mod auth;
pub(crate) mod forward_auth;
pub(crate) mod group;
pub(crate) mod mail;
pub mod network_devices;
#[cfg(feature = "openid")]
pub(crate) mod openid_clients;
#[cfg(feature = "openid")]
pub mod openid_flow;
pub(crate) mod settings;
pub(crate) mod ssh_authorized_keys;
pub(crate) mod support;
pub(crate) mod updates;
pub(crate) mod user;
pub(crate) mod webhooks;
#[cfg(feature = "wireguard")]
pub mod wireguard;
#[cfg(feature = "worker")]
pub mod worker;
pub(crate) mod yubikey;

pub(crate) static SESSION_COOKIE_NAME: &str = "defguard_session";
pub(crate) static SIGN_IN_COOKIE_NAME: &str = "defguard_sign_in";

#[derive(Default, ToSchema)]
pub struct ApiResponse {
    pub json: Value,
    #[schema(value_type = u16)]
    pub status: StatusCode,
}

impl ApiResponse {
    #[must_use]
    pub fn new(json: Value, status: StatusCode) -> Self {
        Self { json, status }
    }
}

impl From<WebError> for ApiResponse {
    fn from(web_error: WebError) -> ApiResponse {
        match web_error {
            WebError::ObjectNotFound(msg) => {
                ApiResponse::new(json!({ "msg": msg }), StatusCode::NOT_FOUND)
            }
            WebError::Authorization(msg) => {
                error!(msg);
                ApiResponse::new(json!({ "msg": msg }), StatusCode::UNAUTHORIZED)
            }
            WebError::Forbidden(msg) => {
                error!(msg);
                ApiResponse::new(json!({ "msg": msg }), StatusCode::FORBIDDEN)
            }
            WebError::DbError(_)
            | WebError::Grpc(_)
            | WebError::Ldap(_)
            | WebError::WebauthnRegistration(_)
            | WebError::Serialization(_)
            | WebError::ModelError(_)
            | WebError::ServerConfigMissing
            | WebError::EmailMfa(_)
            | WebError::ClientIpError => {
                error!("{web_error}");
                ApiResponse::new(
                    json!({"msg": "Internal server error"}),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
            WebError::Http(status) => {
                error!("{status}");
                ApiResponse::new(
                    json!({ "msg": status.canonical_reason().unwrap_or_default() }),
                    status,
                )
            }
            WebError::TooManyLoginAttempts(_) => ApiResponse::new(
                json!({ "msg": "Too many login attempts" }),
                StatusCode::TOO_MANY_REQUESTS,
            ),
            WebError::IncorrectUsername(msg)
            | WebError::PubkeyValidation(msg)
            | WebError::PubkeyExists(msg)
            | WebError::BadRequest(msg) => {
                error!(msg);
                ApiResponse::new(json!({ "msg": msg }), StatusCode::BAD_REQUEST)
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
                    ApiResponse::new(json!({ "msg": msg }), StatusCode::BAD_REQUEST)
                }
                LicenseError::SignatureMismatch => {
                    let msg = "License signature doesn't match its content";
                    warn!(msg);
                    ApiResponse::new(json!({ "msg": msg }), StatusCode::BAD_REQUEST)
                }
                LicenseError::InvalidSignature => {
                    let msg = "License signature is malformed and couldn't be read";
                    warn!(msg);
                    ApiResponse::new(json!({ "msg": msg }), StatusCode::BAD_REQUEST)
                }
                LicenseError::LicenseNotFound => {
                    let msg = "License not found";
                    warn!(msg);
                    ApiResponse::new(json!({ "msg": msg }), StatusCode::NOT_FOUND)
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
        let api_response = ApiResponse::from(self);
        api_response.into_response()
    }
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        let mut response = Json(self.json).into_response();
        response.headers_mut().insert(
            HeaderName::from_static("x-defguard-version"),
            HeaderValue::from_static(VERSION),
        );
        *response.status_mut() = self.status;
        response
    }
}

pub type ApiResult = Result<ApiResponse, WebError>;

#[derive(Deserialize, Serialize)]
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
    pub name: String,
    pub members: Vec<String>,
    pub vpn_locations: Vec<String>,
    pub is_admin: bool,
}

impl GroupInfo {
    #[must_use]
    pub fn new<S: Into<String>>(
        name: S,
        members: Vec<String>,
        vpn_locations: Vec<String>,
        is_admin: bool,
    ) -> Self {
        Self {
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
#[cfg(feature = "wireguard")]
pub async fn device_for_admin_or_self(
    pool: &PgPool,
    session: &SessionInfo,
    id: Id,
) -> Result<Device<Id>, WebError> {
    let fetch = if session.is_admin {
        Device::find_by_id(pool, id).await
    } else {
        Device::find_by_id_and_username(pool, id, &session.user.username).await
    }?;

    match fetch {
        Some(device) => Ok(device),
        None => Err(WebError::ObjectNotFound(format!(
            "device id {id} not found"
        ))),
    }
}
