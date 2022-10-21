use super::Username;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{Group, User},
    enterprise::ldap::utils::{ldap_add_user_to_group, ldap_remove_user_from_group},
    error::OriWebError,
    handlers::{ApiResponse, ApiResult},
};
use rocket::{
    http::Status,
    serde::json::{serde_json::json, Json},
    State,
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
        status: Status::Ok,
    })
}

#[get("/group/<name>", format = "json")]
pub async fn get_group(_session: SessionInfo, appstate: &State<AppState>, name: &str) -> ApiResult {
    debug!("Retrieving group {}", name);
    match Group::find_by_name(&appstate.pool, name).await? {
        Some(group) => {
            let members = group.member_usernames(&appstate.pool).await?;
            info!("Retrieved group {}", name);
            Ok(ApiResponse {
                json: json!(GroupInfo::new(name.into(), members)),
                status: Status::Ok,
            })
        }
        None => {
            error!("Group {} not found", name);
            Err(OriWebError::ObjectNotFound(format!(
                "Group {} not found",
                name
            )))
        }
    }
}

#[post("/group/<name>", format = "json", data = "<data>")]
pub async fn add_group_member(
    _admin: AdminRole,
    appstate: &State<AppState>,
    name: &str,
    data: Json<Username>,
) -> ApiResult {
    match Group::find_by_name(&appstate.pool, name).await? {
        Some(group) => match User::find_by_username(&appstate.pool, &data.username).await? {
            Some(user) => {
                debug!("Adding user: {} to group: {}", user.username, group.name);
                user.add_to_group(&appstate.pool, &group).await?;
                let _result =
                    ldap_add_user_to_group(&appstate.config, &user.username, &group.name).await;
                info!("Added user: {} to group: {}", user.username, group.name);
                Ok(ApiResponse::default())
            }
            None => {
                error!("User not found {}", data.username);
                Err(OriWebError::ObjectNotFound(format!(
                    "User {} not found",
                    data.username
                )))
            }
        },
        None => {
            error!("Group {} not found", name);
            Err(OriWebError::ObjectNotFound(format!(
                "Group {} not found",
                name
            )))
        }
    }
}

#[delete("/group/<name>/user/<username>")]
pub async fn remove_group_member(
    _admin: AdminRole,
    appstate: &State<AppState>,
    name: &str,
    username: &str,
) -> ApiResult {
    match Group::find_by_name(&appstate.pool, name).await? {
        Some(group) => match User::find_by_username(&appstate.pool, username).await? {
            Some(user) => {
                debug!(
                    "Removing user: {} from group: {}",
                    user.username, group.name
                );
                user.remove_from_group(&appstate.pool, &group).await?;
                let _result =
                    ldap_remove_user_from_group(&appstate.config, &user.username, &group.name)
                        .await;
                info!("Removed user: {} from group: {}", user.username, group.name);
                Ok(ApiResponse {
                    json: json!({}),
                    status: Status::Ok,
                })
            }
            None => {
                error!("User not found {}", username);
                Err(OriWebError::ObjectNotFound(format!(
                    "User {} not found",
                    username
                )))
            }
        },
        None => {
            error!("Group {} not found", name);
            Err(OriWebError::ObjectNotFound(format!(
                "Group {} not found",
                name
            )))
        }
    }
}
