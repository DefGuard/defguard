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
    let ldap_user = ldap_connection.get_user(username, password).await?;
    debug!("User {username} logged in through LDAP");
    let user =
        if let Some(defguard_user) = User::find_by_username(pool, &ldap_user.username).await? {
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
    let user = ldap_connection
        .get_user(username, password)
        .await?
        .save(pool)
        .await;

    Ok(user?)
}

/// Adds user to LDAP, if no password was specified, a temporary random password will be used.
/// This will set the `ldap_pass_randomized` field to `true` in the user.
pub(crate) async fn ldap_add_user(user: &mut User<Id>, password: Option<&str>, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Creating user {} in LDAP", user.username);
        let mut ldap_connection = LDAPConnection::create().await?;
        match ldap_connection.add_user(user, password, pool).await {
            Ok(()) => Ok(()),
            // this user might exist in LDAP, just try to set the password
            Err(err) => {
                warn!(
                    "There was an error while trying to create the user {} in LDAP: {}",
                    user.username, err
                );
                debug!(
                    "Trying to set password for user {} in LDAP, in case he already existed",
                    user.username
                );
                if let Some(password) = password {
                    ldap_connection
                        .set_password(&user.username, password)
                        .await?;
                    debug!("Password set for user {} in LDAP", user.username);
                } else {
                    debug!(
                        "No password provided, skipping password setting for user {} in LDAP",
                        user.username
                    );
                }
                Ok(())
            }
        }
    })
    .await;
}

pub(crate) async fn ldap_modify_user(username: &str, user: &User<Id>, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Modifying user {username} in LDAP");
        let mut ldap_connection = LDAPConnection::create().await?;
        ldap_connection.modify_user(username, user).await
    })
    .await;
}

pub(crate) async fn ldap_delete_user(username: &str, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Deleting user {username} from LDAP");
        let mut ldap_connection = LDAPConnection::create().await?;
        ldap_connection.delete_user(username).await
    })
    .await;
}

/// Remove singular user from multiple groups in ldap.
pub(crate) async fn ldap_add_user_to_groups(username: &str, groups: HashSet<&str>, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Adding user {username} to groups {groups:?}");
        let mut ldap_connection = LDAPConnection::create().await?;
        for group in groups {
            if ldap_connection.group_exists(group).await? {
                ldap_connection.add_user_to_group(username, group).await?;
            } else {
                debug!("Group {} doesn't exist in LDAP, creating it", group);
                ldap_connection
                    .add_group_with_members(group, vec![username])
                    .await?;
                debug!("Group {} created and member added in LDAP", group);
            }
        }

        Ok(())
    })
    .await;
}

/// Remove singular user from multiple groups in ldap.
pub(crate) async fn ldap_remove_user_from_groups(
    username: &str,
    groups: HashSet<&str>,
    pool: &PgPool,
) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Removing user {username} from groups {groups:?}");
        let mut ldap_connection = LDAPConnection::create().await?;
        for group in groups {
            if ldap_connection.group_exists(group).await? {
                ldap_connection
                    .remove_user_from_group(username, group)
                    .await?;
            } else {
                debug!(
                    "Group {} doesn't exist in LDAP, skipping removal of member {}",
                    group, username
                );
            }
        }

        Ok(())
    })
    .await;
}

/// Bulk add users to groups in ldap.
///
/// Pass in the following parameters:
/// - `user_groups`: A HashMap containing usernames as keys and a HashSet of group names as values.
pub(crate) async fn ldap_add_users_to_groups(
    user_groups: HashMap<&str, HashSet<&str>>,
    pool: &PgPool,
) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        let mut ldap_connection = LDAPConnection::create().await?;

        for (username, groups) in user_groups {
            for group in groups {
                if ldap_connection.group_exists(group).await? {
                    ldap_connection.add_user_to_group(username, group).await?;
                } else {
                    debug!("Group {} doesn't exist in LDAP, creating it", group);
                    ldap_connection
                        .add_group_with_members(group, vec![username])
                        .await?;
                    debug!("Group {} created and member added in LDAP", group);
                }
            }
        }

        Ok(())
    })
    .await;
}

/// Bulk remove users from groups in ldap.
///
/// Pass in the following parameters:
/// - `user_groups`: A HashMap containing usernames as keys and a HashSet of group names as values.
pub(crate) async fn ldap_remove_users_from_groups(
    user_groups: HashMap<&str, HashSet<&str>>,
    pool: &PgPool,
) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        let mut ldap_connection = LDAPConnection::create().await?;

        for (username, groups) in user_groups {
            for group in groups {
                if ldap_connection.group_exists(group).await? {
                    ldap_connection
                        .remove_user_from_group(username, group)
                        .await?;
                } else {
                    debug!(
                        "Group {} doesn't exist in LDAP, skipping removal of user {}",
                        group, username
                    );
                }
            }
        }

        Ok(())
    })
    .await;
}

pub(crate) async fn ldap_change_password(user: &mut User<Id>, password: &str, pool: &PgPool) {
    let _: Result<(), LdapError> = with_ldap_status(pool, async {
        debug!("Changing password for user {} in LDAP", user.username);
        let mut ldap_connection = LDAPConnection::create().await?;
        if !ldap_connection.user_exists(&user.username).await? {
            debug!("User {} doesn't exist in LDAP, creating it with the provided password", user.username);
            let user_groups = user.member_of_names(pool).await?;
            ldap_connection.add_user(user, Some(password), pool).await?;
            for group in user_groups {
                ldap_connection.add_user_to_group(&user.username, &group).await?;
            }
           debug!("User {} created in LDAP with the provided password", user.username);
        } else {
            debug!("User {} exists in LDAP, changing password", user.username);
            ldap_connection
                .set_password(&user.username, password)
                .await?;
            debug!(
                "Password changed for user {} in LDAP, marking the LDAP password as set in Defguard",
                user.username
            );
            user.ldap_pass_randomized = false;
            user.save(pool).await?;
            debug!(
                "LDAP password state updated in Defguard for user {}",
                user.username
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
