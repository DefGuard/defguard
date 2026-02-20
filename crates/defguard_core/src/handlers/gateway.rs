use axum::{
    Json,
    extract::{Path, State},
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
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::{ApiResponse, ApiResult},
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
    pub certificate: Option<String>,
    pub certificate_expiry: Option<NaiveDateTime>,
    pub version: Option<String>,
    pub modified_at: NaiveDateTime,
    pub modified_by: Id,
    pub modified_by_firstname: String,
    pub modified_by_lastname: String,
    pub location_name: String,
}

impl GatewayInfo {
    pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<Self>> {
        query_as!(
            Self,
            "SELECT gateway.*, \
                u.first_name modified_by_firstname, \
                u.last_name modified_by_lastname, \
                CASE \
                    WHEN gateway.connected_at IS NULL THEN false \
                    WHEN gateway.disconnected_at IS NULL THEN true \
                    WHEN gateway.connected_at >= gateway.disconnected_at THEN true \
                    ELSE false \
                END AS \"connected!\", \
                wn.name AS location_name \
            FROM gateway \
            JOIN \"user\" u on gateway.modified_by = u.id \
            JOIN wireguard_network wn ON gateway.location_id = wn.id",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_location_id(pool: &PgPool, location_id: Id) -> sqlx::Result<Vec<Self>> {
        query_as!(
            Self,
            "SELECT gateway.*, \
                u.first_name modified_by_firstname, \
                u.last_name modified_by_lastname, \
                CASE \
                    WHEN gateway.connected_at IS NULL THEN false \
                    WHEN gateway.disconnected_at IS NULL THEN true \
                    WHEN gateway.connected_at >= gateway.disconnected_at THEN true \
                    ELSE false \
                END AS \"connected!\", \
                wn.name AS location_name \
            FROM gateway JOIN \"user\" u on gateway.modified_by = u.id \
            JOIN wireguard_network wn ON gateway.location_id = wn.id \
            WHERE location_id = $1",
            location_id
        )
        .fetch_all(pool)
        .await
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct GatewayUpdateData {
    pub name: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/gateway",
    responses(
        (status = 200, description = "Gateway list", body = [GatewayInfo]),
        (status = 401, description = "Unauthorized to get gateway list.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to get gateway list.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 500, description = "Unable to get gateway list.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn gateway_list(
    _role: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("User {} displaying gateway list", session.user.username);
    let gateways = GatewayInfo::list(&appstate.pool).await?;
    info!("User {} displayed gateway list", session.user.username);

    Ok(ApiResponse::json(gateways, StatusCode::OK))
}

#[utoipa::path(
    get,
    path = "/api/v1/gateway/{gateway_id}",
    responses(
        (status = 200, description = "Gateway details", body = GatewayInfo),
        (status = 401, description = "Unauthorized to get gateway details.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to get gateway details.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Gateway not found", body = ApiResponse, example = json!({"msg": "gateway not found"})),
        (status = 500, description = "Unable to get gateway details.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn gateway_details(
    Path(gateway_id): Path<i64>,
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

#[utoipa::path(
    put,
    path = "/api/v1/gateway/{gateway_id}",
    request_body = GatewayUpdateData,
    responses(
        (status = 200, description = "Successfully modified gateway.", body = GatewayInfo),
        (status = 401, description = "Unauthorized to modify gateway.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to modify a gateway.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Gateway not found", body = ApiResponse, example = json!({"msg": "gateway not found"})),
        (status = 500, description = "Unable to modify gateway.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn update_gateway(
    _role: AdminRole,
    Path(gateway_id): Path<i64>,
    State(appstate): State<AppState>,
    session: SessionInfo,
    context: ApiRequestContext,
    Json(data): Json<GatewayUpdateData>,
) -> ApiResult {
    debug!(
        "User {} updating gateway {gateway_id}",
        session.user.username
    );
    let gateway = Gateway::find_by_id(&appstate.pool, gateway_id).await?;

    let Some(mut gateway) = gateway else {
        warn!("Gateway {gateway_id} not found");
        return Ok(ApiResponse::json(Value::Null, StatusCode::NOT_FOUND));
    };
    let before = gateway.clone();

    gateway.name = data.name;
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

#[utoipa::path(
    delete,
    path = "/api/v1/gateway/{gateway_id}",
    responses(
        (status = 200, description = "Successfully deleted gateway.", body = ApiResponse),
        (status = 401, description = "Unauthorized to delete gateway.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission delete a gateway.", body = ApiResponse, example = json!({"msg": "access denied"})),
        (status = 404, description = "Gateway not found", body = ApiResponse, example = json!({"msg": "gateway not found"})),
        (status = 500, description = "Unable to delete gateway.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_gateway(
    _role: AdminRole,
    Path(gateway_id): Path<i64>,
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
        warn!("Gateway {gateway_id} not found");
        return Ok(ApiResponse::json(Value::Null, StatusCode::NOT_FOUND));
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
