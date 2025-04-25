//!
//! This module contains utility functions for LDAP operations. Those operations are designed to be used from outside of the module.
//!

use std::collections::{HashMap, HashSet};

use sqlx::PgPool;

use super::{error::LdapError, LDAPConnection};
use crate::{
    db::{Group, Id, User},
    enterprise::ldap::with_ldap_status,
};

pub(crate) async fn login_through_ldap(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<User<Id>, LdapError> {
    debug!("Logging in user {username} through LDAP");
    let mut ldap_connection = LDAPConnection::create().await?;
    let ldap_user = ldap_connection
        .fetch_user_by_credentials(username, password)
        .await?;
    if !ldap_connection.user_in_ldap_sync_groups(&ldap_user).await? {
        return Err(LdapError::UserNotInLDAPSyncGroups(
            username.to_string(),
            "LDAP",
        ));
    }
    debug!("User {ldap_user} logged in through LDAP");
    let user =
        if let Some(defguard_user) = User::find_by_username(pool, &ldap_user.username).await? {
            if !defguard_user.ldap_sync_allowed(pool).await? {
                return Err(LdapError::UserNotInLDAPSyncGroups(
                    ldap_user.to_string(),
                    "Defguard",
                ));
            }
            defguard_user
        } else {
            ldap_user.save(pool).await?
        };

    Ok(user)
}

pub(crate) async fn user_from_ldap(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<User<Id>, LdapError> {
    debug!("Getting user {username} from LDAP");
    let mut ldap_connection = LDAPConnection::create().await?;

    let ldap_user = ldap_connection
        .fetch_user_by_credentials(username, password)
        .await?;
    if !ldap_connection.user_in_ldap_sync_groups(&ldap_user).await? {
        return Err(LdapError::UserNotInLDAPSyncGroups(
            username.to_string(),
            "LDAP",
        ));
    }
    let user = ldap_user.save(pool).await?;

    debug!("User {user} found in LDAP");

    Ok(user)
}

/// Convenience wrapper around [`ldap_update_users_state`] to update a single user.
pub(crate) async fn ldap_update_user_state(user: &mut User<Id>, pool: &PgPool) {
    let vec = vec![user];
    ldap_update_users_state(vec, pool).await;
}

/// See the [`LDAPConnection::update_users_state`] function for details.
pub(crate) async fn ldap_update_users_state(users: Vec<&mut User<Id>>, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Updating users state in LDAP");
        let mut ldap_connection = LDAPConnection::create().await?;
        ldap_connection.update_users_state(users, pool).await?;
        Ok(())
    })
    .await;
}

/// Adds user to LDAP, if no password was specified, a temporary random password will be used.
/// This will set the `ldap_pass_randomized` field to `true` in the user.
///
/// If the user already exists, the creation will be skipped.
pub(crate) async fn ldap_add_user(user: &mut User<Id>, password: Option<&str>, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Creating user {user} in LDAP");
        if !user.ldap_sync_allowed(pool).await? {
            debug!("User {user} is not allowed to be synced to LDAP as he is not in the specified sync groups, skipping");
            return Ok(());
        }
        let mut ldap_connection = LDAPConnection::create().await?;
        if ldap_connection.user_exists(user).await? {
            debug!("User {user} already exists in LDAP, skipping creation");
            return Ok(());
        }
        match ldap_connection.add_user(user, password, pool).await {
            Ok(()) => Ok(()),
            // this user might exist in LDAP, just try to set the password
            Err(err) => {
                warn!("There was an error while trying to create the user {user} in LDAP: {err}");
                debug!(
                    "Trying to set password for user {user} in LDAP, in case he already existed",
                );
                if let Some(password) = password {
                    ldap_connection.set_password(user, password).await?;
                    debug!("Password set for user {user} in LDAP");
                } else {
                    debug!(
                        "No password provided, skipping password setting for user {user} in LDAP"
                    );
                }
                Ok(())
            }
        }
    })
    .await;
}

/// Applies user modifications to LDAP. May update the user object if
/// his RDN in Defguard needs updating. Fails and sets the sync status to desynced
/// if the user does not exist in LDAP despite updating his state.
pub(crate) async fn ldap_handle_user_modify(
    old_username: &str,
    current_user: &mut User<Id>,
    pool: &PgPool,
) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Handling user modify for {old_username} in LDAP");

        // Check if the user is allowed to be synced at all
        if !current_user.ldap_sync_allowed(pool).await? {
            debug!("User {current_user} is not allowed to be synced to LDAP as he is not in the specified sync groups, skipping");
            return Ok(());
        }

        let mut ldap_connection = LDAPConnection::create().await?;
        if !ldap_connection.user_exists(current_user).await? {
            debug!("User {current_user} doesn't exist in LDAP, updating his state first as it may be stale");
            ldap_connection.update_users_state(vec![current_user], pool).await?;
        } else {
            debug!("User {current_user} exists in LDAP, modifying it");
        }
        ldap_connection
            .modify_user(old_username, current_user, pool)
            .await
    })
    .await;
}

pub(crate) async fn ldap_delete_user<I>(user: &User<I>, pool: &PgPool) {
    ldap_delete_users(vec![user], pool).await;
}

pub(crate) async fn ldap_delete_users<I>(users: Vec<&User<I>>, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Deleting {:?} users from LDAP", users.len());
        let mut ldap_connection = LDAPConnection::create().await?;
        for user in users {
            debug!("Deleting user {user} from LDAP");
            ldap_connection.delete_user(user).await?;
            debug!("User {user} deleted from LDAP");
        }

        Ok(())
    })
    .await;
}

/// Remove singular user from multiple groups in ldap.
pub(crate) async fn ldap_remove_user_from_groups(
    user: &User<Id>,
    groups: HashSet<&str>,
    pool: &PgPool,
) {
    let map = HashMap::from([(user, groups)]);
    ldap_remove_users_from_groups(map, pool).await;
}

/// Add singular user to multiple groups in ldap. Convenience wrapper around [`ldap_add_users_to_groups`].
pub(crate) async fn ldap_add_user_to_groups(user: &User<Id>, groups: HashSet<&str>, pool: &PgPool) {
    let map = HashMap::from([(user, groups)]);
    ldap_add_users_to_groups(map, pool).await
}

/// Bulk add users to groups in ldap.
pub(crate) async fn ldap_add_users_to_groups(
    user_groups: HashMap<&User<Id>, HashSet<&str>>,
    pool: &PgPool,
) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        let mut ldap_connection = LDAPConnection::create().await?;
        let sync_groups = ldap_connection.config.ldap_sync_groups.clone();
        let sync_groups_lookup = sync_groups.iter().map(|s| s.as_str()).collect::<HashSet<_>>();

        for (user, groups) in user_groups {
            let adding_to_sync_groups = groups
                .iter()
                .any(|group| sync_groups_lookup.contains(*group));
            if !user.ldap_sync_allowed(pool).await? && !adding_to_sync_groups {
                debug!("User {user} is not allowed to be synced to LDAP as he is not in the specified sync groups, skipping");
                continue;
            }

            for group in groups {
                ldap_connection.add_user_to_group(user, group).await?;
            }
        }

        Ok(())
    })
    .await;
}

/// Bulk remove users from groups in ldap.
pub(crate) async fn ldap_remove_users_from_groups(
    user_groups: HashMap<&User<Id>, HashSet<&str>>,
    pool: &PgPool,
) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        let mut ldap_connection = LDAPConnection::create().await?;
        let sync_groups = ldap_connection.config.ldap_sync_groups.clone();
        let sync_groups_lookup = sync_groups.iter().map(|s| s.as_str()).collect::<HashSet<_>>();

        for (user, groups) in user_groups {
            let removing_from_sync_groups = groups
                .iter()
                .any(|group| sync_groups_lookup.contains(*group));
            if !user.ldap_sync_allowed(pool).await? && !removing_from_sync_groups {
                debug!("User {user} is not allowed to be synced to LDAP as he is not in the specified sync groups, skipping");
                continue;
            }
            for group in groups {
                if ldap_connection.group_exists(group).await? {
                    ldap_connection.remove_user_from_group(user, group).await?;
                } else {
                    debug!("Group {group} doesn't exist in LDAP, skipping removal of user {user}");
                }
            }
        }

        Ok(())
    })
    .await;
}

pub(crate) async fn ldap_change_password(user: &mut User<Id>, password: &str, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Changing password for user {user} in LDAP");
        if !user.ldap_sync_allowed(pool).await? {
            debug!("User {user} is not allowed to be synced to LDAP as he is not in the specified sync groups, skipping");
            return Ok(());
        }
        let mut ldap_connection = LDAPConnection::create().await?;
        if !ldap_connection.user_exists(user).await? {
            debug!("User {user} doesn't exist in LDAP, creating it with the provided password");
            let user_groups = user.member_of_names(pool).await?;
            ldap_connection.add_user(user, Some(password), pool).await?;
            for group in user_groups {
                ldap_connection.add_user_to_group(user, &group).await?;
            }
           debug!("User {user} created in LDAP with the provided password");
        } else {
            debug!("User {user} exists in LDAP, changing password");
            ldap_connection
                .set_password(user, password)
                .await?;
            debug!(
                "Password changed for user {user} in LDAP, marking the LDAP password as set in Defguard"
            );
            user.ldap_pass_randomized = false;
            user.save(pool).await?;
            debug!(
                "LDAP password state updated in Defguard for user {user}"
            );
        }

        Ok(())
    })
    .await;
}

pub(crate) async fn ldap_modify_group(groupname: &str, group: &Group<Id>, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Modifying group {groupname} in LDAP");
        let mut ldap_connection = LDAPConnection::create().await?;
        ldap_connection.modify_group(groupname, group).await
    })
    .await;
}

pub(crate) async fn ldap_delete_group(groupname: &str, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Deleting group {groupname} from LDAP");
        let mut ldap_connection = LDAPConnection::create().await?;
        ldap_connection.delete_group(groupname).await
    })
    .await;
}
