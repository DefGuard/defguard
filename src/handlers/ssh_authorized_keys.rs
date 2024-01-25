use crate::{
    appstate::AppState,
    auth::SessionInfo,
    db::{models::authentication_key::AuthenticationKey, DbPool, Group, User},
    error::WebError,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use ssh_key::PublicKey;

use super::{ApiResponse, ApiResult};

static SSH_KEY_TYPE: &str = "SSH";
static GPG_KEY_TYPE: &str = "GPG";

/// Trim optional newline
fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}

async fn add_user_ssh_keys_to_list(pool: &DbPool, user: &User, ssh_keys: &mut Vec<String>) {
    if let Some(user_id) = user.id {
        let keys_result =
            AuthenticationKey::fetch_user_authentication_keys_by_type(pool, user_id, SSH_KEY_TYPE)
                .await;

        if let Ok(authentication_keys) = keys_result {
            let mut keys: Vec<String> = authentication_keys
                .into_iter()
                .map(|item| item.key)
                .collect();
            ssh_keys.append(&mut keys);
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct SshKeysRequestParams {
    username: Option<String>,
    group: Option<String>,
}

/// Fetch public SSH keys for user
///
/// Meant to be used with `AuthorizedKeysCommand` config option in `sshd`.
/// Should always return a response to partially mitigate user enumeration.
/// Optional query params `username` and `group` are used for filtering users.
/// If no params are specified an empty response is returned.
pub async fn get_authorized_keys(
    params: Query<SshKeysRequestParams>,
    State(appstate): State<AppState>,
) -> Result<String, WebError> {
    info!("Fetching public SSH keys for {:?}", params);
    let mut ssh_keys: Vec<String> = Vec::new();

    // TODO: should be obsolete once YubiKeys are moved to a new table
    let add_user_keys_to_list = |user: User, ssh_keys: &mut Vec<String>| {
        // add key to list if user has an assigned SSH key
        if let Some(mut key) = user.ssh_key {
            trim_newline(&mut key);
            ssh_keys.push(key);
        }
    };

    // check if group filter was specified
    match &params.group {
        Some(group_name) => {
            // fetch group
            if let Some(group) = Group::find_by_name(&appstate.pool, group_name).await? {
                // check if user filter was specified
                match &params.username {
                    Some(username) => {
                        debug!("Fetching SSH keys for user {username} in group {group_name}");
                        // fetch user
                        if let Some(user) = User::find_by_username(&appstate.pool, username).await?
                        {
                            // check if user belongs to specified group
                            let members = group.member_usernames(&appstate.pool).await?;
                            if members.contains(&user.username) {
                                add_user_keys_to_list(user.clone(), &mut ssh_keys);
                                add_user_ssh_keys_to_list(&appstate.pool, &user, &mut ssh_keys)
                                    .await;
                            } else {
                                debug!("User {username} is not a member of group {group_name}",);
                            }
                        } else {
                            debug!("Specified user does not exist");
                        }
                    }
                    None => {
                        debug!("Fetching SSH keys for all users in group {group_name}");
                        // fetch all users in group
                        let users = group.members(&appstate.pool).await?;
                        for user in users {
                            add_user_keys_to_list(user.clone(), &mut ssh_keys);
                            add_user_ssh_keys_to_list(&appstate.pool, &user, &mut ssh_keys).await;
                        }
                    }
                }
            } else {
                debug!("Specified group does not exist");
            }
        }
        None => {
            // check if user filter was specified
            if let Some(username) = &params.username {
                debug!("Fetching SSH keys for user {username}");
                // fetch user
                if let Some(user) = User::find_by_username(&appstate.pool, username).await? {
                    add_user_keys_to_list(user.clone(), &mut ssh_keys);
                    add_user_ssh_keys_to_list(&appstate.pool, &user, &mut ssh_keys).await;
                } else {
                    debug!("Specified user does not exist");
                }
            }
        }
    }

    // concatenate all keys into a response
    Ok(ssh_keys.join("\n"))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AddAuthenticationKeyData {
    pub key: String,
    pub name: String,
    pub key_type: String,
}

pub async fn add_authentication_key(
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<AddAuthenticationKeyData>,
) -> Result<(), WebError> {
    let user = session.user;

    info!(
        "Adding an authentication key {data:?} to user {}",
        user.email
    );

    if ![SSH_KEY_TYPE, GPG_KEY_TYPE].contains(&&data.key_type.as_str()) {
        return Err(WebError::BadRequest(
            "unsupported authentication key type".into(),
        ));
    }

    let public_key = data.key.parse::<PublicKey>();

    if data.key_type == "SSH" && public_key.is_err() {
        return Err(WebError::BadRequest("invalid key format".into()));
    }

    // TODO: verify GPG key

    let user_id = if let Some(user_id) = user.id {
        user_id
    } else {
        return Err(WebError::BadRequest("invalid user".into()));
    };

    let existing_key =
        AuthenticationKey::find_by_user(&appstate.pool, user_id, data.key.clone()).await?;

    if existing_key.is_some() {
        return Err(WebError::BadRequest("key already exists".into()));
    }

    let key = data.key.clone();

    AuthenticationKey::new(user_id, data.key, data.name, data.key_type)
        .save(&appstate.pool)
        .await?;

    info!("Added authentication key {key} to user {}", user.email);

    Ok(())
}

pub async fn fetch_authentication_keys(
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    let user = session.user;

    let user_id = if let Some(user_id) = user.id {
        user_id
    } else {
        return Err(WebError::BadRequest("invalid user".into()));
    };

    let authentication_keys =
        AuthenticationKey::fetch_user_authentication_keys(&appstate.pool, user_id).await?;

    Ok(ApiResponse {
        json: json!(authentication_keys),
        status: StatusCode::OK,
    })
}

pub async fn delete_authentication_key(
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<i64>,
) -> Result<(), WebError> {
    let user = session.user;

    info!(
        "Attempting to delete authentication key with ID of {id} by user {}",
        user.email
    );

    let user_id = if let Some(user_id) = user.id {
        user_id
    } else {
        return Err(WebError::BadRequest("invalid user".into()));
    };

    let exisiting_key = AuthenticationKey::find_by_id(&appstate.pool, id).await?;

    if let Some(authentication_key) = exisiting_key {
        // Check whether key belongs to authenticated user
        if authentication_key.user_id != user_id {
            return Err(WebError::Forbidden("access denied".into()));
        }

        let key = authentication_key.clone().key;
        authentication_key.delete(&appstate.pool).await?;
        info!("Authentication key {} deleted by {}", key, user.email);
    } else {
        return Err(WebError::ObjectNotFound(
            "authentication key not found".into(),
        ));
    }

    Ok(())
}
