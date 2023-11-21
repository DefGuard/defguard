use crate::{
    appstate::AppState,
    db::{Group, User},
    error::WebError,
};
use axum::extract::{Query, State};

/// Trim optional newline
fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
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
    let mut ssh_keys = Vec::new();

    let mut add_user_keys_to_list = |user: User| {
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
                                add_user_keys_to_list(user);
                            } else {
                                debug!("User {username} is not a member of group {group_name}",);
                            }
                        } else {
                            debug!("Specified user does not exist")
                        }
                    }
                    None => {
                        debug!("Fetching SSH keys for all users in group {group_name}");
                        // fetch all users in group
                        let users = group.fetch_all_members(&appstate.pool).await?;
                        for user in users {
                            add_user_keys_to_list(user)
                        }
                    }
                }
            } else {
                debug!("Specified group does not exist")
            }
        }
        None => {
            // check if user filter was specified
            if let Some(username) = &params.username {
                debug!("Fetching SSH keys for user {username}");
                // fetch user
                if let Some(user) = User::find_by_username(&appstate.pool, username).await? {
                    add_user_keys_to_list(user);
                } else {
                    debug!("Specified user does not exist")
                }
            }
        }
    }

    // concatenate all keys into a response
    Ok(ssh_keys.join("\n"))
}
