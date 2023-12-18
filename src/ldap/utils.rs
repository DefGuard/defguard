use sqlx::PgExecutor;

use super::{error::LdapError, LDAPConnection};
use crate::db::{DbPool, Group, User};

pub async fn user_from_ldap(
    pool: &DbPool,
    username: &str,
    password: &str,
) -> Result<User, LdapError> {
    let mut ldap_connection = LDAPConnection::create(pool).await?;
    let mut user = ldap_connection.get_user(username, password).await?;
    let _result = user.save(pool).await; // FIXME: do not ignore errors
    Ok(user)
}

pub async fn ldap_add_user<'e, E>(executor: E, user: &User, password: &str) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    match ldap_connection.add_user(user, password).await {
        Ok(()) => Ok(()),
        // this user might exist in LDAP, just try to set the password
        Err(_) => ldap_connection.set_password(&user.username, password).await,
    }
}

pub async fn ldap_modify_user<'e, E>(
    executor: E,
    username: &str,
    user: &User,
) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    ldap_connection.modify_user(username, user).await
}

pub async fn ldap_delete_user<'e, E>(executor: E, username: &str) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    ldap_connection.delete_user(username).await
}

pub async fn ldap_add_user_to_group<'e, E>(
    executor: E,
    username: &str,
    groupname: &str,
) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    ldap_connection.add_user_to_group(username, groupname).await
}

pub async fn ldap_remove_user_from_group<'e, E>(
    executor: E,
    username: &str,
    groupname: &str,
) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    ldap_connection
        .remove_user_from_group(username, groupname)
        .await
}

pub async fn ldap_change_password<'e, E>(
    executor: E,
    username: &str,
    password: &str,
) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    ldap_connection.set_password(username, password).await
}

pub async fn ldap_modify_group<'e, E>(
    executor: E,
    groupname: &str,
    group: &Group,
) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    ldap_connection.modify_group(groupname, group).await
}
