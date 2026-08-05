use axum::{
    Json,
    extract::{Path, State, rejection::JsonRejection},
};
use chrono::NaiveDateTime;
use defguard_common::db::{Id, models::gateway::Gateway};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, query_as};
use utoipa::ToSchema;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiErrorResponse, ApiResponse, ApiResult},
};

#[derive(Serialize, ToSchema)]
pub struct GatewayInfo {
    pub id: Id,
    pub location_id: Id,
    pub name: String,
    pub address: String,
    pub port: i32,
    pub connected_at: Option<NaiveDateTime>,
    pub disconnected_at: Option<NaiveDateTime>,
    pub connected: bool,
    pub certificate_serial: Option<String>,
    pub certificate_expiry: Option<NaiveDateTime>,
    pub version: Option<String>,
    pub enabled: bool,
    pub modified_at: NaiveDateTime,
    pub modified_by: String,
    pub location_name: String,
}

impl GatewayInfo {
    pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        query_as!(
            Self,
            "SELECT \
                g.id, g.location_id, g.name, g.address, g.port, g.connected_at, g.disconnected_at, \
                CASE \
                    WHEN g.connected_at IS NULL THEN false \
                    WHEN g.disconnected_at IS NULL THEN true \
                    WHEN g.connected_at >= g.disconnected_at THEN true \
                    ELSE false \
                END AS \"connected!\", \
                g.certificate_serial, g.certificate_expiry, g.version, \
                g.enabled, g.modified_at, g.modified_by, \
                wn.name AS location_name \
            FROM gateway g \
            JOIN wireguard_network wn ON g.location_id = wn.id",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_location_id(pool: &PgPool, location_id: Id) -> sqlx::Result<Vec<Self>> {
        query_as!(
            Self,
            "SELECT \
                g.id, g.location_id, g.name, g.address, g.port, g.connected_at, g.disconnected_at, \
                CASE \
                    WHEN g.connected_at IS NULL THEN false \
                    WHEN g.disconnected_at IS NULL THEN true \
                    WHEN g.connected_at >= g.disconnected_at THEN true \
                    ELSE false \
                END AS \"connected!\", \
                g.certificate_serial, g.certificate_expiry, g.version, \
                g.enabled, g.modified_at, g.modified_by, \
                wn.name AS location_name \
            FROM gateway g \
            JOIN wireguard_network wn ON g.location_id = wn.id \
            WHERE location_id = $1",
            location_id
        )
        .fetch_all(pool)
        .await
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GatewayUpdateData {
    pub name: String,
    pub enabled: bool,
}

/// List gateways in all locations
#[utoipa::path(
    get,
    path = "/api/v1/gateway",
    tag = "gateway",
    responses(
        (status = 200, description = "All gateways.", body = [GatewayInfo]),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to list gateways.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub async fn gateway_list(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} displaying gateway list", session.user.username);
    let gateways = GatewayInfo::list(&appstate.pool).await?;
    info!("User {} displayed gateway list", session.user.username);

    Ok(ApiResponse::json(gateways, StatusCode::OK))
}

/// Get a gateway
#[utoipa::path(
    get,
    path = "/api/v1/gateway/{gateway_id}",
    tag = "gateway",
    params(
        ("gateway_id" = i64, Path, description = "ID of the gateway."),
    ),
    responses(
        (status = 200, description = "Gateway details.", body = Gateway),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Gateway not found."),
        (status = 500, description = "Unable to get gateway.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn gateway_details(
    Path(gateway_id): Path<Id>,
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!(
        "User {} displaying details for gateway {gateway_id}",
        session.user.username
    );
    let gateway = Gateway::find_by_id(&appstate.pool, gateway_id).await?;
    let response = match gateway {
        Some(gateway) => ApiResponse::json(gateway, StatusCode::OK),
        None => ApiResponse::json(Value::Null, StatusCode::NOT_FOUND),
    };
    info!(
        "User {} displayed details for gateway {gateway_id}",
        session.user.username
    );

    Ok(response)
}

/// Rename a gateway, or enable or disable it
#[utoipa::path(
    put,
    path = "/api/v1/gateway/{gateway_id}",
    tag = "gateway",
    params(
        ("gateway_id" = i64, Path, description = "ID of the gateway."),
    ),
    request_body = GatewayUpdateData,
    responses(
        (status = 200, description = "Gateway updated.", body = GatewayInfo),
        (status = 400, description = "Malformed request body.", body = ApiErrorResponse, example = json!({"msg": "Failed to parse request data"})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Gateway not found.", body = ApiErrorResponse, example = json!({"msg": "gateway not found"})),
        (status = 500, description = "Unable to update gateway.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn update_gateway(
    _role: AdminRole,
    Path(gateway_id): Path<Id>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    payload: Result<Json<GatewayUpdateData>, JsonRejection>,
) -> ApiResult {
    let Json(data) = match payload {
        Ok(payload) => payload,
        Err(err) => {
            let msg = format!("Failed to parse request data: {err}");
            warn!(msg);
            return Err(WebError::BadRequest(msg));
        }
    };
    debug!(
        "User {} updating gateway {gateway_id}",
        session.user.username
    );
    let gateway = Gateway::find_by_id(&appstate.pool, gateway_id).await?;

    let Some(mut gateway) = gateway else {
        let msg = format!("Gateway {gateway_id} not found");
        warn!(msg);
        return Err(WebError::ObjectNotFound(msg));
    };
    let before = gateway.clone();

    gateway.name = data.name;
    gateway.enabled = data.enabled;
    gateway.save(&appstate.pool).await?;

    info!(
        "User {} updated gateway {gateway_id}",
        session.user.username
    );

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::GatewayModified {
            before,
            after: gateway.clone(),
        }),
    })?;

    Ok(ApiResponse::json(gateway, StatusCode::OK))
}

/// Delete a gateway
#[utoipa::path(
    delete,
    path = "/api/v1/gateway/{gateway_id}",
    tag = "gateway",
    params(
        ("gateway_id" = i64, Path, description = "ID of the gateway."),
    ),
    responses(
        (status = 200, description = "Gateway deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Gateway not found.", body = ApiErrorResponse, example = json!({"msg": "gateway not found"})),
        (status = 500, description = "Unable to delete gateway.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_gateway(
    _role: AdminRole,
    Path(gateway_id): Path<Id>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
) -> ApiResult {
    debug!(
        "User {} deleting gateway {gateway_id}",
        session.user.username
    );
    let gateway = Gateway::find_by_id(&appstate.pool, gateway_id).await?;

    let Some(gateway) = gateway else {
        let msg = format!("Gateway {gateway_id} not found");
        warn!(msg);
        return Err(WebError::ObjectNotFound(msg));
    };

    gateway.clone().delete(&appstate.pool).await?;

    info!(
        "User {} deleted gateway {gateway_id}",
        session.user.username
    );

    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::GatewayDeleted { gateway }),
    })?;

    Ok(ApiResponse::default())
}
