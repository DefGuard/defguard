use crate::{
    appstate::AppState,
    auth::SessionInfo,
    enterprise::db::{
        openid::{AuthorizedApp, NewOpenIDClient},
        OAuth2Client,
    },
    handlers::{webhooks::ChangeStateData, ApiResponse, ApiResult},
};
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
};

#[post("/", format = "json", data = "<data>")]
pub async fn add_openid_client(
    _session: SessionInfo,
    appstate: &State<AppState>,
    data: Json<NewOpenIDClient>,
) -> ApiResult {
    let client: OAuth2Client = data.into_inner().into();
    client.save(&appstate.pool).await?;
    Ok(ApiResponse {
        json: json!(client),
        status: Status::Created,
    })
}

// #[get("/", format = "json")]
// pub async fn list_openid_clients(_session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
//     debug!("Listing OpenID clients");
//     let openid_clients = OAuth2Client::all(&appstate.pool).await?;
//     Ok(ApiResponse {
//         json: json!(openid_clients),
//         status: Status::Ok,
//     })
// }

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
    _session: SessionInfo,
    appstate: &State<AppState>,
    client_id: &str,
    data: Json<NewOpenIDClient>,
) -> ApiResult {
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, client_id).await? {
        Some(mut openid_client) => {
            let data = data.into_inner();
            openid_client.name = data.name;
            openid_client.redirect_uri = data.redirect_uri;
            openid_client.enabled = data.enabled;
            openid_client.save(&appstate.pool).await?;
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
    _session: SessionInfo,
    appstate: &State<AppState>,
    client_id: &str,
    data: Json<ChangeStateData>,
) -> ApiResult {
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, client_id).await? {
        Some(mut openid_client) => {
            openid_client.enabled = data.enabled;
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
    _session: SessionInfo,
    appstate: &State<AppState>,
    client_id: &str,
) -> ApiResult {
    let status = match OAuth2Client::find_by_client_id(&appstate.pool, client_id).await? {
        Some(openid_client) => {
            openid_client.delete(&appstate.pool).await?;
            Status::Ok
        }
        None => Status::NotFound,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[get("/apps/<username>")]
pub async fn get_user_apps(
    session_info: SessionInfo,
    appstate: &State<AppState>,
    username: &str, // FIXME: unused
) -> ApiResult {
    let apps = AuthorizedApp::all_for_user(&appstate.pool, &session_info.user).await?;
    Ok(ApiResponse {
        json: json!(apps),
        status: Status::Ok,
    })
}

#[put("/apps/<id>", format = "json", data = "<data>")]
pub async fn update_user_app(
    _session: SessionInfo,
    appstate: &State<AppState>,
    id: i64,
    data: Json<AuthorizedApp>,
) -> ApiResult {
    let status = match AuthorizedApp::find_by_id(&appstate.pool, id).await? {
        Some(mut app) => {
            let update = data.into_inner();
            app.client_id = update.client_id;
            app.home_url = update.home_url;
            app.date = update.date;
            app.name = update.name;
            app.save(&appstate.pool).await?;
            Status::Ok
        }
        None => Status::NotFound,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}

#[delete("/apps/<id>")]
pub async fn delete_user_app(
    _session: SessionInfo,
    appstate: &State<AppState>,
    id: i64,
) -> ApiResult {
    debug!("Removing authorized app with id: {}", id);
    let status = match AuthorizedApp::find_by_id(&appstate.pool, id).await? {
        Some(app) => {
            app.delete(&appstate.pool).await?;
            Status::Ok
        }
        None => Status::NotFound,
    };
    Ok(ApiResponse {
        json: json!({}),
        status,
    })
}
