use super::{ApiResponse, ApiResult, Username};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{Group, User},
    error::WebError,
    ldap::utils::{ldap_add_user_to_group, ldap_remove_user_from_group},
};

#[derive(Serialize)]
pub struct Groups {
    groups: Vec<String>,
}

impl Groups {
    #[must_use]
    pub fn new(groups: Vec<String>) -> Self {
        Self { groups }
    }
}

#[derive(Serialize)]
pub struct GroupInfo {
    name: String,
    members: Vec<String>,
}

impl GroupInfo {
    #[must_use]
    pub fn new(name: String, members: Vec<String>) -> Self {
        Self { name, members }
    }
}

#[get("/group", format = "json")]
pub async fn list_groups(_session: SessionInfo, appstate: &State<AppState>) -> ApiResult {
    debug!("Listing groups");
    let groups = Group::all(&appstate.pool)
        .await?
        .into_iter()
        .map(|group| group.name)
        .collect();
    info!("Listed groups");
    Ok(ApiResponse {
        json: json!(Groups::new(groups)),
        status: StatusCode::OK,
    })
}

#[get("/group/<name>", format = "json")]
pub async fn get_group(_session: SessionInfo, appstate: &State<AppState>, name: &str) -> ApiResult {
    debug!("Retrieving group {name}");
    if let Some(group) = Group::find_by_name(&appstate.pool, name).await? {
        let members = group.member_usernames(&appstate.pool).await?;
        info!("Retrieved group {name}");
        Ok(ApiResponse {
            json: json!(GroupInfo::new(name.into(), members)),
            status: StatusCode::OK,
        })
    } else {
        error!("Group {name} not found");
        Err(WebError::ObjectNotFound(format!(
            "Group {name} not found",
        )))
    }
}

#[post("/group/<name>", format = "json", data = "<data>")]
pub async fn add_group_member(
    _admin: AdminRole,
    appstate: &State<AppState>,
    name: &str,
    data: Json<Username>,
) -> ApiResult {
    if let Some(group) = Group::find_by_name(&appstate.pool, name).await? {
        if let Some(user) = User::find_by_username(&appstate.pool, &data.username).await? {
            debug!("Adding user: {} to group: {}", user.username, group.name);
            user.add_to_group(&appstate.pool, &group).await?;
            let _result =
                ldap_add_user_to_group(&appstate.config, &user.username, &group.name).await;
            info!("Added user: {} to group: {}", user.username, group.name);
            Ok(ApiResponse::default())
        } else {
            error!("User not found {}", data.username);
            Err(WebError::ObjectNotFound(format!(
                "User {} not found",
                data.username
            )))
        }
    } else {
        error!("Group {name} not found");
        Err(WebError::ObjectNotFound(format!(
            "Group {name} not found"
        )))
    }
}

#[delete("/group/<name>/user/<username>")]
pub async fn remove_group_member(
    _admin: AdminRole,
    appstate: &State<AppState>,
    name: &str,
    username: &str,
) -> ApiResult {
    if let Some(group) = Group::find_by_name(&appstate.pool, name).await? {
        if let Some(user) = User::find_by_username(&appstate.pool, username).await? {
            debug!(
                "Removing user: {} from group: {}",
                user.username, group.name
            );
            user.remove_from_group(&appstate.pool, &group).await?;
            let _result =
                ldap_remove_user_from_group(&appstate.config, &user.username, &group.name).await;
            info!("Removed user: {} from group: {}", user.username, group.name);
            Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::OK,
            })
        } else {
            error!("User not found {}", username);
            Err(WebError::ObjectNotFound(format!(
                "User {username} not found"
            )))
        }
    } else {
        error!("Group {name} not found");
        Err(WebError::ObjectNotFound(format!(
            "Group {name} not found",
        )))
    }
}
