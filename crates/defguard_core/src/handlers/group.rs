use std::collections::{HashMap, HashSet};

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
};
use defguard_common::db::{
    Id,
    models::{
        User,
        group::{Group, Permission},
    },
};
use sqlx::query_as;
use utoipa::ToSchema;

use super::{ApiErrorResponse, ApiResponse, ApiResult, EditGroupInfo, GroupInfo, Username};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::ldap::utils::{
        ldap_add_user_to_groups, ldap_add_users_to_groups, ldap_delete_group, ldap_modify_group,
        ldap_remove_user_from_groups, ldap_remove_users_from_groups, ldap_update_user_state,
        ldap_update_users_state,
    },
    error::WebError,
    events::{ApiEvent, ApiEventType, ApiRequestContext},
    handlers::pagination::{PaginatedApiResponse, PaginatedApiResult, PaginationParams},
    hashset,
    location_management::sync_all_networks,
};

#[derive(Deserialize, ToSchema)]
pub(crate) struct BulkAssignToGroupsRequest {
    // groups by name
    groups: Vec<String>,
    // users by id
    users: Vec<Id>,
}

/// Assign multiple users to multiple groups
#[utoipa::path(
    post,
    path = "/api/v1/groups-assign",
    tag = "group",
    request_body(content = BulkAssignToGroupsRequest, example = json!({"groups": ["admin", "developers"], "users": [1, 4, 6, 23, 35]})),
    responses(
        (status = 200, description = "Users assigned to the groups."),
        (status = 400, description = "The request contains unknown users or groups.", body = ApiErrorResponse, example = json!({"msg": "Request contained users that doesn't exists in db."})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to assign users.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn bulk_assign_to_groups(
    _role: AdminRole,
    State(appstate): State<AppState>,
    context: ApiRequestContext,
    Json(data): Json<BulkAssignToGroupsRequest>,
) -> Result<ApiResponse, WebError> {
    debug!("Assigning groups to users.");
    let mut users = query_as!(
        User,
        "SELECT id, username, password_hash, last_name, first_name, email, phone, mfa_enabled, \
        totp_enabled, email_mfa_enabled, totp_secret, email_mfa_secret, \
        mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
        from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, ldap_remote_enrollment_completed, enrollment_pending \
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

    let mut ldap_user_groups: HashMap<&User<Id>, HashSet<&str>> = HashMap::new();
    let mut transaction = appstate.pool.begin().await?;
    for group in &groups {
        for user in &users {
            user.add_to_group(&mut *transaction, group).await?;
            ldap_user_groups
                .entry(user)
                .or_default()
                .insert(&group.name);
        }
    }

    sync_all_networks(&mut transaction, &appstate.wireguard_tx).await?;

    transaction.commit().await?;

    ldap_add_users_to_groups(ldap_user_groups, &appstate.pool).await;

    let users_to_maybe_update = users.iter_mut().collect::<Vec<_>>();
    Box::pin(ldap_update_users_state(
        users_to_maybe_update,
        &appstate.pool,
        &appstate.wireguard_tx,
    ))
    .await;

    info!("Assigned {} groups to {} users.", groups.len(), users.len());
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::GroupsBulkAssigned { users, groups }),
    })?;

    Ok(ApiResponse::with_status(StatusCode::OK))
}

/// List groups with their details
#[utoipa::path(
    get,
    path = "/api/v1/group-info",
    tag = "group",
    responses(
        (status = 200, description = "All groups with their members.", body = [GroupInfo], example = json!([
            {
                "id": 1,
                "name": "name",
                "members": ["user"],
                "vpn_locations": ["location"],
                "is_admin": false
            }
        ])),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list groups.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_groups_info(
    _role: AdminRole,
    State(appstate): State<AppState>,
) -> ApiResult {
    debug!("Listing groups info");
    let q_result = query_as!(
        GroupInfo,
        "SELECT g.id, g.name, \
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
    Ok(ApiResponse::json(q_result, StatusCode::OK))
}

/// List group names
///
/// Returns group names only. Use `GET /api/v1/group-info` for full details, including
/// members and locations.
#[utoipa::path(
    get,
    path = "/api/v1/group",
    tag = "group",
    params(
        ("page" = Option<u32>, Query, description = "Page number. Defaults to 1."),
        ("per_page" = Option<u32>, Query, description = "Number of items per page, from 1 to 100. Defaults to 50.")
    ),
    responses(
        (status = 200, description = "Paginated list of group names.", body = PaginatedApiResponse<String>, example = json!({"data": ["admin"], "pagination": {"current_page": 1, "page_size": 50, "total_items": 1, "total_pages": 1, "next_page": null}})),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 500, description = "Unable to list groups.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn list_groups(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    pagination: Query<PaginationParams>,
) -> PaginatedApiResult<String> {
    let pagination = pagination.0;

    debug!("User {} is listing groups", &session.user.username);

    let groups = Group::all_paginated(
        &appstate.pool,
        i64::from(pagination.per_page()),
        i64::from(pagination.offset()),
    )
    .await?
    .into_iter()
    .map(|group| group.name)
    .collect();

    info!("User {} listed groups", &session.user.username);

    let count = Group::count(&appstate.pool).await?;
    Ok(PaginatedApiResponse::new(groups, pagination, count as u32))
}

/// Get a group
#[utoipa::path(
    get,
    path = "/api/v1/group/{id}",
    tag = "group",
    params(
        ("id" = i64, description = "ID of the group.")
    ),
    responses(
        (status = 200, description = "Group details.", body = GroupInfo, example = json!(
            {
                "id": 1,
                "name": "name",
                "members": ["user"],
                "vpn_locations": ["location"],
                "is_admin": false
            }
        )),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Group not found.", body = ApiErrorResponse, example = json!({"msg": "Group <id> not found"})),
        (status = 500, description = "Unable to get group.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn get_group(
    _admin: AdminRole,
    _session: SessionInfo,
    State(appstate): State<AppState>,
    Path(id): Path<Id>,
) -> ApiResult {
    debug!("Retrieving group {id}");
    if let Some(group) = Group::find_by_id(&appstate.pool, id).await? {
        let members = group.member_usernames(&appstate.pool).await?;
        let vpn_locations = group.allowed_vpn_locations(&appstate.pool).await?;
        let is_admin = group
            .has_permission(&appstate.pool, Permission::IsAdmin)
            .await?;
        info!("Retrieved group {id}");
        Ok(ApiResponse::json(
            GroupInfo::new(group.id, group.name, members, vpn_locations, is_admin),
            StatusCode::OK,
        ))
    } else {
        let msg = format!("Group {id} not found");
        error!(msg);
        Err(WebError::ObjectNotFound(msg))
    }
}

/// Create a group
///
/// Set `is_admin` to grant admin privileges to the group's members.
#[utoipa::path(
    post,
    path = "/api/v1/group",
    tag = "group",
    request_body(content = EditGroupInfo, example = json!({"name": "engineering", "members": ["jdoe", "asmith"], "is_admin": false})),
    responses(
        (status = 201, description = "Group created.", body = EditGroupInfo, example = json!(
            {
                "name": "name",
                "members": ["user"],
                "is_admin": false
            }
        )),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User not found.", body = ApiErrorResponse, example = json!({"msg": "Failed to find user <username>"})),
        (status = 500, description = "Unable to create group.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn create_group(
    _role: AdminRole,
    State(appstate): State<AppState>,
    context: ApiRequestContext,
    Json(group_info): Json<EditGroupInfo>,
) -> ApiResult {
    debug!("Creating group {}", group_info.name);

    let mut ldap_user_groups: HashMap<&User<Id>, HashSet<&str>> = HashMap::new();
    let mut transaction = appstate.pool.begin().await?;

    // FIXME: conflicts must not return internal server error (500).
    let group = Group::new(&group_info.name).save(&appstate.pool).await?;
    group
        .set_permission(&mut *transaction, Permission::IsAdmin, group_info.is_admin)
        .await?;

    let mut members = Vec::new();
    for member_username in &group_info.members {
        if let Some(user) = User::find_by_username(&mut *transaction, member_username).await? {
            members.push(user);
        } else {
            let msg = format!("Failed to find user {member_username}");
            error!(msg);
            return Err(WebError::ObjectNotFound(msg));
        }
    }

    for user in &members {
        user.add_to_group(&mut *transaction, &group).await?;
        ldap_user_groups
            .entry(user)
            .or_default()
            .insert(&group_info.name);
    }

    sync_all_networks(&mut transaction, &appstate.wireguard_tx).await?;

    transaction.commit().await?;

    if !ldap_user_groups.is_empty() {
        ldap_add_users_to_groups(ldap_user_groups, &appstate.pool).await;
        let users_to_maybe_update = members.iter_mut().collect::<Vec<_>>();
        Box::pin(ldap_update_users_state(
            users_to_maybe_update,
            &appstate.pool,
            &appstate.wireguard_tx,
        ))
        .await;
    }

    info!("Created group {}", group_info.name);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::GroupAdded { group }),
    })?;

    Ok(ApiResponse::json(group_info, StatusCode::CREATED))
}

/// Update a group
///
/// Renames the group and replaces its members. Set `is_admin` to grant admin privileges
/// to the group's members.
#[utoipa::path(
    put,
    path = "/api/v1/group/{id}",
    tag = "group",
    params(
        ("id" = i64, description = "ID of the group.")
    ),
    request_body = EditGroupInfo,
    responses(
        (status = 200, description = "Group updated."),
        (status = 400, description = "Cannot remove admin permissions from the last admin group."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User or group not found.", body = ApiErrorResponse, example = json!({"msg": "Group <id> not found"})),
        (status = 500, description = "Unable to update group.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn modify_group(
    _role: AdminRole,
    State(appstate): State<AppState>,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    Json(group_info): Json<EditGroupInfo>,
) -> ApiResult {
    debug!("Modifying group {id}");
    let Some(mut group) = Group::find_by_id(&appstate.pool, id).await? else {
        let msg = format!("Group {id} not found");
        error!(msg);
        return Err(WebError::ObjectNotFound(msg));
    };
    // store group before modifications
    let before = group.clone();

    let mut add_to_ldap_groups: HashMap<&User<Id>, HashSet<&str>> = HashMap::new();
    let mut remove_from_ldap_groups: HashMap<&User<Id>, HashSet<&str>> = HashMap::new();
    let mut transaction = appstate.pool.begin().await?;

    // Rename only when needed.
    //
    if group.name != group_info.name {
        group.name.clone_from(&group_info.name);
        group.save(&mut *transaction).await?;
    }

    if group.is_admin != group_info.is_admin && !group_info.is_admin {
        // prevent removing admin permissions from the last admin group
        let admin_groups_count = Group::find_by_permission(&appstate.pool, Permission::IsAdmin)
            .await?
            .len();
        if admin_groups_count == 1 {
            error!(
                "Can't remove admin permissions from the last admin group: {}",
                group.name
            );
            return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
        }
    }

    group
        .set_permission(&mut *transaction, Permission::IsAdmin, group_info.is_admin)
        .await?;

    // Modify group members.
    let mut current_members = group.members(&mut *transaction).await?;
    let users_before = current_members.clone();
    let mut members = Vec::new();
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
            members.push(user);
        }
    }

    for user in &members {
        user.add_to_group(&mut *transaction, &group).await?;
        add_to_ldap_groups
            .entry(user)
            .or_default()
            .insert(group.name.as_str());
    }

    // Remove outstanding members.
    for user in &current_members {
        user.remove_from_group(&mut *transaction, &group).await?;
        remove_from_ldap_groups
            .entry(user)
            .or_default()
            .insert(group.name.as_str());
    }

    sync_all_networks(&mut transaction, &appstate.wireguard_tx).await?;
    let users_after = group.members(&mut *transaction).await?.clone();
    transaction.commit().await?;

    ldap_add_users_to_groups(add_to_ldap_groups, &appstate.pool).await;
    ldap_remove_users_from_groups(remove_from_ldap_groups, &appstate.pool).await;
    if before.name != group.name {
        ldap_modify_group(&before.name, &group, &appstate.pool).await;
    }

    let affected_users = members
        .iter_mut()
        .chain(current_members.iter_mut())
        .collect::<Vec<_>>();
    ldap_update_users_state(affected_users, &appstate.pool, &appstate.wireguard_tx).await;

    let set_users_before: HashSet<_> = users_before.into_iter().collect();
    let set_users_after: HashSet<_> = users_after.into_iter().collect();
    let added: Vec<_> = set_users_after
        .difference(&set_users_before)
        .cloned()
        .collect();
    let removed: Vec<_> = set_users_before
        .difference(&set_users_after)
        .cloned()
        .collect();

    if !(added.is_empty() && removed.is_empty()) {
        appstate.emit_event(ApiEvent {
            context: context.clone(),
            event: Box::new(ApiEventType::GroupMembersModified {
                group: group.clone(),
                added,
                removed,
            }),
        })?;
    }

    info!("Modified group {}", group.name);
    appstate.emit_event(ApiEvent {
        context,
        event: Box::new(ApiEventType::GroupModified {
            before,
            after: group,
        }),
    })?;
    Ok(ApiResponse::default())
}

/// Delete a group
///
/// Removes the group and the group memberships of its members.
#[utoipa::path(
    delete,
    path = "/api/v1/group/{id}",
    tag = "group",
    params(
        ("id" = i64, description = "ID of the group.")
    ),
    responses(
        (status = 200, description = "Group deleted."),
        (status = 400, description = "The admin group cannot be deleted."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "Group not found.", body = ApiErrorResponse, example = json!({"msg": "Failed to find group <id>"})),
        (status = 500, description = "Unable to delete group.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn delete_group(
    _admin: AdminRole,
    session: SessionInfo,
    State(appstate): State<AppState>,
    context: ApiRequestContext,
    Path(id): Path<Id>,
) -> ApiResult {
    debug!("User {} deletes group {id}", &session.user.username);
    if let Some(group) = Group::find_by_id(&appstate.pool, id).await? {
        // Prevent removing the last admin group
        if group.is_admin {
            let admin_group_count = Group::find_by_permission(&appstate.pool, Permission::IsAdmin)
                .await?
                .len();
            if admin_group_count == 1 {
                error!("Cannot delete the last admin group: {}", group.name);
                return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
            }
        }
        let blocked_locations = group
            .locations_with_one_allowed_group(&appstate.pool)
            .await?;
        if !blocked_locations.is_empty() {
            let msg = format!(
                "Cannot delete group {} because it is the only allowed group in locations: {}",
                group.name,
                blocked_locations.join(", ")
            );
            error!("{msg}");
            return Ok(ApiResponse::with_status(StatusCode::BAD_REQUEST));
        }
        group.clone().delete(&appstate.pool).await?;
        ldap_delete_group(&group.name, &appstate.pool).await;

        // sync allowed devices for all locations
        let mut conn = appstate.pool.acquire().await?;
        sync_all_networks(&mut conn, &appstate.wireguard_tx).await?;

        info!(
            "User {} deleted group {}",
            &session.user.username, group.name
        );
        appstate.emit_event(ApiEvent {
            context,
            event: Box::new(ApiEventType::GroupRemoved { group }),
        })?;
        Ok(ApiResponse::default())
    } else {
        let msg = format!("Failed to find group {id}");
        error!(msg);
        Err(WebError::ObjectNotFound(msg))
    }
}

/// Add a member to a group
#[utoipa::path(
    post,
    path = "/api/v1/group/{id}",
    tag = "group",
    params(
        ("id" = i64, description = "ID of the group.")
    ),
    request_body = Username,
    responses(
        (status = 200, description = "Member added to the group."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User or group not found.", body = ApiErrorResponse, example = json!({"msg": "Group <id> not found"})),
        (status = 500, description = "Unable to add group member.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn add_group_member(
    _role: AdminRole,
    State(appstate): State<AppState>,
    context: ApiRequestContext,
    Path(id): Path<Id>,
    Json(data): Json<Username>,
) -> ApiResult {
    if let Some(group) = Group::find_by_id(&appstate.pool, id).await? {
        if let Some(mut user) = User::find_by_username(&appstate.pool, &data.username).await? {
            debug!("Adding user: {} to group: {}", user.username, group.name);
            user.add_to_group(&appstate.pool, &group).await?;
            ldap_add_user_to_groups(&user, hashset![group.name.as_str()], &appstate.pool).await;
            ldap_update_user_state(&mut user, &appstate.pool, &appstate.wireguard_tx).await;
            let mut conn = appstate.pool.acquire().await?;
            sync_all_networks(&mut conn, &appstate.wireguard_tx).await?;
            info!("Added user: {} to group: {}", user.username, group.name);
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::GroupMemberAdded { group, user }),
            })?;
            Ok(ApiResponse::default())
        } else {
            error!("User not found {}", data.username);
            Err(WebError::ObjectNotFound(format!(
                "User {} not found",
                data.username
            )))
        }
    } else {
        let msg = format!("Group {id} not found");
        error!(msg);
        Err(WebError::ObjectNotFound(msg))
    }
}

/// Remove a member from a group
#[utoipa::path(
    delete,
    path = "/api/v1/group/{id}/user/{username}",
    tag = "group",
    params(
        ("id" = i64, description = "ID of the group."),
        ("username" = String, description = "Name of the user.")
    ),
    responses(
        (status = 200, description = "Member removed from the group."),
        (status = 401, description = "Session is missing or invalid.", body = ApiErrorResponse, example = json!({"msg": "Session is required"})),
        (status = 403, description = "Requires admin privileges.", body = ApiErrorResponse, example = json!({"msg": "requires privileged access"})),
        (status = 404, description = "User or group not found.", body = ApiErrorResponse, example = json!({"msg": "Group <id> not found"})),
        (status = 500, description = "Unable to remove group member.", body = ApiErrorResponse, example = json!({"msg": "Internal server error"}))
    ),
    security(
        ("cookie" = []),
        ("api_token" = [])
    )
)]
pub(crate) async fn remove_group_member(
    _role: AdminRole,
    State(appstate): State<AppState>,
    context: ApiRequestContext,
    Path((id, username)): Path<(i64, String)>,
) -> ApiResult {
    if let Some(group) = Group::find_by_id(&appstate.pool, id).await? {
        if let Some(user) = User::find_by_username(&appstate.pool, &username).await? {
            debug!(
                "Removing user: {} from group: {}",
                user.username, group.name
            );
            user.remove_from_group(&appstate.pool, &group).await?;
            ldap_remove_user_from_groups(&user, hashset![group.name.as_str()], &appstate.pool)
                .await;

            let mut conn = appstate.pool.acquire().await?;
            sync_all_networks(&mut conn, &appstate.wireguard_tx).await?;
            info!("Removed user: {} from group: {}", user.username, group.name);
            appstate.emit_event(ApiEvent {
                context,
                event: Box::new(ApiEventType::GroupMemberRemoved { group, user }),
            })?;
            Ok(ApiResponse::with_status(StatusCode::OK))
        } else {
            let msg = format!("User {username} not found");
            error!(msg);
            Err(WebError::ObjectNotFound(msg))
        }
    } else {
        error!("Group {id} not found");
        Err(WebError::ObjectNotFound(format!("Group {id} not found")))
    }
}
