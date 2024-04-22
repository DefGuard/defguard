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
    db::{Group, User, WireguardNetwork},
    error::WebError,
    server_config,
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

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct BulkAssignToGroupsRequest {
    // groups by name
    groups: Vec<String>,
    // users by id
    users: Vec<i64>,
}

// POST bulk assign users to one group for users overview
// assign many users to many groups at once
pub(crate) async fn bulk_assign_to_groups(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
    Json(data): Json<BulkAssignToGroupsRequest>,
) -> Result<ApiResponse, WebError> {
    debug!("Assigning groups to users.");
    let users = query_as!(
        User,
        "SELECT id \"id?\", username, password_hash, last_name, first_name, email, \
            phone, mfa_enabled, totp_enabled, email_mfa_enabled, \
            totp_secret, email_mfa_secret, mfa_method \"mfa_method: _\", recovery_codes \
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

/// GET: Retrieve all groups info
pub(crate) async fn list_groups_info(
    _role: UserAdminRole,
    State(appstate): State<AppState>,
) -> Result<ApiResponse, WebError> {
    debug!("Listing groups info");
    let q_result = query_as!(
        GroupInfo,
        "SELECT g.name as name, \
        COALESCE(ARRAY_AGG(DISTINCT u.username) FILTER (WHERE u.username IS NOT NULL), '{}') as \"members!\", \
        COALESCE(ARRAY_AGG(DISTINCT wn.name) FILTER (WHERE wn.name IS NOT NULL), '{}') as \"vpn_locations!\" \
        FROM \"group\" g \
        LEFT JOIN \"group_user\" gu ON gu.group_id = g.id \
        LEFT JOIN \"user\" u ON u.id = gu.user_id \
        LEFT JOIN \"wireguard_network_allowed_group\" wnag ON wnag.group_id = g.id \
        LEFT JOIN \"wireguard_network\" wn ON wn.id = wnag.network_id \
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
        let vpn_locations = group.allowed_vpn_locations(&appstate.pool).await?;
        info!("Retrieved group {name}");
        Ok(ApiResponse {
            json: json!(GroupInfo::new(name, members, vpn_locations)),
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

    for username in &group_info.members {
        let Some(user) = User::find_by_username(&mut *transaction, username).await? else {
            let msg = format!("Failed to find user {username}");
            error!(msg);
            return Err(WebError::ObjectNotFound(msg));
        };
        user.add_to_group(&mut *transaction, &group).await?;
        // let _result = ldap_add_user_to_group(&mut *transaction, username, &group.name).await;
    }

    transaction.commit().await?;

    WireguardNetwork::sync_all_networks(&appstate).await?;

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

    transaction.commit().await?;

    WireguardNetwork::sync_all_networks(&appstate).await?;

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
    if name == server_config().admin_groupname {
        return Ok(ApiResponse {
            json: json!({}),
            status: StatusCode::BAD_REQUEST,
        });
    }

    if let Some(group) = Group::find_by_name(&appstate.pool, &name).await? {
        group.delete(&appstate.pool).await?;
        // TODO: delete group from LDAP

        WireguardNetwork::sync_all_networks(&appstate).await?;

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
