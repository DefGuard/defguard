use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::{ApiResponse, ApiResult, WebError};
use crate::{
    appstate::AppState,
    auth::{SessionInfo, VpnRole},
    db::{models::gateway::Gateway, Id, WireguardNetwork},
};

#[derive(Serialize, Deserialize)]
pub struct AddGatewayData {
    pub url: String,
}

pub async fn add_gateway(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(network_id): Path<Id>,
    Json(data): Json<AddGatewayData>,
) -> ApiResult {
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Network with id {} not found while adding a new gateway, aborting",
                network_id
            ))
        })?;

    debug!(
        "User {} is adding a new gateway with url {} to network {}",
        session.user.username, data.url, network.name
    );

    let gateway = Gateway::new(network_id, &data.url);

    gateway.save(&appstate.pool).await?;

    info!(
        "User {} has added a new gateway with url {} to network {}",
        session.user.username, data.url, network.name
    );

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::CREATED,
    })
}

pub async fn delete_gateway(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(gateway_id): Path<i64>,
) -> ApiResult {
    debug!(
        "User {} is removing gateway with id {}",
        session.user.username, gateway_id
    );
    let gateway = Gateway::find_by_id(&appstate.pool, gateway_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Gateway with id {} not found while removing gateway, aborting",
                gateway_id
            ))
        })?;
    debug!(
        "The gateway with id {} which is being removed by user {} has url {}",
        gateway_id, session.user.username, gateway.url
    );

    let msg = format!(
        "User {} has removed gateway with url {}",
        session.user.username, gateway.url
    );

    gateway.delete(&appstate.pool).await?;

    info!("{msg}");

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

#[derive(Serialize, Deserialize)]
pub struct GetGatewaysData {
    pub network_id: Id,
}

pub async fn get_gateways(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(network_id): Path<i64>,
) -> ApiResult {
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Network with id {} not found while getting gateways, aborting",
                network_id
            ))
        })?;

    debug!(
        "User {} is getting gateways for network {}",
        session.user.username, network.name
    );

    let gateways = Gateway::find_by_network_id(&appstate.pool, network_id).await?;

    Ok(ApiResponse {
        json: json!({ "gateways": gateways }),
        status: StatusCode::OK,
    })
}

#[derive(Serialize, Deserialize)]
pub struct GetGatewayData {
    pub gateway_id: Id,
}

pub async fn get_gateway(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(gateway_id): Path<i64>,
) -> ApiResult {
    debug!(
        "User {} is getting gateway with id {}",
        session.user.username, gateway_id
    );
    let gateway = Gateway::find_by_id(&appstate.pool, gateway_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Gateway with id {} not found while getting gateway, aborting",
                gateway_id
            ))
        })?;

    Ok(ApiResponse {
        json: json!({ "gateway": gateway }),
        status: StatusCode::OK,
    })
}

#[derive(Serialize, Deserialize)]
pub struct UpdateGatewayData {
    pub url: String,
}

pub async fn update_gateway(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(gateway_id): Path<i64>,
    Json(data): Json<UpdateGatewayData>,
) -> ApiResult {
    debug!(
        "User {} is updating gateway with id {}",
        session.user.username, gateway_id
    );
    let mut gateway = Gateway::find_by_id(&appstate.pool, gateway_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Gateway with id {} not found while updating gateway, aborting",
                gateway_id
            ))
        })?;

    debug!(
        "The gateway with id {} which is being updated by user {} has url {}",
        gateway_id, session.user.username, gateway.url
    );

    gateway.save(&appstate.pool).await?;

    info!(
        "User {} has updated gateway with id {} to have url {}",
        session.user.username, gateway_id, data.url
    );

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}
