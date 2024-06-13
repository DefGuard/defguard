use axum::{extract::State, http::StatusCode, Json};

use crate::{appstate::AppState, auth::AdminRole, db::models::openid_provider::OpenIdProvider};

use super::{ApiResponse, ApiResult};
use serde_json::json;

// pub async fn add_openid_provider(
//     _admin: AdminRole,
//     session: SessionInfo,
//     State(appstate): State<AppState>,
//     Json(data): Json<OpenIdProvider>,
// ) -> ApiResult {
//     debug!(
//         "User {} adding OpenID provider {}",
//         session.user.username, client.name
//     );
//     client.save(&appstate.pool).await?;
//     info!(
//         "User {} added OpenID client {}",
//         session.user.username, client.name
//     );
//     Ok(ApiResponse {
//         json: json!(client),
//         status: StatusCode::CREATED,
//     })
// }

pub async fn list_openid_providers(
    _admin: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    let providers = OpenIdProvider::all(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(providers),
        status: StatusCode::OK,
    })
}
