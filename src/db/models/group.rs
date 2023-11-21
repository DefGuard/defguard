use model_derive::Model;
use sqlx::{query, query_as, query_scalar, Error as SqlxError, PgConnection};

use crate::db::User;
use crate::{
    db::{models::error::ModelError, WireguardNetwork},
    DbPool,
};

#[derive(Model)]
pub struct Group {
    pub(crate) id: Option<i64>,
    pub name: String,
}

impl Group {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            id: None,
            name: name.into(),
        }
    }

    pub async fn find_by_name<'e, E>(executor: E, name: &str) -> Result<Option<Self>, SqlxError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        query_as!(
            Self,
            "SELECT id \"id?\", name FROM \"group\" WHERE name = $1",
            name
        )
        .fetch_optional(executor)
        .await
    }

    pub async fn member_usernames(&self, pool: &DbPool) -> Result<Vec<String>, SqlxError> {
        if let Some(id) = self.id {
            query_scalar!(
                "SELECT \"user\".username FROM \"user\" JOIN group_user ON \"user\".id = group_user.user_id \
                WHERE group_user.group_id = $1",
                id
            )
            .fetch_all(pool)
            .await
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn fetch_all_members(&self, pool: &DbPool) -> Result<Vec<User>, SqlxError> {
        if let Some(id) = self.id {
            query_as!(
                User,
                "SELECT \"user\".id \"id?\", username, password_hash, last_name, first_name, email, \
                phone, ssh_key, pgp_key, pgp_cert_id, mfa_enabled, totp_enabled, totp_secret, \
                mfa_method \"mfa_method: _\", recovery_codes \
                FROM \"user\" \
                JOIN group_user ON \"user\".id = group_user.user_id \
                WHERE group_user.group_id = $1",
                id
            )
            .fetch_all(pool)
            .await
        } else {
            Ok(Vec::new())
        }
    }
}

impl WireguardNetwork {
    /// Fetch a list of all allowed groups for a given network from DB
    pub async fn fetch_allowed_groups<'e, E>(&self, executor: E) -> Result<Vec<String>, ModelError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        debug!("Fetching all allowed groups for network {}", self);
        let groups = query_scalar!(
            r#"
            SELECT name
            FROM wireguard_network_allowed_group wag
            JOIN "group" g ON wag.group_id = g.id
            WHERE wag.network_id = $1
            "#,
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
        admin_group_name: &String,
    ) -> Result<Option<Vec<String>>, ModelError> {
        debug!("Returning a list of allowed groups for network {}", self);
        // get allowed groups from DB
        let mut groups = self.fetch_allowed_groups(&mut *transaction).await?;

        // if no allowed groups are set then all group are allowed
        if groups.is_empty() {
            return Ok(None);
        }

        // make sure admin group is included
        if !groups.contains(admin_group_name) {
            groups.push(admin_group_name.clone());
        }

        Ok(Some(groups))
    }

    /// Set allowed groups, removing or adding groups as necessary.
    pub async fn set_allowed_groups(
        &self,
        transaction: &mut PgConnection,
        allowed_groups: Vec<String>,
    ) -> Result<(), ModelError> {
        info!("Setting allowed groups for network {}", self);
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
        info!("Adding allowed group {} for network {}", group, self);
        query!(
            r#"
            INSERT INTO wireguard_network_allowed_group (network_id, group_id)
            SELECT $1, g.id
            FROM "group" g
            WHERE g.name = $2
            "#,
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
        info!("Removing allowed groups {:?} for network {}", groups, self);
        let result = query!(
            r#"
            DELETE FROM wireguard_network_allowed_group
            WHERE network_id = $1 AND group_id IN (
                SELECT id
                FROM "group"
                WHERE name IN (SELECT * FROM UNNEST($2::text[]))
            )
            "#,
            self.id,
            &groups
        )
        .execute(transaction)
        .await?;
        info!(
            "Removed {} allowed groups for network {}",
            result.rows_affected(),
            self
        );
        Ok(())
    }

    /// Remove all allowed groups for a given network
    async fn clear_allowed_groups(&self, transaction: &mut PgConnection) -> Result<(), ModelError> {
        info!("Removing all allowed groups for network {}", self);
        let result = query!(
            "DELETE FROM wireguard_network_allowed_group WHERE network_id=$1",
            self.id
        )
        .execute(transaction)
        .await?;
        info!(
            "Removed {} allowed groups for network {}",
            result.rows_affected(),
            self
        );
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::User;

    #[sqlx::test]
    async fn test_group(pool: DbPool) {
        let mut group = Group::new("worker");
        group.save(&pool).await.unwrap();

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
    async fn test_group_members(pool: DbPool) {
        let mut group = Group::new("worker");
        group.save(&pool).await.unwrap();

        let mut user = User::new(
            "hpotter".into(),
            Some("pass123"),
            "Potter".into(),
            "Harry".into(),
            "h.potter@hogwart.edu.uk".into(),
            None,
        );
        user.save(&pool).await.unwrap();
        user.add_to_group(&pool, &group).await.unwrap();

        let members = group.member_usernames(&pool).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0], user.username);

        user.remove_from_group(&pool, &group).await.unwrap();

        let members = group.member_usernames(&pool).await.unwrap();
        assert!(members.is_empty());
    }
}
