use crate::{
    enterprise::license::validate_license,
    handlers::{ApiResponse, ApiResult},
};

pub mod openid_login;
pub mod openid_providers;

use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::{request::Parts, StatusCode},
};

use crate::{appstate::AppState, error::WebError};

pub struct LicenseInfo {
    pub valid: bool,
}

#[async_trait]
impl<S> FromRequestParts<S> for LicenseInfo
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = WebError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let appstate = AppState::from_ref(state);

        let license = appstate
            .license
            .lock()
            .expect("Failed to acquire lock on the license.");

        match validate_license((*license).as_ref()) {
            // Useless struct, but may come in handy later
            Ok(_) => Ok(LicenseInfo { valid: true }),
            Err(e) => Err(WebError::Forbidden(e.to_string())),
        }
    }
}

pub async fn check_enterprise_status(State(appstate): State<AppState>) -> ApiResult {
    let license = appstate
        .license
        .lock()
        .expect("Failed to acquire lock on the license.");

    let valid = validate_license((*license).as_ref()).is_ok();

    Ok(ApiResponse {
        json: serde_json::json!({ "enabled": valid }),
        status: StatusCode::OK,
    })
}
