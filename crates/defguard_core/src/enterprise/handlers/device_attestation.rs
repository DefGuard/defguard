use axum::{
    extract::{Path, Query},
    http::StatusCode,
};
use chrono::Utc;
use defguard_certs::cert_der_to_pem;
use serde::{Deserialize, Serialize};

#[cfg(feature = "openapi")]
use crate::handlers::ApiErrorResponse;
use crate::{
    auth::{AdminRole, SessionInfo},
    enterprise::tpm::trust_bundle::{TpmCaFilter, TpmCaInfo, TpmCaSort, tpm_trust_bundle},
    error::WebError,
    handlers::{
        ApiResponse, ApiResult,
        pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
    },
};

#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct TpmCaCertificatePem {
    pub pem: String,
}

/// List TPM CA certificates bundled with Defguard
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/api/v1/device-attestation/certificates",
    tag = "device attestation",
    params(
        ("page" = Option<u32>, Query, description = "Page number. Defaults to 1."),
        ("per_page" = Option<u32>, Query, description = "Number of items per page, from 1 to 100. Defaults to 50."),
        ("search" = Option<String>, Query, description = "Case-insensitive search across common name, organization, subject, issuer, vendor, serial number and SHA-256."),
        ("sort_by" = Option<String>, Query, description = "Sort key: `name`, `vendor`, `kind` or `not_after`. Defaults to `name`."),
        ("sort_order" = Option<String>, Query, description = "Sort direction: `asc` or `desc`. Defaults to `asc`."),
    ),
    responses(
        (status = 200, description = "Paginated list of bundled TPM CA certificates.", body = PaginatedApiResponse<TpmCaInfo>),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
))]
pub async fn list_tpm_ca_certificates(
    _admin: AdminRole,
    session: SessionInfo,
    pagination: Query<PaginationParams>,
    filter: Query<TpmCaFilter>,
    sort: Query<TpmCaSort>,
) -> PaginatedApiResult<TpmCaInfo> {
    debug!(
        "User {} listing bundled TPM CA certificates",
        session.user.username
    );
    let pagination = pagination.0;
    let (entries, total) = tpm_trust_bundle().query(
        &filter,
        &sort,
        pagination.offset(),
        pagination.per_page(),
        Utc::now(),
    );
    let certificates = entries.into_iter().map(|e| e.info.clone()).collect();
    Ok(PaginatedApiResponse::new(certificates, pagination, total))
}

/// Get a bundled TPM CA certificate in PEM format
#[cfg_attr(feature = "openapi", utoipa::path(
    get,
    path = "/api/v1/device-attestation/certificates/{sha256}/pem",
    tag = "device attestation",
    params(
        ("sha256" = String, Path, description = "SHA-256 of the certificate DER, hex encoded."),
    ),
    responses(
        (status = 200, description = "Certificate in PEM format.", body = TpmCaCertificatePem),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "No bundled certificate has this SHA-256.", body = ApiErrorResponse, example = json!({"msg": "TPM CA certificate not found"})),
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
))]
pub async fn get_tpm_ca_certificate_pem(
    _admin: AdminRole,
    session: SessionInfo,
    Path(sha256): Path<String>,
) -> ApiResult {
    debug!(
        "User {} downloading bundled TPM CA certificate {sha256}",
        session.user.username
    );
    let entry = tpm_trust_bundle()
        .find(&sha256)
        .ok_or_else(|| WebError::ObjectNotFound("TPM CA certificate not found".into()))?;
    let pem = cert_der_to_pem(&entry.der)?;
    Ok(ApiResponse::json(
        TpmCaCertificatePem { pem },
        StatusCode::OK,
    ))
}
