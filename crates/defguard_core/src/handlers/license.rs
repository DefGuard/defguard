use axum::{Json, http::StatusCode};
use utoipa::ToSchema;

use super::{ApiResponse, ApiResult};
use crate::{
    enterprise::{
        license::License,
        limits::{Counts, get_counts},
    },
    grpc::proto::enterprise::license::LicenseLimits,
};

#[derive(Deserialize, ToSchema)]
pub struct CheckParams {
    license: String,
}

#[derive(Serialize)]
pub struct CheckResult {
    limits: Option<LicenseLimits>,
    counts: Counts,
}

/// Check given license. Return [`LicenseLimits`].
#[utoipa::path(
    post,
    path = "/api/v1/license/check",
    request_body = CheckParams,
    responses(
        (
            status = 200,
            description = "Decoded license limits.",
            // TODO: uncomment when LicenseLimits and Counts implement ToSchema.
            // body = CheckResult,
            example = json!({
                "users": 100,
                "devices": 250,
                "locations": 10,
                "network_devices": 50
            })
        ),
        (status = 400, description = "Invalid license key.", body = ApiResponse, example = json!({"msg": "License signature doesn't match its content"})),
        (status = 404, description = "License not found.", body = ApiResponse, example = json!({"msg": "License not found"}))
    )
)]
pub(crate) async fn license_check(Json(params): Json<CheckParams>) -> ApiResult {
    let license = License::from_base64(params.license.trim())?;

    Ok(ApiResponse::json(
        CheckResult {
            limits: license.limits,
            counts: get_counts().clone(),
        },
        StatusCode::OK,
    ))
}
