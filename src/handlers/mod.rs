use axum::http::StatusCode;
use serde_json::{json, Value};
use webauthn_rs::prelude::RegisterPublicKeyCredential;

#[cfg(feature = "wireguard")]
use crate::db::Device;
use crate::{
    auth::SessionInfo,
    db::{DbPool, User, UserInfo},
    error::WebError,
    VERSION,
};

pub(crate) mod app_info;
// pub(crate) mod auth;
// pub mod forward_auth;
// pub(crate) mod group;
// pub(crate) mod mail;
// #[cfg(feature = "openid")]
// pub mod openid_clients;
// #[cfg(feature = "openid")]
// pub mod openid_flow;
// pub(crate) mod settings;
// pub(crate) mod support;
// pub(crate) mod user;
// pub(crate) mod webhooks;
// #[cfg(feature = "wireguard")]
// pub mod wireguard;
// #[cfg(feature = "worker")]
// pub mod worker;

#[derive(Default)]
pub struct ApiResponse {
    pub json: Value,
    pub status: StatusCode,
}

pub type ApiResult = Result<ApiResponse, WebError>;

// impl<'r, 'o: 'r> Responder<'r, 'o> for WebError {
//     fn respond_to(self, request: &'r Request<'_>) -> Result<Response<'o>, StatusCode> {
//         let (json, status) = match self {
//             WebError::ObjectNotFound(msg) => (json!({ "msg": msg }), StatusCode::NOT_FOUND),
//             WebError::Authorization(msg) => {
//                 error!("{}", msg);
//                 (json!({ "msg": msg }), StatusCode::UNAUTHORIZED)
//             }
//             WebError::Forbidden(msg) => {
//                 error!("{}", msg);
//                 (json!({ "msg": msg }), StatusCode::FORBIDDEN)
//             }
//             WebError::DbError(_)
//             | WebError::Grpc(_)
//             | WebError::Ldap(_)
//             | WebError::WebauthnRegistration(_)
//             | WebError::Serialization(_)
//             | WebError::ModelError(_)
//             | WebError::ServerConfigMissing => {
//                 error!("{self}");
//                 (
//                     json!({"msg": "Internal server error"}),
//                     StatusCode::INTERNAL_SERVER_ERROR,
//                 )
//             }
//             WebError::Http(status) => {
//                 error!("{}", status);
//                 (json!({ "msg": status.reason_lossy() }), status)
//             }
//             WebError::TooManyLoginAttempts(_) => (
//                 json!({ "msg": "Too many login attempts" }),
//                 StatusCode::TOO_MANY_REQUESTS,
//             ),
//             WebError::IncorrectUsername(msg)
//             | WebError::PubkeyValidation(msg)
//             | WebError::BadRequest(msg) => {
//                 error!("{}", msg);
//                 (json!({ "msg": msg }), StatusCode::BAD_REQUEST)
//             }
//             WebError::TemplateError(err) => {
//                 error!("Template error: {err}");
//                 (
//                     json!({"msg": "Internal server error"}),
//                     StatusCode::INTERNAL_SERVER_ERROR,
//                 )
//             }
//         };
//         Response::build_from(json.respond_to(request)?)
//             .status(status)
//             .header(ContentType::JSON)
//             .raw_header("X-Defguard-Version", VERSION)
//             .ok()
//     }
// }

// impl<'r, 'o: 'r> Responder<'r, 'o> for ApiResponse {
//     fn respond_to(self, request: &'r Request<'_>) -> Result<Response<'o>, StatusCode> {
//         Response::build_from(self.json.respond_to(request)?)
//             .status(self.status)
//             .header(ContentType::JSON)
//             .raw_header("X-Defguard-Version", VERSION)
//             .ok()
//     }
// }

#[derive(Deserialize, Serialize)]
pub struct Auth {
    username: String,
    password: String,
}

impl Auth {
    #[must_use]
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AuthTotp {
    pub secret: String,
}

impl AuthTotp {
    #[must_use]
    pub fn new(secret: String) -> Self {
        Self { secret }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AuthCode {
    code: u32,
}

impl AuthCode {
    #[must_use]
    pub fn new(code: u32) -> Self {
        Self { code }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Username {
    pub username: String,
}

#[derive(Deserialize, Serialize)]
pub struct AddUserData {
    pub username: String,
    pub last_name: String,
    pub first_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct StartEnrollmentRequest {
    #[serde(default)]
    pub send_enrollment_notification: bool,
    pub email: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct PasswordChangeSelf {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Deserialize, Serialize)]
pub struct PasswordChange {
    pub new_password: String,
}

#[derive(Deserialize)]
pub struct WalletSignature {
    pub address: String,
    pub signature: String,
}

#[derive(Deserialize, Serialize)]
pub struct WalletChallenge {
    pub id: i64,
    pub message: String,
}

#[derive(Deserialize)]
pub struct WalletChange {
    pub use_for_mfa: bool,
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

#[derive(Deserialize)]
pub struct WalletAddress {
    address: String,
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

/// Return type needed to know if user came from openid flow
/// with optional url to redirect him later if yes
#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: UserInfo,
    pub url: Option<String>,
}

/// Try to fetch [`User`] if the username is of the currently logged in user, or
/// the logged in user is an admin.
pub async fn user_for_admin_or_self(
    pool: &DbPool,
    session: &SessionInfo,
    username: &str,
) -> Result<User, WebError> {
    if session.user.username == username || session.is_admin {
        match User::find_by_username(pool, username).await? {
            Some(user) => Ok(user),
            None => Err(WebError::ObjectNotFound(format!(
                "user {username} not found"
            ))),
        }
    } else {
        Err(WebError::Forbidden("requires privileged access".into()))
    }
}

/// Try to fetch [`Device'] if the device.id is of the currently logged in user, or
/// the logged in user is an admin.
#[cfg(feature = "wireguard")]
pub async fn device_for_admin_or_self(
    pool: &DbPool,
    session: &SessionInfo,
    id: i64,
) -> Result<Device, WebError> {
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
