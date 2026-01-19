use axum::extract::State;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::handlers::LicenseInfo,
    handlers::{ApiResponse, ApiResult},
};

/// List ACL destinations.
#[utoipa::path(
    get,
    path = "/api/v1/acl/destination",
    responses(
        (status = OK, description = "ACL destination"),
    )
)]
pub(crate) async fn list_acl_destinations(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    Ok(ApiResponse::default())
}

/// Create ACL destination.
#[utoipa::path(
    post,
    path = "/api/v1/acl/destination",
    params(
        // ("data" = AddProviderData, Path, description = "OpenID provider data",)
    ),
    responses(
        (status = OK, description = "ACL destination"),
    )
)]
pub(crate) async fn create_acl_destination(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    Ok(ApiResponse::default())
}
