use axum::extract::{Query, State};
use crate::appstate::AppState;
use crate::db::User;
use crate::error::WebError;

#[derive(Deserialize)]
pub struct SshKeysRequestParams {
    username: String,
    group: Option<String>,
}

/// Fetch public SSH keys for user
///
/// Meant to be used with `AuthorizedKeysCommand` config option in `sshd`.
/// Should always return a response to partially mitigate user enumeration.
/// Requires `username` query param and optionally `group` for further filtering
/// (for example to only authorize admin users).
#[axum::debug_handler]
pub async fn get_authorized_keys(params: Query<SshKeysRequestParams>, State(appstate): State<AppState>) -> Result<String, WebError> {
    info!("Fetching public SSH keys for user {}", params.username);
    let mut ssh_keys = Vec::new();

    // find user by username
    if let Some(user) = User::find_by_username(&appstate.pool, &params.username).await? {
        // TODO: check if user belongs to specified group

        // add key to list if user has an assigned SSH key
        if let Some(key) = user.ssh_key {
            ssh_keys.push(key);
        }
    } else {
        debug!("Specified user does not exist")
    }

    Ok(ssh_keys.join("\n"))
}
