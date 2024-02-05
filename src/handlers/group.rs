use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;
use sqlx::query_as;

use super::{ApiResponse, GroupInfo, Username};
use crate::{
    appstate::AppState,
    auth::{SessionInfo, UserAdminRole},
    db::{Group, User},
    error::WebError,
    // ldap::utils::{ldap_add_user_to_group, ldap_modify_group, ldap_remove_user_from_group},
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

/// GET: Retrieve all groups info
pub(crate) async fn list_groups_info(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
) -> Result<ApiResponse, WebError> {
    debug!("Listing groups info");
    let q_result = query_as!(
        GroupInfo,
        "SELECT g.name as name, ARRAY_AGG(u.username) as members \
    FROM \"group\" g \
    JOIN \"group_user\" gu ON gu.group_id = g.id \
    JOIN \"user\" u ON u.id = gu.user_id \
    GROUP BY g.name"
    )
    .fetch_all(&appstate.pool)
    .await?;
    Ok(ApiResponse {
        json: json!(q_result),
        status: StatusCode::OK,
    })
}

/// GET: Retrieve all groups.
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
            json: json!(GroupInfo::new(name, Some(members))),
            status: StatusCode::OK,
        })
    } else {
        let msg = format!("Group {name} not found");
        error!(msg);
        Err(WebError::ObjectNotFound(msg))
    }
}

/// POST: Create group with a given name and member list.
pub(crate) async fn create_group(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
    Json(group_info): Json<GroupInfo>,
) -> Result<ApiResponse, WebError> {
    debug!("Creating group {}", group_info.name);

    // FIXME: LDAP operations are not reverted.
    let mut transaction = appstate.pool.begin().await?;

    let mut group = Group::new(&group_info.name);
    // FIXME: conflicts must not return internal server error (500).
    group.save(&appstate.pool).await?;
    // TODO: create group in LDAP

    if let Some(ref members) = group_info.members {
        for username in members {
            let Some(user) = User::find_by_username(&mut *transaction, username).await? else {
                let msg = format!("Failed to find user {username}");
                error!(msg);
                return Err(WebError::ObjectNotFound(msg));
            };
            user.add_to_group(&mut *transaction, &group).await?;
            // let _result = ldap_add_user_to_group(&mut *transaction, username, &group.name).await;
        }
    }

    transaction.commit().await?;

    info!("Created group {}", group_info.name);
    Ok(ApiResponse {
        json: json!(group_info),
        status: StatusCode::CREATED,
    })
}

/// PUT: Rename group and/or change group members.
pub(crate) async fn modify_group(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
    Json(group_info): Json<GroupInfo>,
) -> Result<ApiResponse, WebError> {
    debug!("Modifying group {}", group_info.name);
    let Some(mut group) = Group::find_by_name(&appstate.pool, &name).await? else {
        let msg = format!("Group {name} not found");
        error!(msg);
        return Err(WebError::ObjectNotFound(msg));
    };

    // FIXME: LDAP operations are not reverted.
    let mut transaction = appstate.pool.begin().await?;

    // Rename only when needed.
    if group.name != group_info.name {
        group.name = group_info.name;
        group.save(&mut *transaction).await?;
        // let _result = ldap_modify_group(&mut *transaction, &group.name, &group).await;
    }

    // Modify group members.
    if let Some(ref members) = group_info.members {
        let mut current_members = group.members(&mut *transaction).await?;
        for username in members {
            if let Some(index) = current_members
                .iter()
                .position(|gm| &gm.username == username)
            {
                // This member is already in the group.
                current_members.remove(index);
                continue;
            }

            // Add new members to the group.
            if let Some(user) = User::find_by_username(&mut *transaction, username).await? {
                user.add_to_group(&mut *transaction, &group).await?;
                // let _result =
                //     ldap_add_user_to_group(&mut *transaction, username, &group.name).await;
            }
        }

        // Remove outstanding members.
        for user in current_members {
            user.remove_from_group(&mut *transaction, &group).await?;
            // let _result =
            //     ldap_remove_user_from_group(&mut *transaction, &user.username, &group.name).await;
        }
    }

    transaction.commit().await?;

    info!("Modified group {}", group.name);
    Ok(ApiResponse::default())
}

/// DELETE: Remove group with `name`.
pub(crate) async fn delete_group(
    _session: SessionInfo,

    State(appstate): State<AppState>,
    Path(name): Path<String>,
) -> Result<ApiResponse, WebError> {
    debug!("Deleting group {name}");
    // Administrative group must not be removed.
    // Note: Group names are unique, so this condition should be sufficient.
    if name == appstate.config.admin_groupname {
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    if let Some(group) = Group::find_by_name(&appstate.pool, &name).await? {
        group.delete(&appstate.pool).await?;
        // TODO: delete group from LDAP

        info!("Deleted group {name}");
        Ok(ApiResponse::default())
    } else {
        let msg = format!("Failed to find group {name}");
        error!(msg);
        Err(WebError::ObjectNotFound(msg))
    }
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
            // let _result = ldap_add_user_to_group(&appstate.pool, &user.username, &group.name).await;
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
        let msg = format!("Group {name} not found");
        error!(msg);
        Err(WebError::ObjectNotFound(msg))
    }
}

/// DELETE: Remove `username` from group with `name`.
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
            // let _result =
            //     ldap_remove_user_from_group(&appstate.pool, &user.username, &group.name).await;
            info!("Removed user: {} from group: {}", user.username, group.name);
            Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::OK,
            })
        } else {
            let msg = format!("User {username} not found");
            error!(msg);
            Err(WebError::ObjectNotFound(msg))
        }
    } else {
        error!("Group {name} not found");
        Err(WebError::ObjectNotFound(format!("Group {name} not found",)))
    }
}
