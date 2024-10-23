use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::{ApiResponse, WebError};
use crate::{
    appstate::AppState,
    auth::{SessionInfo, VpnRole},
    db::{
        models::{gateway::Gateway, wireguard::WireguardNetwork},
        Id,
    },
};

#[derive(Deserialize, Serialize)]
pub(crate) struct GatewayData {
    url: String,
}

pub(crate) async fn add_gateway(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(network_id): Path<Id>,
    Json(data): Json<GatewayData>,
) -> Result<ApiResponse, WebError> {
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Network ID {network_id} not found while adding a gateway, aborting"
            ))
        })?;

    debug!(
        "User {} is adding a gateway with URL {} to network {}",
        session.user.username, data.url, network.name
    );

    let gateway = Gateway::new(network_id, &data.url);

    gateway.save(&appstate.pool).await?;

    info!(
        "User {} has added a gateway with URL {} to network {}",
        session.user.username, data.url, network.name
    );

    Ok(ApiResponse::new(json!({}), StatusCode::CREATED))
}

pub(crate) async fn delete_gateway(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(gateway_id): Path<Id>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} is removing gateway ID {gateway_id}",
        session.user.username
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
        "User {} has removed gateway with URL {}",
        session.user.username, gateway.url
    );

    gateway.delete(&appstate.pool).await?;

    info!("{msg}");

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

#[derive(Deserialize, Serialize)]
struct GetGatewaysData {
    network_id: Id,
}

pub(crate) async fn get_gateways(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(network_id): Path<Id>,
) -> Result<ApiResponse, WebError> {
    let network = WireguardNetwork::find_by_id(&appstate.pool, network_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Network ID {} not found while getting gateways, aborting",
                network_id
            ))
        })?;

    debug!(
        "User {} is getting gateways for network {}",
        session.user.username, network.name
    );

    let gateways = Gateway::find_by_network_id(&appstate.pool, network_id).await?;

    Ok(ApiResponse::new(
        json!({ "gateways": gateways }),
        StatusCode::OK,
    ))
}

#[derive(Deserialize, Serialize)]
struct GetGatewayData {
    gateway_id: Id,
}

pub(crate) async fn get_gateway(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(gateway_id): Path<i64>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} is getting gateway ID {gateway_id}",
        session.user.username
    );
    let gateway = Gateway::find_by_id(&appstate.pool, gateway_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!("Gateway ID {gateway_id} not found, aborting"))
        })?;

    Ok(ApiResponse::new(
        json!({ "gateway": gateway }),
        StatusCode::OK,
    ))
}

pub(crate) async fn update_gateway(
    _role: VpnRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(gateway_id): Path<Id>,
    Json(data): Json<GatewayData>,
) -> Result<ApiResponse, WebError> {
    debug!(
        "User {} is updating gateway ID {gateway_id}",
        session.user.username
    );
    let mut gateway = Gateway::find_by_id(&appstate.pool, gateway_id)
        .await?
        .ok_or_else(|| {
            WebError::ObjectNotFound(format!(
                "Gateway ID {gateway_id} not found while updating gateway, aborting"
            ))
        })?;

    debug!(
        "Updating gateway ID {gateway_id} by user {} has URL {}",
        session.user.username, gateway.url
    );

    let msg = format!(
        "User {} has updated gateway ID {} to have URL {}",
        session.user.username, gateway_id, data.url
    );

    gateway.url = data.url;
    gateway.save(&appstate.pool).await?;

    info!("{msg}");

    Ok(ApiResponse::new(json!({}), StatusCode::OK))
}
