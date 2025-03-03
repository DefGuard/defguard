use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
};
use serde_json::json;
use sqlx::query_as;
use utoipa::ToSchema;

use super::{ApiResponse, EditGroupInfo, GroupInfo, Username};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{models::group::Permission, Group, User, WireguardNetwork},
    error::WebError,
};

#[derive(Serialize, ToSchema)]
pub(crate) struct Groups {
    groups: Vec<String>,
}

impl Groups {
    #[must_use]
    pub fn new(groups: Vec<String>) -> Self {
        Self { groups }
    }
}

#[derive(Deserialize, Debug, Clone, ToSchema)]
pub(crate) struct BulkAssignToGroupsRequest {
    // groups by name
    groups: Vec<String>,
    // users by id
    users: Vec<i64>,
}

/// Bulk assign users to groups
///
/// Assign many users to many groups at once.
///
/// # Returns
/// If error occurs, it returns `WebError` object.
#[utoipa::path(
    post,
    path = "/api/v1/groups-assign",
    responses(
        (status = 200, description = "Successfully assign users to groups."),
        (status = 400, description = "Bad request. Request contains users or groups that don't exist in db.", body = ApiResponse, example = json!({"msg": "Request contained users that doesn't exists in db."})),
        (status = 401, description = "Unauthorized to assign users to groups.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to assign users to groups.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Cannot assign users to groups.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
pub(crate) async fn bulk_assign_to_groups(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<BulkAssignToGroupsRequest>,
) -> Result<ApiResponse, WebError> {
    debug!("Assigning groups to users.");
    let users = query_as!(
        User,
        "SELECT id, username, password_hash, last_name, first_name, email, \
            phone, mfa_enabled, totp_enabled, email_mfa_enabled, \
            totp_secret, email_mfa_secret, mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
            FROM \"user\" WHERE id = ANY($1)",
        &data.users
    )
    .fetch_all(&appstate.pool)
    .await?;

    let groups = query_as!(
        Group,
        "SELECT * FROM \"group\" WHERE name = ANY($1)",
        &data.groups
    )
    .fetch_all(&appstate.pool)
    .await?;

    if users.len() != data.users.len() {
        return Err(WebError::BadRequest(
            "Request contained users that doesn't exists in db.".into(),
        ));
    }

    if groups.len() != data.groups.len() {
        return Err(WebError::BadRequest(
            "Request contained groups that doesn't exists in db.".into(),
        ));
    }

    let mut transaction = appstate.pool.begin().await?;
    for group in &groups {
        for user in &users {
            user.add_to_group(&mut *transaction, group).await?;
        }
    }
    transaction.commit().await?;
    WireguardNetwork::sync_all_networks(&appstate).await?;
    info!("Assigned {} groups to {} users.", groups.len(), users.len());

    Ok(ApiResponse {
        json: json!({}),
        status: StatusCode::OK,
    })
}

/// Retrieve all groups info
///
/// For each group, the endpoint retrieves a `GroupInfo` object containing: group name, a list of members' usernames and a list of vpn_location.
///
/// `There is another endpoint "/api/v1/group" that retrives only name of each groups if you don't want all information.`
///
/// # Returns
/// Returns a list of `GroupInfo` objects or `WebError` if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/group-info",
    responses(
        (status = 200, description = "Successfully listed groups info.", body = [GroupInfo], example = json!([
            {
                "name": "name",
                "members": ["user"],
                "vpn_locations": ["location"]
            }
        ])),
        (status = 401, description = "Unauthorized to list groups info.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 401, description = "Unauthorized to assign users to groups.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to list groups info.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Cannot list groups info.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
pub(crate) async fn list_groups_info(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> Result<ApiResponse, WebError> {
    debug!("Listing groups info");
    let q_result = query_as!(
        GroupInfo,
        "SELECT g.name, \
        COALESCE(ARRAY_AGG(DISTINCT u.username) FILTER (WHERE u.username IS NOT NULL), '{}') \"members!\", \
        COALESCE(ARRAY_AGG(DISTINCT wn.name) FILTER (WHERE wn.name IS NOT NULL), '{}') \"vpn_locations!\", \
        is_admin \
        FROM \"group\" g \
        LEFT JOIN \"group_user\" gu ON gu.group_id = g.id \
        LEFT JOIN \"user\" u ON u.id = gu.user_id \
        LEFT JOIN \"wireguard_network_allowed_group\" wnag ON wnag.group_id = g.id \
        LEFT JOIN \"wireguard_network\" wn ON wn.id = wnag.network_id \
        GROUP BY g.name, g.id"
    )
    .fetch_all(&appstate.pool)
    .await?;
    Ok(ApiResponse {
        json: json!(q_result),
        status: StatusCode::OK,
    })
}

/// Retrieve all groups.
///
/// # Returns
/// Returns a `Groups` object or `WebError` if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/group",
    responses(
        (status = 200, description = "Retrieve all groups.", body = Groups, example = json!({"groups": ["admin"]})),
        (status = 401, description = "Unauthorized to retrive all groups.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 500, description = "Cannot retrive all groups.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
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

/// Retrieve group with `name`.
///
/// # Returns
/// Returns a `GroupInfo` object or `WebError` if error occurs.
#[utoipa::path(
    get,
    path = "/api/v1/group/{name}",
    params(
        ("name" = String, description = "Group name")
    ),
    responses(
        (status = 200, description = "Retrieve a group.", body = GroupInfo, example = json!(
            {
                "name": "name",
                "members": ["user"],
                "vpn_locations": ["location"],
                "is_admin": false
            }
        )),
        (status = 401, description = "Unauthorized to retrive a group.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Incorrect name of the group.", body = ApiResponse, example = json!({"msg": "Group <name> not found"})),
        (status = 500, description = "Cannot retrive a group.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
pub(crate) async fn get_group(
    _session: SessionInfo,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
) -> Result<ApiResponse, WebError> {
    debug!("Retrieving group {name}");
    if let Some(group) = Group::find_by_name(&appstate.pool, &name).await? {
        let members = group.member_usernames(&appstate.pool).await?;
        let vpn_locations = group.allowed_vpn_locations(&appstate.pool).await?;
        let is_admin = group
            .has_permission(&appstate.pool, Permission::IsAdmin)
            .await?;
        info!("Retrieved group {name}");
        Ok(ApiResponse {
            json: json!(GroupInfo::new(name, members, vpn_locations, is_admin)),
            status: StatusCode::OK,
        })
    } else {
        let msg = format!("Group {name} not found");
        error!(msg);
        Err(WebError::ObjectNotFound(msg))
    }
}

/// Create group
///
/// Create group with a given name and member list.
///
/// # Returns
/// Returns a `GroupsInfo` object or `WebError` if error occurs.
#[utoipa::path(
    post,
    path = "/api/v1/group",
    request_body = EditGroupInfo,
    responses(
        (status = 201, description = "Successfully created a group and added users.", body = EditGroupInfo, example = json!(
            {
                "name": "name",
                "members": ["user"]
            }
        )),
        (status = 401, description = "Unauthorized to retrive a group.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to list groups info.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Cannot create group: user don't exist.", body = ApiResponse, example = json!({"msg": "Failed to find user <username>"})),
        (status = 500, description = "Cannot retrive a group.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
pub(crate) async fn create_group(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Json(group_info): Json<EditGroupInfo>,
) -> Result<ApiResponse, WebError> {
    debug!("Creating group {}", group_info.name);

    // FIXME: LDAP operations are not reverted.
    let mut transaction = appstate.pool.begin().await?;

    // FIXME: conflicts must not return internal server error (500).
    let group = Group::new(&group_info.name).save(&appstate.pool).await?;
    // TODO: create group in LDAP
    group
        .set_permission(&mut *transaction, Permission::IsAdmin, group_info.is_admin)
        .await?;
    for username in &group_info.members {
        let Some(user) = User::find_by_username(&mut *transaction, username).await? else {
            let msg = format!("Failed to find user {username}");
            error!(msg);
            return Err(WebError::ObjectNotFound(msg));
        };
        user.add_to_group(&mut *transaction, &group).await?;
        // TODO: update LDAP
    }

    transaction.commit().await?;

    WireguardNetwork::sync_all_networks(&appstate).await?;

    info!("Created group {}", group_info.name);

    Ok(ApiResponse {
        json: json!(group_info),
        status: StatusCode::CREATED,
    })
}

/// Modify group
///
/// Rename group and/or change group members.
///
/// # Returns
/// Returns a `GroupsInfo` object or `WebError` if error occurs.
#[utoipa::path(
    put,
    path = "/api/v1/group/{name}",
    request_body = EditGroupInfo,
    responses(
        (status = 201, description = "Successfully updated group."),
        (status = 401, description = "Unauthorized to update user group.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to update user group.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Cannot update group: user or group don't exist.", body = ApiResponse, example = json!({"msg": "Group <group_name> not found"})),
        (status = 500, description = "Cannot update a group.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
pub(crate) async fn modify_group(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
    Json(group_info): Json<EditGroupInfo>,
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
        // TODO: update LDAP
    }

    if group.is_admin != group_info.is_admin && !group_info.is_admin {
        // prevent removing admin permissions from the last admin group
        let admin_groups_count = Group::find_by_permission(&appstate.pool, Permission::IsAdmin)
            .await?
            .len();
        if admin_groups_count == 1 {
            error!(
                "Can't remove admin permissions from the last admin group: {}",
                name
            );
            return Ok(ApiResponse {
                json: json!({}),
                status: StatusCode::BAD_REQUEST,
            });
        }
    }

    group
        .set_permission(&mut *transaction, Permission::IsAdmin, group_info.is_admin)
        .await?;

    // Modify group members.
    let mut current_members = group.members(&mut *transaction).await?;
    for username in &group_info.members {
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
            // TODO: update LDAP
        }
    }

    // Remove outstanding members.
    for user in current_members {
        user.remove_from_group(&mut *transaction, &group).await?;
        // TODO: update LDAP
    }

    transaction.commit().await?;

    WireguardNetwork::sync_all_networks(&appstate).await?;

    info!("Modified group {}", group.name);
    Ok(ApiResponse::default())
}

/// Remove group with `name`.
///
/// Delete group and group members.
///
/// # Returns
/// If error occurs it returns `WebError` object.
#[utoipa::path(
    delete,
    path = "/api/v1/group/{name}",
    params(
        ("name" = String, description = "Group name")
    ),
    responses(
        (status = 200, description = "Successfully deleted a group."),
        (status = 400, description = "Cannot delete admin group.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to delete group.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 404, description = "Cannot delete group: user or group don't exist.", body = ApiResponse, example = json!({"msg": "Failed to find group <group_name>"})),
        (status = 500, description = "Cannot delete a group.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
pub(crate) async fn delete_group(
    _session: SessionInfo,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
) -> Result<ApiResponse, WebError> {
    debug!("Deleting group {name}");
    if let Some(group) = Group::find_by_name(&appstate.pool, &name).await? {
        // Prevent removing the last admin group
        if group.is_admin {
            let admin_group_count = Group::find_by_permission(&appstate.pool, Permission::IsAdmin)
                .await?
                .len();
            if admin_group_count == 1 {
                error!("Cannot delete the last admin group: {name}");
                return Ok(ApiResponse {
                    json: json!({}),
                    status: StatusCode::BAD_REQUEST,
                });
            }
        }
        group.delete(&appstate.pool).await?;
        // TODO: delete group from LDAP

        // sync allowed devices for all locations
        WireguardNetwork::sync_all_networks(&appstate).await?;

        info!("Deleted group {name}");
        Ok(ApiResponse::default())
    } else {
        let msg = format!("Failed to find group {name}");
        error!(msg);
        Err(WebError::ObjectNotFound(msg))
    }
}

/// Add a group member
///
/// Find a group with `name` and add `username` as a member.
///
/// # Returns
/// If error occurs it returns `WebError` object.
#[utoipa::path(
    post,
    path = "/api/v1/group/{name}",
    params(
        ("name" = String, description = "Group name")
    ),
    request_body = Username,
    responses(
        (status = 200, description = "Successfully add a new member to group."),
        (status = 401, description = "Unauthorized to add a new group member.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to add a new group member.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Cannot add a new group member: user or group don't exist.", body = ApiResponse, example = json!({"msg": "Failed to find group <group_name>"})),
        (status = 500, description = "Cannot add a new group memmber.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
pub(crate) async fn add_group_member(
    _role: AdminRole,
    State(appstate): State<AppState>,
    Path(name): Path<String>,
    Json(data): Json<Username>,
) -> Result<ApiResponse, WebError> {
    if let Some(group) = Group::find_by_name(&appstate.pool, &name).await? {
        if let Some(user) = User::find_by_username(&appstate.pool, &data.username).await? {
            debug!("Adding user: {} to group: {}", user.username, group.name);
            user.add_to_group(&appstate.pool, &group).await?;
            // let _result = ldap_add_user_to_group(&appstate.pool, &user.username, &group.name).await;
            WireguardNetwork::sync_all_networks(&appstate).await?;
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

/// Remove `username` from group with `name`.
///
/// Find a group with `name` and remove `username` as a member.
///
/// # Returns
/// If error occurs it returns `WebError` object.
#[utoipa::path(
    delete,
    path = "/api/v1/group/{name}/user/{username}",
    params(
        ("name" = String, description = "Name of the group that you want to delete a user."),
        ("username" = String, description = "Name of the user that you want to delete.")
    ),
    responses(
        (status = 200, description = "Successfully remove a member from group.", body = ApiResponse, example = json!({})),
        (status = 401, description = "Unauthorized to remove a group member.", body = ApiResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "You don't have permission to remove a group member.", body = ApiResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Cannot remove a  group member: user or group don't exist.", body = ApiResponse, example = json!({"msg": "Failed to find group <group_name>"})),
        (status = 500, description = "Cannot remove a group member.", body = ApiResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = []) 
    )
)]
pub(crate) async fn remove_group_member(
    _role: AdminRole,
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
            // TODO: update LDAP

            WireguardNetwork::sync_all_networks(&appstate).await?;
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
