use sqlx::PgPool;

use super::{error::LdapError, LDAPConnection};
use crate::db::{Group, Id, User};

pub async fn user_from_ldap(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<User<Id>, LdapError> {
    let mut ldap_connection = LDAPConnection::create().await?;
    // FIXME: do not ignore errors
    ldap_connection
        .get_user(username, password)
        .await?
        .save(pool)
        .await
        .map_err(|_| LdapError::Database)
}

pub async fn ldap_add_user(user: &User<Id>, password: &str) -> Result<(), LdapError> {
    let mut ldap_connection = LDAPConnection::create().await?;
    match ldap_connection.add_user(user, password).await {
        Ok(()) => Ok(()),
        // this user might exist in LDAP, just try to set the password
        Err(_) => ldap_connection.set_password(&user.username, password).await,
    }
}

pub async fn ldap_modify_user(username: &str, user: &User<Id>) -> Result<(), LdapError> {
    let mut ldap_connection = LDAPConnection::create().await?;
    ldap_connection.modify_user(username, user).await
}

pub async fn ldap_delete_user(username: &str) -> Result<(), LdapError> {
    let mut ldap_connection = LDAPConnection::create().await?;
    ldap_connection.delete_user(username).await
}

pub async fn ldap_add_user_to_group(username: &str, groupname: &str) -> Result<(), LdapError> {
    let mut ldap_connection = LDAPConnection::create().await?;
    ldap_connection.add_user_to_group(username, groupname).await
}

pub async fn ldap_remove_user_from_group(username: &str, groupname: &str) -> Result<(), LdapError> {
    let mut ldap_connection = LDAPConnection::create().await?;
    ldap_connection
        .remove_user_from_group(username, groupname)
        .await
}

pub async fn ldap_change_password(username: &str, password: &str) -> Result<(), LdapError> {
    let mut ldap_connection = LDAPConnection::create().await?;
    ldap_connection.set_password(username, password).await
}

pub async fn ldap_modify_group(groupname: &str, group: &Group) -> Result<(), LdapError> {
    let mut ldap_connection = LDAPConnection::create().await?;
    ldap_connection.modify_group(groupname, group).await
}
