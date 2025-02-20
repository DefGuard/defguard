use std::collections::HashSet;

use crate::db::{Group, Id, NoId, User, WireguardNetwork};
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{postgres::types::PgRange, query_as, Error as SqlxError, PgExecutor, PgPool};

/// Helper struct combining all DB objects related to given [`AclRule`]
pub struct AclRuleInfo {
    pub id: Id,
    pub name: String,
    pub all_networks: bool,
    pub networks: Vec<WireguardNetwork<Id>>,
    pub expires: Option<NaiveDateTime>,
    // source
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub allowed_users: Vec<User<Id>>,
    pub denied_users: Vec<User<Id>>,
    pub allowed_groups: Vec<Group<Id>>,
    pub denied_groups: Vec<Group<Id>>,
    // destination
    pub destination: Vec<IpNetwork>, // TODO: does not solve the "IP range" case
    pub aliases: Vec<AclAlias<Id>>,
    pub ports: Vec<PgRange<i32>>,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRule<I = NoId> {
    pub id: I,
    pub name: String,
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub all_networks: bool,
    #[model(ref)]
    pub destination: Vec<IpNetwork>, // TODO: does not solve the "IP range" case
    #[model(ref)]
    pub ports: Vec<PgRange<i32>>,
    pub expires: Option<NaiveDateTime>,
}

impl AclRule {
    #[must_use]
    pub(crate) fn new<S: Into<String>>(
        name: S,
        allow_all_users: bool,
        deny_all_users: bool,
        all_networks: bool,
        destination: Vec<IpNetwork>,
        ports: Vec<PgRange<i32>>,
        expires: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            allow_all_users,
            deny_all_users,
            all_networks,
            destination,
            ports,
            expires,
        }
    }
}

impl AclRule<Id> {
    pub(crate) async fn get_networks<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<WireguardNetwork<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if self.all_networks {
            WireguardNetwork::all(executor).await
        } else {
            query_as!(
                WireguardNetwork,
                "SELECT n.id, name, address, port, pubkey, prvkey, endpoint, dns, allowed_ips, \
                connected_at, mfa_enabled, keepalive_interval, peer_disconnect_threshold \
                FROM aclrulenetwork r \
                JOIN wireguard_network n \
                ON n.id = r.network_id \
                WHERE r.rule_id = $1",
                self.id,
            )
            .fetch_all(executor)
            .await
        }
    }

    pub(crate) async fn get_aliases<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AclAlias<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            AclAlias,
            "SELECT a.id, name, destination, ports, created_at \
            FROM aclrulealias r \
            JOIN aclalias a \
            ON a.id = r.alias_id \
            WHERE r.rule_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    pub(crate) async fn get_allowed_users<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<User<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if self.allow_all_users {
            query_as!(
                User,
                "SELECT id, username, password_hash, last_name, first_name, email, \
                phone, mfa_enabled, totp_enabled, totp_secret, \
                email_mfa_enabled, email_mfa_secret, \
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
                FROM \"user\" \
                WHERE is_active = true"
            )
            .fetch_all(executor)
            .await
        } else {
            query_as!(
                User,
                "SELECT u.id, username, password_hash, last_name, first_name, email, \
                phone, mfa_enabled, totp_enabled, totp_secret, \
                email_mfa_enabled, email_mfa_secret, \
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
                FROM aclruleuser r \
                JOIN \"user\" u \
                ON u.id = r.user_id \
                WHERE r.rule_id = $1 \
                AND r.allow \
                AND u.is_active = true",
                self.id,
            )
            .fetch_all(executor)
            .await
        }
    }

    pub(crate) async fn get_denied_users<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<User<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if self.deny_all_users {
            query_as!(
                User,
                "SELECT id, username, password_hash, last_name, first_name, email, \
                phone, mfa_enabled, totp_enabled, totp_secret, \
                email_mfa_enabled, email_mfa_secret, \
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
                FROM \"user\" \
                WHERE is_active = true"
            )
            .fetch_all(executor)
            .await
        } else {
            query_as!(
                User,
                "SELECT u.id, username, password_hash, last_name, first_name, email, \
                phone, mfa_enabled, totp_enabled, totp_secret, \
                email_mfa_enabled, email_mfa_secret, \
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
                FROM aclruleuser r \
                JOIN \"user\" u \
                ON u.id = r.user_id \
                WHERE r.rule_id = $1 \
                AND NOT r.allow \
                AND u.is_active = true",
                self.id,
            )
            .fetch_all(executor)
            .await
        }
    }

    pub(crate) async fn get_allowed_groups<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<Group<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Group,
            "SELECT g.id, name, is_admin \
            FROM aclrulegroup r \
            JOIN \"group\" g \
            ON g.id = r.group_id \
            WHERE r.rule_id = $1 \
            AND r.allow",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    pub(crate) async fn get_denied_groups<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<Group<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Group,
            "SELECT g.id, name, is_admin \
            FROM aclrulegroup r \
            JOIN \"group\" g \
            ON g.id = r.group_id \
            WHERE r.rule_id = $1 \
            AND NOT r.allow",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    /// Wrapper function which combines explicitly specified allowed users with members of allowed
    /// groups to generate a list of all unique allowed users for a given ACL.
    pub(crate) async fn get_all_allowed_users(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<User<Id>>, SqlxError> {
        // fetch explicitly allowed users
        let mut allowed_users = self.get_allowed_users(pool).await?;

        // fetch allowed groups
        let allowed_groups = self.get_allowed_groups(pool).await?;
        let allowed_group_ids: Vec<Id> = allowed_groups.iter().map(|group| group.id).collect();

        // fetch all active members of allowed groups
        let allowed_groups_users: Vec<User<Id>> = query_as!(
            User,
            "SELECT id, username, password_hash, last_name, first_name, email, \
                phone, mfa_enabled, totp_enabled, totp_secret, \
                email_mfa_enabled, email_mfa_secret, \
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
                FROM \"user\" u \
            JOIN group_user gu ON u.id=gu.user_id \
                WHERE u.is_active=true AND gu.group_id=ANY($1)",
            &allowed_group_ids
        )
        .fetch_all(pool)
        .await?;

        // get unique users from both lists
        allowed_users.extend(allowed_groups_users);
        let unique_allowed_users: HashSet<_> = allowed_users.into_iter().collect();

        // convert HashSet to output Vec
        Ok(unique_allowed_users.into_iter().collect())
    }

    /// Wrapper function which combines explicitly specified denied users with members of denied
    /// groups to generate a list of all unique denied users for a given ACL.
    pub(crate) async fn get_all_denied_users(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<User<Id>>, SqlxError> {
        // fetch explicitly denied users
        let mut denied_users = self.get_denied_users(pool).await?;

        // fetch denied groups
        let denied_groups = self.get_denied_groups(pool).await?;
        let denied_group_ids: Vec<Id> = denied_groups.iter().map(|group| group.id).collect();

        // fetch all active members of denied groups
        let denied_groups_users: Vec<User<Id>> = query_as!(
            User,
            "SELECT id, username, password_hash, last_name, first_name, email, \
                phone, mfa_enabled, totp_enabled, totp_secret, \
                email_mfa_enabled, email_mfa_secret, \
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub \
                FROM \"user\" u \
            JOIN group_user gu ON u.id=gu.user_id \
                WHERE u.is_active=true AND gu.group_id=ANY($1)",
            &denied_group_ids
        )
        .fetch_all(pool)
        .await?;

        // get unique users from both lists
        denied_users.extend(denied_groups_users);
        let unique_denied_users: HashSet<_> = denied_users.into_iter().collect();

        // convert HashSet to output Vec
        Ok(unique_denied_users.into_iter().collect())
    }

    pub async fn to_info(&self, pool: &PgPool) -> Result<AclRuleInfo, SqlxError> {
        let aliases = self.get_aliases(pool).await?;
        let networks = self.get_networks(pool).await?;
        let allowed_users = self.get_allowed_users(pool).await?;
        let denied_users = self.get_denied_users(pool).await?;
        let allowed_groups = self.get_allowed_groups(pool).await?;
        let denied_groups = self.get_denied_groups(pool).await?;

        Ok(AclRuleInfo {
            id: self.id,
            name: self.name.clone(),
            allow_all_users: self.allow_all_users,
            deny_all_users: self.deny_all_users,
            all_networks: self.all_networks,
            destination: self.destination.clone(),
            ports: self.ports.clone(),
            expires: self.expires,
            aliases,
            networks,
            allowed_users,
            denied_users,
            allowed_groups,
            denied_groups,
        })
    }
}

// TODO: serialize, deserialize #[derive(Clone, Debug, Deserialize, Model, PartialEq, Serialize, ToSchema)]
#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclAlias<I = NoId> {
    pub id: I,
    pub name: String,
    #[model(ref)]
    pub destination: Vec<IpNetwork>, // TODO: does not solve the "IP range" case
    #[model(ref)]
    pub ports: Vec<PgRange<i32>>,
    pub created_at: NaiveDateTime,
}

impl AclAlias {
    #[must_use]
    pub fn new<S: Into<String>>(
        name: S,
        destination: Vec<IpNetwork>,
        ports: Vec<PgRange<i32>>,
    ) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            destination,
            ports,
            created_at: Utc::now().naive_utc(),
        }
    }
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleNetwork<I = NoId> {
    pub id: I,
    pub rule_id: i64,
    pub network_id: i64,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleUser<I = NoId> {
    pub id: I,
    pub rule_id: i64,
    pub user_id: i64,
    pub allow: bool,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleGroup<I = NoId> {
    pub id: I,
    pub rule_id: i64,
    pub group_id: i64,
    pub allow: bool,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleAlias<I = NoId> {
    pub id: I,
    pub rule_id: i64,
    pub alias_id: i64,
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::handlers::wireguard::parse_address_list;
    use std::ops::Bound;

    #[sqlx::test]
    async fn test_alias(pool: PgPool) {
        let destination = parse_address_list("10.0.0.1, 10.1.0.0/16");
        let ports = vec![
            PgRange {
                start: Bound::Included(10),
                end: Bound::Excluded(21),
            },
            PgRange {
                start: Bound::Included(100),
                end: Bound::Excluded(201),
            },
        ];
        let alias = AclAlias::new("alias", destination.clone(), ports.clone())
            .save(&pool)
            .await
            .unwrap();

        assert_eq!(alias.id, 1);

        let retrieved = AclAlias::find_by_id(&pool, 1).await.unwrap().unwrap();

        assert_eq!(retrieved.id, 1);
        assert_eq!(retrieved.destination, destination);
        assert_eq!(retrieved.ports, ports);
    }

    #[sqlx::test]
    async fn test_rule_relations(pool: PgPool) {
        let mut rule = AclRule::new("rule", false, false, false, Vec::new(), Vec::new(), None)
            .save(&pool)
            .await
            .unwrap();

        let network1 = WireguardNetwork::new(
            "network1".to_string(),
            Vec::new(),
            1000,
            "endpoint1".to_string(),
            None,
            Vec::new(),
            false,
            100,
            100,
        )
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
        let _network2 = WireguardNetwork::new(
            "network2".to_string(),
            Vec::new(),
            2000,
            "endpoint2".to_string(),
            None,
            Vec::new(),
            false,
            200,
            200,
        )
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

        let _rn = AclRuleNetwork {
            id: NoId,
            rule_id: rule.id,
            network_id: network1.id,
        }
        .save(&pool)
        .await
        .unwrap();

        let mut user1 = User::new("user1", None, "", "", "u1@mail.com", None)
            .save(&pool)
            .await
            .unwrap();
        let user2 = User::new("user2", None, "", "", "u2@mail.com", None)
            .save(&pool)
            .await
            .unwrap();

        let _ru1 = AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user1.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();
        let _ru2 = AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user2.id,
            allow: false,
        }
        .save(&pool)
        .await
        .unwrap();

        let group1 = Group::new("group1").save(&pool).await.unwrap();
        let group2 = Group::new("group2").save(&pool).await.unwrap();
        let _rg = AclRuleGroup {
            id: NoId,
            rule_id: rule.id,
            group_id: group1.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();
        let _rg = AclRuleGroup {
            id: NoId,
            rule_id: rule.id,
            group_id: group2.id,
            allow: false,
        }
        .save(&pool)
        .await
        .unwrap();

        let alias1 = AclAlias::new("alias1", Vec::new(), Vec::new())
            .save(&pool)
            .await
            .unwrap();
        let _alias2 = AclAlias::new("alias2", Vec::new(), Vec::new())
            .save(&pool)
            .await
            .unwrap();
        let _ra = AclRuleAlias {
            id: NoId,
            rule_id: rule.id,
            alias_id: alias1.id,
        }
        .save(&pool)
        .await
        .unwrap();

        let info = rule.to_info(&pool).await.unwrap();

        assert_eq!(info.aliases.len(), 1);
        assert_eq!(info.allowed_users.len(), 1);
        assert_eq!(info.denied_users.len(), 1);
        assert_eq!(info.allowed_groups.len(), 1);
        assert_eq!(info.denied_groups.len(), 1);
        assert_eq!(info.networks.len(), 1);

        assert_eq!(info.aliases[0].id, alias1.id); // db modifies datetime precision
        assert_eq!(info.allowed_users[0], user1);
        assert_eq!(info.denied_users[0], user2);
        assert_eq!(info.allowed_groups[0], group1);
        assert_eq!(info.denied_groups[0], group2);
        assert_eq!(info.networks[0], network1);

        // test all_networks flag
        rule.all_networks = true;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_networks(&pool).await.unwrap().len(), 2);

        // test allowed/denied users
        let allowed_users = rule.get_allowed_users(&pool).await.unwrap();
        let denied_users = rule.get_denied_users(&pool).await.unwrap();
        assert_eq!(allowed_users.len(), 1);
        assert_eq!(allowed_users[0], user1);
        assert_eq!(denied_users.len(), 1);
        assert_eq!(denied_users[0], user2);

        // TODO: filter only active users?
        // test `allow_all_users` flag
        rule.allow_all_users = true;
        rule.deny_all_users = false;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_allowed_users(&pool).await.unwrap().len(), 2);
        assert_eq!(rule.get_denied_users(&pool).await.unwrap().len(), 0);

        // test `deny_all_users` flag
        rule.allow_all_users = false;
        rule.deny_all_users = true;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_allowed_users(&pool).await.unwrap().len(), 0);
        assert_eq!(rule.get_denied_users(&pool).await.unwrap().len(), 2);

        // ensure only active users are returned with `allow_all_users = true`
        rule.allow_all_users = true;
        rule.deny_all_users = false;
        rule.save(&pool).await.unwrap();

        user1.is_active = false;
        user1.save(&pool).await.unwrap();
        let allowed_users = rule.get_allowed_users(&pool).await.unwrap();
        let denied_users = rule.get_denied_users(&pool).await.unwrap();
        assert_eq!(allowed_users.len(), 1);
        assert_eq!(allowed_users[0], user2);
        assert_eq!(allowed_users.len(), 0);

        // ensure only active users are returned with `deny_all_users = true`
        rule.allow_all_users = false;
        rule.deny_all_users = true;
        rule.save(&pool).await.unwrap();

        user1.is_active = false;
        user1.save(&pool).await.unwrap();
        let allowed_users = rule.get_allowed_users(&pool).await.unwrap();
        let denied_users = rule.get_denied_users(&pool).await.unwrap();
        assert_eq!(allowed_users.len(), 0);
        assert_eq!(allowed_users.len(), 1);
        assert_eq!(allowed_users[0], user2);
    }
}
