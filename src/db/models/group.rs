use std::fmt;

use model_derive::Model;
use sqlx::{query, query_as, query_scalar, Error as SqlxError, FromRow, PgConnection, PgExecutor};
use utoipa::ToSchema;

use crate::db::{models::error::ModelError, Id, NoId, User, WireguardNetwork};

#[derive(Debug)]
pub enum Permission {
    Admin,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admin => write!(f, "admin"),
        }
    }
}

#[derive(Debug, Model, ToSchema, FromRow)]
pub struct Group<I = NoId> {
    pub(crate) id: I,
    pub name: String,
}

impl Group {
    #[must_use]
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            id: NoId,
            name: name.into(),
        }
    }
}

impl Group<Id> {
    pub async fn find_by_name<'e, E>(executor: E, name: &str) -> Result<Option<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(Self, "SELECT id, name FROM \"group\" WHERE name = $1", name)
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
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
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

    pub(crate) async fn find_by_permission<'e, E>(
        executor: E,
        permission: Permission,
    ) -> Result<Vec<Self>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let query = format!(
            "SELECT id, name FROM \"group\" WHERE id in (\
            SELECT group_id FROM group_permission WHERE {} = TRUE\
        )",
            permission
        );
        query_as(&query).fetch_all(executor).await
    }
}

impl WireguardNetwork<Id> {
    /// Fetch a list of all allowed groups for a given network from DB
    pub async fn fetch_allowed_groups<'e, E>(&self, executor: E) -> Result<Vec<String>, ModelError>
    where
        E: PgExecutor<'e>,
    {
        debug!("Fetching all allowed groups for network {self}");
        let groups = query_scalar!(
            "SELECT name FROM wireguard_network_allowed_group wag \
            JOIN \"group\" g ON wag.group_id = g.id WHERE wag.network_id = $1",
            self.id
        )
        .fetch_all(executor)
        .await?;

        Ok(groups)
    }

    /// Return a list of allowed groups for a given network.
    /// Admin group should always be included.
    /// If no `allowed_groups` are specified for a network then all devices are allowed.
    /// In this case `None` is returned to signify that there's no filtering.
    /// This helper method is meant for use in all business logic gating
    /// access to networks based on allowed groups.
    pub async fn get_allowed_groups(
        &self,
        transaction: &mut PgConnection,
    ) -> Result<Option<Vec<String>>, ModelError> {
        debug!("Returning a list of allowed groups for network {self}");
        let admin_groups = Group::find_by_permission(&mut *transaction, Permission::Admin).await?;

        // get allowed groups from DB
        let mut groups = self.fetch_allowed_groups(&mut *transaction).await?;

        // if no allowed groups are set then all groups are allowed
        if groups.is_empty() {
            return Ok(None);
        }

        for group in admin_groups {
            if !groups.iter().any(|name| name == &group.name) {
                groups.push(group.name);
            }
        }

        Ok(Some(groups))
    }

    /// Set allowed groups, removing or adding groups as necessary.
    pub async fn set_allowed_groups(
        &self,
        transaction: &mut PgConnection,
        allowed_groups: Vec<String>,
    ) -> Result<(), ModelError> {
        info!("Setting allowed groups for network {self} to : {allowed_groups:?}");
        if allowed_groups.is_empty() {
            return self.clear_allowed_groups(transaction).await;
        }

        // get list of current allowed groups
        let mut current_groups = self.fetch_allowed_groups(&mut *transaction).await?;

        // add to group if not already a member
        for group in &allowed_groups {
            if !current_groups.contains(group) {
                self.add_to_group(transaction, group).await?;
            }
        }

        // remove groups which are no longer present
        current_groups.retain(|group| !allowed_groups.contains(group));
        if !current_groups.is_empty() {
            self.remove_from_groups(transaction, current_groups).await?;
        }

        Ok(())
    }

    pub async fn add_to_group(
        &self,
        transaction: &mut PgConnection,
        group: &str,
    ) -> Result<(), ModelError> {
        info!("Adding allowed group {group} for network {self}");
        query!(
            "INSERT INTO wireguard_network_allowed_group (network_id, group_id) \
            SELECT $1, g.id FROM \"group\" g WHERE g.name = $2",
            self.id,
            group
        )
        .execute(transaction)
        .await?;
        Ok(())
    }

    pub async fn remove_from_groups(
        &self,
        transaction: &mut PgConnection,
        groups: Vec<String>,
    ) -> Result<(), ModelError> {
        info!("Removing allowed groups {groups:?} for network {self}");
        let result = query!(
            "DELETE FROM wireguard_network_allowed_group \
            WHERE network_id = $1 AND group_id IN ( \
                SELECT id FROM \"group\" \
                WHERE name IN (SELECT * FROM UNNEST($2::text[])) \
            )",
            self.id,
            &groups
        )
        .execute(transaction)
        .await?;
        info!(
            "Removed {} allowed groups for network {self}",
            result.rows_affected(),
        );
        Ok(())
    }

    /// Remove all allowed groups for a given network
    async fn clear_allowed_groups(&self, transaction: &mut PgConnection) -> Result<(), ModelError> {
        info!("Removing all allowed groups for network {self}");
        let result = query!(
            "DELETE FROM wireguard_network_allowed_group WHERE network_id=$1",
            self.id
        )
        .execute(transaction)
        .await?;
        info!(
            "Removed {} allowed groups for network {self}",
            result.rows_affected(),
        );
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::{PgPool, User};

    #[sqlx::test]
    async fn test_group(pool: PgPool) {
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
    async fn test_group_members(pool: PgPool) {
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
}
