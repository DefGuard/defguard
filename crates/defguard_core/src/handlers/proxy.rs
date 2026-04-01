use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use defguard_certs::{CertificateAuthority, Csr, DnType, generate_key_pair};
use defguard_common::{
    db::{
        Id,
        models::{Certificates, ProxyCertSource, proxy::Proxy},
    },
    types::proxy::{ProxyControlMessage, ProxyInfo},
};
use serde_json::Value;
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ProxyUpdateData {
    pub name: String,
    pub enabled: bool,
}

#[utoipa::path(
    get,
    path = "/api/v1/proxy",
    responses(
        (status = 200, description = "Edge list", body = [ProxyInfo]),
        (status = 401, description = "Unauthorized to get edge list.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to get edge list.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to get edge list.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn proxy_list(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} displaying proxy list", session.user.username);
    let proxies = Proxy::list(&appstate.pool).await?;
    info!("User {} displayed proxy list", session.user.username);

    Ok(ApiResponse::json(proxies, StatusCode::OK))
}

#[utoipa::path(
    get,
    path = "/api/v1/proxy/{proxy_id}",
    responses(
        (status = 200, description = "Edge details", body = Proxy),
        (status = 401, description = "Unauthorized to get edge details.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to get edge details.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge not found", body = ApiResponse, example = json!({"msg": "network not found"})),
        (status = 500, description = "Unable to get edge details.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn proxy_details(
    Path(proxy_id): Path<Id>,
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} displaying details for proxy {proxy_id}",
        session.user.username
    );
    let proxy = Proxy::find_by_id(&appstate.pool, proxy_id).await?;
    let response = match proxy {
        Some(proxy) => ApiResponse::json(proxy, StatusCode::OK),
        None => ApiResponse::json(Value::Null, StatusCode::NOT_FOUND),
    };
    info!(
        "User {} displayed details for proxy {proxy_id}",
        session.user.username
    );

    Ok(response)
}

#[utoipa::path(
    put,
    path = "/api/v1/proxy/{proxy_id}",
    request_body = Proxy,
    responses(
        (status = 200, description = "Successfully modified edge.", body = ProxyUpdateData),
        (status = 401, description = "Unauthorized to modify edge.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to modify an edge.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge not found", body = ApiResponse, example = json!({"msg": "proxy not found"})),
        (status = 500, description = "Unable to modify edge.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn update_proxy(
    _role: AdminRole,
    Path(proxy_id): Path<Id>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<ProxyUpdateData>,
) -> ApiResult {
    debug!("User {} updating proxy {proxy_id}", session.user.username);
    let proxy = Proxy::find_by_id(&appstate.pool, proxy_id).await?;

    let Some(mut proxy) = proxy else {
        warn!("Proxy {proxy_id} not found");
        return Ok(ApiResponse::json(Value::Null, StatusCode::NOT_FOUND));
    };
    let before = proxy.clone();

    proxy.name = data.name;
    proxy.enabled = data.enabled;
    proxy.modified_by = session.user.fullname();
    proxy.modified_at = Utc::now().naive_utc();
    proxy.save(&appstate.pool).await?;

    if before.enabled != proxy.enabled {
        if data.enabled {
            if let Err(err) = appstate
                .proxy_control_tx
                .send(ProxyControlMessage::StartConnection(proxy.id))
                .await
            {
                error!(
                    "Failed to start Proxy {}, it may be disconnected: {err:?}",
                    proxy.id
                );
            }
        } else if let Err(err) = appstate
            .proxy_control_tx
            .send(ProxyControlMessage::ShutdownConnection(proxy.id))
            .await
        {
            error!(
                "Failed to shutdown Proxy {}, it may be disconnected: {err:?}",
                proxy.id
            );
        }
    }

    info!("User {} updated proxy {proxy_id}", session.user.username);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::ProxyModified {
            before,
            after: proxy.clone(),
        }),
    })?;

    Ok(ApiResponse::json(proxy, StatusCode::OK))
}

#[utoipa::path(
    delete,
    path = "/api/v1/proxy/{proxy_id}",
    request_body = Proxy,
    responses(
        (status = 200, description = "Successfully deleted edge.", body = ApiResponse),
        (status = 401, description = "Unauthorized to delete edge.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission delete an edge.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Edge not found", body = ApiResponse, example = json!({"msg": "proxy not found"})),
        (status = 500, description = "Unable to delete edge.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_proxy(
    _role: AdminRole,
    Path(proxy_id): Path<Id>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
) -> ApiResult {
    debug!("User {} deleteing proxy {proxy_id}", session.user.username);
    let proxy = Proxy::find_by_id(&appstate.pool, proxy_id).await?;

    let Some(proxy) = proxy else {
        warn!("Proxy {proxy_id} not found");
        return Ok(ApiResponse::json(Value::Null, StatusCode::NOT_FOUND));
    };

    // Disconnect and purge the proxy
    if let Err(err) = appstate
        .proxy_control_tx
        .send(ProxyControlMessage::Purge(proxy.id))
        .await
    {
        error!(
            "Failed to purge Proxy {}, it may be disconnected: {err:?}",
            proxy.id
        );
    }

    proxy.clone().delete(&appstate.pool).await?;

    info!("User {} deleted proxy {proxy_id}", session.user.username);

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::ProxyDeleted { proxy }),
    })?;

    Ok(ApiResponse::default())
}

/// Upload a custom PEM certificate + private key for proxy HTTPS.
///
/// Sets `proxy_cert_source = custom` and immediately broadcasts the cert to all
/// connected proxies so they restart with HTTPS.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CustomCertUpload {
    /// PEM-encoded certificate chain.
    pub cert_pem: String,
    /// PEM-encoded private key.
    pub key_pem: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/proxy/cert/upload",
    request_body = CustomCertUpload,
    responses(
        (status = 200, description = "Custom certificate uploaded and broadcast to all proxies.", body = ApiResponse),
        (status = 401, description = "Unauthorized.", body = ApiResponse),
        (status = 403, description = "Forbidden.", body = ApiResponse),
        (status = 500, description = "Internal server error.", body = ApiResponse)
    ),
    security(("cookie" = []), ("api_token" = []))
)]
pub(crate) async fn proxy_cert_upload(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<CustomCertUpload>,
) -> ApiResult {
    debug!(
        "User {} uploading custom proxy certificate",
        session.user.username
    );

    let mut certs = Certificates::get_or_default(&appstate.pool)
        .await
        .map_err(|err| {
            error!("Failed to load certificates: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;
    certs.proxy_http_cert_pem = Some(data.cert_pem.clone());
    certs.proxy_http_cert_key_pem = Some(data.key_pem.clone());
    certs.proxy_http_cert_source = ProxyCertSource::Custom;
    certs.save(&appstate.pool).await.map_err(|err| {
        error!("Failed to save custom proxy cert: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    broadcast_https_certs(&appstate, data.cert_pem, data.key_pem).await;

    info!(
        "User {} uploaded custom proxy certificate",
        session.user.username
    );
    Ok(ApiResponse::default())
}

/// Provision a proxy HTTPS certificate signed by the built-in Core CA.
///
/// The certificate is issued for the given SANs (hostnames / IPs), signed
/// with the Core CA stored in settings, saved as `proxy_cert_source = self_signed`,
/// and broadcast to all connected proxies.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SelfSignedCertRequest {
    /// List of Subject Alternative Names (domain names or IP addresses).
    pub san: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/proxy/cert/self-signed",
    request_body = SelfSignedCertRequest,
    responses(
        (status = 200, description = "Self-signed certificate provisioned and broadcast.", body = ApiResponse),
        (status = 400, description = "Invalid request (e.g. CA not configured).", body = ApiResponse),
        (status = 401, description = "Unauthorized.", body = ApiResponse),
        (status = 403, description = "Forbidden.", body = ApiResponse),
        (status = 500, description = "Internal server error.", body = ApiResponse)
    ),
    security(("cookie" = []), ("api_token" = []))
)]
pub(crate) async fn proxy_cert_self_signed(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    Json(data): Json<SelfSignedCertRequest>,
) -> ApiResult {
    debug!(
        "User {} provisioning self-signed proxy certificate",
        session.user.username
    );

    let mut certs = Certificates::get_or_default(&appstate.pool)
        .await
        .map_err(|err| {
            error!("Failed to load certificates: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let (Some(ca_cert_der), Some(ca_key_der)) =
        (certs.ca_cert_der.clone(), certs.ca_key_der.clone())
    else {
        warn!("CA not configured; cannot issue self-signed proxy cert");
        return Ok(ApiResponse::json(
            serde_json::json!({"msg": "Core CA is not configured"}),
            StatusCode::BAD_REQUEST,
        ));
    };

    // Build CA from stored DER blobs.
    let ca =
        CertificateAuthority::from_cert_der_key_pair(&ca_cert_der, &ca_key_der).map_err(|err| {
            error!("Failed to load Core CA: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let Some(common_name) = data.san.first() else {
        return Err(WebError::BadRequest(
            "At least one SAN entry is required to issue a certificate".to_string(),
        ));
    };

    // Generate a new leaf key pair + CSR.
    let leaf_key = generate_key_pair().map_err(|err| {
        error!("Failed to generate leaf key pair: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let csr = Csr::new(
        &leaf_key,
        &data.san,
        vec![(DnType::CommonName, common_name.as_str())],
    )
    .map_err(|err| {
        error!("Failed to build CSR: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let signed = ca
        .sign_csr(&csr)
        .map_err(|err: defguard_certs::CertificateError| {
            error!("Failed to sign CSR with Core CA: {err}");
            WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
        })?;

    let cert_pem = signed.pem();
    let key_pem = leaf_key.serialize_pem();

    certs.proxy_http_cert_pem = Some(cert_pem.clone());
    certs.proxy_http_cert_key_pem = Some(key_pem.clone());
    certs.proxy_http_cert_source = ProxyCertSource::SelfSigned;
    certs.save(&appstate.pool).await.map_err(|err| {
        error!("Failed to save self-signed proxy cert: {err}");
        WebError::Http(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    broadcast_https_certs(&appstate, cert_pem, key_pem).await;

    info!(
        "User {} provisioned self-signed proxy certificate (SAN: {:?})",
        session.user.username, data.san
    );
    Ok(ApiResponse::default())
}

/// Broadcast an `HttpsCerts` message to all currently connected proxies via the proxy manager.
async fn broadcast_https_certs(appstate: &AppState, cert_pem: String, key_pem: String) {
    if let Err(err) = appstate
        .proxy_control_tx
        .send(ProxyControlMessage::BroadcastHttpsCerts { cert_pem, key_pem })
        .await
    {
        error!("Failed to broadcast HttpsCerts to proxies: {err:?}");
    }
}
