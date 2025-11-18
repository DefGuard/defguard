use std::fmt;

use defguard_common::db::{Id, NoId};
use model_derive::Model;
use sqlx::{Error as SqlxError, FromRow, PgExecutor, query, query_as, query_scalar};
use utoipa::ToSchema;

use crate::db::User;

#[derive(Debug)]
pub enum Permission {
    IsAdmin,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IsAdmin => write!(f, "is_admin"),
        }
    }
}

#[derive(Clone, Debug, Model, ToSchema, FromRow, PartialEq, Serialize)]
pub struct Group<I = NoId> {
    pub(crate) id: I,
    pub name: String,
    pub is_admin: bool,
}

#[cfg(test)]
impl Default for Group {
    fn default() -> Self {
        Self {
            id: NoId,
            name: Default::default(),
            is_admin: Default::default(),
        }
    }
}

impl Group {
    #[must_use]
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            is_admin: false,
        }
    }
}

impl Group<Id> {
    pub async fn find_by_name<'e, E>(executor: E, name: &str) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Self,
            "SELECT id, name, is_admin FROM \"group\" WHERE name = $1",
            name
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn member_usernames<'e, E>(&self, executor: E) -> Result<Vec<String>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_scalar!(
            "SELECT \"user\".username FROM \"user\" JOIN group_user ON \"user\".id = group_user.user_id \
            WHERE group_user.group_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn members<'e, E>(&self, executor: E) -> Result<Vec<User<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            User,
            "SELECT \"user\".id, username, password_hash, last_name, first_name, email, \
            phone, mfa_enabled, totp_enabled, totp_secret, email_mfa_enabled, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn, ldap_user_path, enrollment_pending \
            FROM \"user\" \
            JOIN group_user ON \"user\".id = group_user.user_id \
            WHERE group_user.group_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    /// Fetches a list of VPN locations where a given group is explicitly allowed.
    /// This does not include VPN locations where all groups are implicitly allowed (admin group),
    /// because no access control in configured.
    pub async fn allowed_vpn_locations<'e, E>(&self, executor: E) -> Result<Vec<String>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_scalar!(
            "SELECT wn.name FROM wireguard_network wn JOIN wireguard_network_allowed_group wnag ON wn.id = wnag.network_id \
            WHERE wnag.group_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await
    }

    pub async fn find_by_permission<'e, E>(
        executor: E,
        permission: Permission,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let query = format!(
            "SELECT id, name, is_admin FROM \"group\" WHERE {permission} = TRUE ORDER BY id"
        );
        query_as(&query).fetch_all(executor).await
    }

    pub(crate) async fn has_permission<'e, E>(
        &self,
        executor: E,
        permission: Permission,
    ) -> Result<bool, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let query_str = format!("SELECT {permission} FROM \"group\" WHERE id = $1");
        let result = query_scalar(&query_str)
            .bind(self.id)
            .fetch_optional(executor)
            .await?;
        Ok(result.unwrap_or(false))
    }

    pub(crate) async fn set_permission<'e, E>(
        &self,
        executor: E,
        permission: Permission,
        value: bool,
    ) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let query_str = format!("UPDATE \"group\" SET {permission} = $2 WHERE id = $1");
        query(&query_str)
            .bind(self.id)
            .bind(value)
            .execute(executor)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use defguard_common::db::setup_pool;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::db::User;

    #[sqlx::test]
    async fn test_group(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let group = Group::new("worker").save(&pool).await.unwrap();

        let fetched_group = Group::find_by_name(&pool, "worker").await.unwrap();
        assert!(fetched_group.is_some());
        assert_eq!(fetched_group.unwrap().name, "worker");

        let fetched_group = Group::find_by_name(&pool, "wheel").await.unwrap();
        assert!(fetched_group.is_none());

        group.delete(&pool).await.unwrap();

        let fetched_group = Group::find_by_name(&pool, "worker").await.unwrap();
        assert!(fetched_group.is_none());
    }

    #[sqlx::test]
    async fn test_group_members(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let group = Group::new("worker").save(&pool).await.unwrap();
        let user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();
        user.add_to_group(&pool, &group).await.unwrap();

        let members = group.member_usernames(&pool).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0], user.username);

        user.remove_from_group(&pool, &group).await.unwrap();

        let members = group.member_usernames(&pool).await.unwrap();
        assert!(members.is_empty());
    }

    #[sqlx::test]
    async fn test_group_permissions(_: PgPoolOptions, options: PgConnectOptions) {
        let pool = setup_pool(options).await;

        let group = Group::new("admin2").save(&pool).await.unwrap();
        let user = User::new(
            "hpotter",
            Some("pass123"),
            "Potter",
            "Harry",
            "h.potter@hogwart.edu.uk",
            None,
        )
        .save(&pool)
        .await
        .unwrap();
        user.add_to_group(&pool, &group).await.unwrap();
        assert!(!user.is_admin(&pool).await.unwrap());
        assert!(
            !group
                .has_permission(&pool, Permission::IsAdmin)
                .await
                .unwrap()
        );
        group
            .set_permission(&pool, Permission::IsAdmin, true)
            .await
            .unwrap();

        assert!(
            group
                .has_permission(&pool, Permission::IsAdmin)
                .await
                .unwrap()
        );
        assert!(user.is_admin(&pool).await.unwrap());
        let groups = Group::find_by_permission(&pool, Permission::IsAdmin)
            .await
            .unwrap();
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().any(|g| g.name == "admin2"));
    }
}
