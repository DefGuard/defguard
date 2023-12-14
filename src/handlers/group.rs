use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;

use super::{ApiResponse, Username};
use crate::{
    appstate::AppState,
    auth::{SessionInfo, UserAdminRole},
    db::{Group, User},
    error::WebError,
    ldap::utils::{ldap_add_user_to_group, ldap_remove_user_from_group},
};

#[derive(Serialize)]
pub(crate) struct Groups {
    groups: Vec<String>,
}

impl Groups {
    #[must_use]
    pub fn new(groups: Vec<String>) -> Self {
        Self { groups }
    }
}

#[derive(Deserialize, Serialize)]
pub(crate) struct GroupInfo {
    name: String,
    members: Vec<String>,
}

impl GroupInfo {
    #[must_use]
    pub fn new(name: String, members: Vec<String>) -> Self {
        Self { name, members }
    }
}

pub(crate) async fn list_groups(
    _session: SessionInfo,
    State(appstate): State<AppState>,
) -> Result<ApiResponse, WebError> {
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

/// GET: Retrieve group with `name`.
pub(crate) async fn get_group(
    _session: SessionInfo,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
) -> Result<ApiResponse, WebError> {
    debug!("Retrieving group {name}");
    if let Some(group) = Group::find_by_name(&appstate.pool, &name).await? {
        let members = group.member_usernames(&appstate.pool).await?;
        info!("Retrieved group {name}");
        Ok(ApiResponse {
            json: json!(GroupInfo::new(name, members)),
            status: StatusCode::OK,
        })
    } else {
        error!("Group {name} not found");
        Err(WebError::ObjectNotFound(format!("Group {name} not found",)))
    }
}

/// POST: Create group with a given name and member list.
pub(crate) async fn create_group(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
    Json(group_info): Json<GroupInfo>,
) -> Result<ApiResponse, WebError> {
    debug!("Creating group {}", group_info.name);

    let mut transaction = appstate.pool.begin().await?;

    let mut group = Group::new(&group_info.name);
    group.save(&appstate.pool).await?;

    for username in &group_info.members {
        let Some(user) = User::find_by_username(&mut *transaction, username).await? else {
            let msg = format!("Failed to find user {username}");
            error!(msg);
            return Err(WebError::ObjectNotFound(msg));
        };
        user.add_to_group(&mut *transaction, &group).await?;
        let _result = ldap_add_user_to_group(&mut *transaction, username, &group.name).await;
    }

    transaction.commit().await?;

    info!("Created group {}", group_info.name);
    Ok(ApiResponse {
        json: json!(&group_info),
        status: StatusCode::CREATED,
    })
}

/// POST: Find a group with `name` and add `username` as a member.
pub(crate) async fn add_group_member(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
    Json(data): Json<Username>,
) -> Result<ApiResponse, WebError> {
    if let Some(group) = Group::find_by_name(&appstate.pool, &name).await? {
        if let Some(user) = User::find_by_username(&appstate.pool, &data.username).await? {
            debug!("Adding user: {} to group: {}", user.username, group.name);
            user.add_to_group(&appstate.pool, &group).await?;
            let _result = ldap_add_user_to_group(&appstate.pool, &user.username, &group.name).await;
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
        Err(WebError::ObjectNotFound(format!("Group {name} not found")))
    }
}

pub(crate) async fn remove_group_member(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
    Path((name, username)): Path<(String, String)>,
) -> Result<ApiResponse, WebError> {
    if let Some(group) = Group::find_by_name(&appstate.pool, &name).await? {
        if let Some(user) = User::find_by_username(&appstate.pool, &username).await? {
            debug!(
                "Removing user: {} from group: {}",
                user.username, group.name
            );
            user.remove_from_group(&appstate.pool, &group).await?;
            let _result =
                ldap_remove_user_from_group(&appstate.pool, &user.username, &group.name).await;
            info!("Removed user: {} from group: {}", user.username, group.name);
            Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::OK,
            })
        } else {
            error!("User not found {username}");
            Err(WebError::ObjectNotFound(format!(
                "User {username} not found"
            )))
        }
    } else {
        error!("Group {name} not found");
        Err(WebError::ObjectNotFound(format!("Group {name} not found",)))
    }
}
