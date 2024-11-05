use sqlx::{PgExecutor, PgPool};

use super::{error::LdapError, LDAPConnection};
use crate::db::{models::user::User, Id};

pub(crate) async fn user_from_ldap(
    pool: &PgPool,
    username: &str,
    password: &str,
) -> Result<User<Id>, LdapError> {
    let mut ldap_connection = LDAPConnection::create(pool).await?;
    // FIXME: do not ignore errors
    ldap_connection
        .get_user(username, password)
        .await?
        .save(pool)
        .await
        .map_err(|_| LdapError::Database)
}

pub(crate) async fn ldap_add_user<'e, E>(
    executor: E,
    user: &User<Id>,
    password: &str,
) -> Result<(), LdapError>
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

pub(crate) async fn ldap_modify_user<'e, E>(
    executor: E,
    username: &str,
    user: &User<Id>,
) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    ldap_connection.modify_user(username, user).await
}

pub(crate) async fn ldap_delete_user<'e, E>(executor: E, username: &str) -> Result<(), LdapError>
where
    E: PgExecutor<'e>,
{
    let mut ldap_connection = LDAPConnection::create(executor).await?;
    ldap_connection.delete_user(username).await
}

// pub(crate) async fn ldap_add_user_to_group<'e, E>(
//     executor: E,
//     username: &str,
//     groupname: &str,
// ) -> Result<(), LdapError>
// where
//     E: PgExecutor<'e>,
// {
//     let mut ldap_connection = LDAPConnection::create(executor).await?;
//     ldap_connection.add_user_to_group(username, groupname).await
// }

// pub(crate) async fn ldap_remove_user_from_group<'e, E>(
//     executor: E,
//     username: &str,
//     groupname: &str,
// ) -> Result<(), LdapError>
// where
//     E: PgExecutor<'e>,
// {
//     let mut ldap_connection = LDAPConnection::create(executor).await?;
//     ldap_connection
//         .remove_user_from_group(username, groupname)
//         .await
// }

pub(crate) async fn ldap_change_password<'e, E>(
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

// pub(crate) async fn ldap_modify_group<'e, E>(
//     executor: E,
//     groupname: &str,
//     group: &Group,
// ) -> Result<(), LdapError>
// where
//     E: PgExecutor<'e>,
// {
//     let mut ldap_connection = LDAPConnection::create(executor).await?;
//     ldap_connection.modify_group(groupname, group).await
// }
