use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::models::{oauth2client::OAuth2Client, NewOpenIDClient},
    handlers::{webhooks::ChangeStateData, ApiResponse, ApiResult},
};
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};

#[post("/", format = "json", data = "<data>")]
pub async fn add_openid_client(
    session: SessionInfo,
    appstate: &State<AppState>,
    data: Json<NewOpenIDClient>,
) -> ApiResult {
    let mut client = OAuth2Client::from_new(data.into_inner());
    debug!(
        "User {} adding OpenID client {}",
        session.user.username, client.name
    );
    client.save(&appstate.pool).await?;
    info!(
        "User {} added OpenID client {}",
        session.user.username, client.name
    );
    Ok(ApiResponse {
        json: json!(client),
        status: Status::Created,
    })
}

#[get("/", format = "json")]
pub async fn list_openid_clients(_session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    let openid_clients = OAuth2Client::all(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(openid_clients),
        status: Status::Ok,
    })
}

#[get("/<client_id>", format = "json")]
pub async fn get_openid_client(
    _session: SessionInfo,
    appstate: &State<AppState>,
    client_id: &str,
) -> ApiResult {
    match OAuth2Client::find_by_client_id(&appstate.pool, client_id).await? {
        Some(openid_client) => Ok(ApiResponse {
            json: json!(openid_client),
            status: Status::Ok,
        }),
        None => Ok(ApiResponse {
            json: json!({}),
            status: Status::NotFound,
        }),
    }
}

#[put("/<client_id>", format = "json", data = "<data>")]
pub async fn change_openid_client(
    session: SessionInfo,
    appstate: &State<AppState>,
    client_id: &str,
    data: Json<NewOpenIDClient>,
) -> ApiResult {
    debug!(
        "User {} updating OpenID client {}",
        session.user.username, client_id
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, client_id).await? {
        Some(mut openid_client) => {
            let data = data.into_inner();
            openid_client.name = data.name;
            openid_client.redirect_uri = data.redirect_uri;
            openid_client.enabled = data.enabled;
            openid_client.scope = data.scope;
            openid_client.save(&appstate.pool).await?;
            info!(
                "User {} updated OpenID client {} ({})",
                session.user.username, client_id, openid_client.name
            );
            Status::Ok
        }
        None => Status::NotFound,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[post("/<client_id>", format = "json", data = "<data>")]
pub async fn change_openid_client_state(
    session: SessionInfo,
    appstate: &State<AppState>,
    client_id: &str,
    data: Json<ChangeStateData>,
) -> ApiResult {
    debug!(
        "User {} changin OpenID client {} enabled state",
        session.user.username, client_id
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, client_id).await? {
        Some(mut openid_client) => {
            openid_client.enabled = data.enabled;
            openid_client.save(&appstate.pool).await?;
            info!(
                "User {} changed OpenID client {} ({}) enabled state to {}",
                session.user.username, client_id, openid_client.name, openid_client.enabled,
            );
            Status::Ok
        }
        None => Status::NotFound,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[delete("/<client_id>")]
pub async fn delete_openid_client(
    session: SessionInfo,
    appstate: &State<AppState>,
    client_id: &str,
) -> ApiResult {
    debug!(
        "User {} deleting OpenID client {}",
        session.user.username, client_id
    );
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, client_id).await? {
        Some(openid_client) => {
            openid_client.delete(&appstate.pool).await?;
            info!(
                "User {} deleted OpenID client {}",
                session.user.username, client_id
            );
            Status::Ok
        }
        None => Status::NotFound,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}
