#[cfg(feature = "wireguard")]
use crate::db::Device;
use crate::{
    auth::SessionInfo,
    db::{DbPool, User},
    error::OriWebError,
};
use rocket::{
    http::{ContentType, Status},
    request::Request,
    response::{Responder, Response},
    serde::json::{serde_json::json, Value},
};
use std::env;
use webauthn_rs::prelude::RegisterPublicKeyCredential;

pub(crate) mod auth;
pub(crate) mod group;
pub(crate) mod license;
pub(crate) mod settings;
pub(crate) mod user;
pub(crate) mod version;
pub(crate) mod webhooks;
#[cfg(feature = "wireguard")]
pub mod wireguard;

#[derive(Default)]
pub struct ApiResponse {
    pub json: Value,
    pub status: Status,
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub type ApiResult = Result<ApiResponse, OriWebError>;

fn internal_server_error(msg: &str) -> (Value, Status) {
    error!("{}", msg);
    (
        json!({"msg": "Internal server error"}),
        Status::InternalServerError,
    )
}

impl<'r, 'o: 'r> Responder<'r, 'o> for OriWebError {
    fn respond_to(self, request: &'r Request<'_>) -> Result<Response<'o>, Status> {
        let (json, status) = match self {
            OriWebError::ObjectNotFound(msg) => (json!({ "msg": msg }), Status::NotFound),
            OriWebError::Authorization(msg) => {
                error!("{}", msg);
                (json!({ "msg": msg }), Status::Unauthorized)
            }
            OriWebError::Forbidden(msg) => {
                error!("{}", msg);
                (json!({ "msg": msg }), Status::Forbidden)
            }
            OriWebError::DbError(msg) => internal_server_error(&msg),
            OriWebError::Grpc(msg) => internal_server_error(&msg),
            OriWebError::Ldap(msg) => internal_server_error(&msg),
            OriWebError::IncorrectUsername(msg) => {
                error!("{}", msg);
                (json!({ "msg": msg }), Status::BadRequest)
            }
            OriWebError::Serialization(msg) => internal_server_error(&msg),
            OriWebError::ModelError(msg) => internal_server_error(&msg),
            OriWebError::Http(status) => {
                error!("{}", status);
                (json!({ "msg": status.reason_lossy() }), status)
            }
        };
        Response::build_from(json.respond_to(request)?)
            .status(status)
            .header(ContentType::JSON)
            .raw_header("X-Defguard-Version", VERSION)
            .ok()
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for ApiResponse {
    fn respond_to(self, request: &'r Request<'_>) -> Result<Response<'o>, Status> {
        Response::build_from(self.json.respond_to(request)?)
            .status(self.status)
            .header(ContentType::JSON)
            .raw_header("X-Defguard-Version", VERSION)
            .ok()
    }
}

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
    pub phone: String,
    pub password: String,
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

/// Try to fetch [`User`] if the username is of the currently logged in user, or
/// the logged in user is an admin.
pub async fn user_for_admin_or_self(
    pool: &DbPool,
    session: &SessionInfo,
    username: &str,
) -> Result<User, OriWebError> {
    if session.user.username == username || session.is_admin {
        match User::find_by_username(pool, username).await? {
            Some(user) => Ok(user),
            None => Err(OriWebError::ObjectNotFound(format!(
                "user {} not found",
                username
            ))),
        }
    } else {
        Err(OriWebError::Forbidden("requires privileged access".into()))
    }
}

/// Try to fetch [`Device'] if the device.id is of the currently logged in user, or
/// the logged in user is an admin.
#[cfg(feature = "wireguard")]
pub async fn device_for_admin_or_self(
    pool: &DbPool,
    session: &SessionInfo,
    id: i64,
) -> Result<Device, OriWebError> {
    let fetch = if session.is_admin {
        Device::find_by_id(pool, id).await
    } else {
        Device::find_by_id_and_username(pool, id, &session.user.username).await
    }?;

    match fetch {
        Some(device) => Ok(device),
        None => Err(OriWebError::ObjectNotFound(format!(
            "device id {} not found",
            id
        ))),
    }
}
