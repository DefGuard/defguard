use crate::db::{Group, Id, NoId, User, WireguardNetwork};
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{postgres::types::PgRange, query_as, Error as SqlxError, PgExecutor, PgPool};

/// Helper struct combining all DB objects related to given [`AclRule`]
pub struct AclRuleInfo {
    pub id: Id,
    pub name: String,
    pub all_users: bool,
    pub all_locations: bool,
    pub expires: Option<NaiveDateTime>,
    pub networks: Vec<WireguardNetwork<Id>>,
    // source
    pub users: Vec<User<Id>>,
    pub groups: Vec<Group<Id>>,
    // destination
    pub aliases: Vec<AclAlias<Id>>,
    pub ports: Vec<PgRange<i32>>,
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
pub struct AclRule<I = NoId> {
    pub id: I,
    pub name: String,
    pub all_users: bool,
    pub all_locations: bool,
    #[model(ref)]
    pub ports: Vec<PgRange<i32>>,
    pub expires: Option<NaiveDateTime>,
}

impl AclRule {
    #[must_use]
    pub(crate) fn new<S: Into<String>>(
        name: S,
        all_users: bool,
        all_locations: bool,
        ports: Vec<PgRange<i32>>,
        expires: Option<NaiveDateTime>,
    ) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            all_users,
            all_locations,
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
        if self.all_locations {
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

    pub(crate) async fn get_users<'e, E>(&self, executor: E) -> Result<Vec<User<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if self.all_users {
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
                AND u.is_active = true",
                self.id,
            )
            .fetch_all(executor)
            .await
        }
    }

    pub(crate) async fn get_groups<'e, E>(&self, executor: E) -> Result<Vec<Group<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Group,
            "SELECT g.id, name, is_admin \
            FROM aclrulegroup r \
            JOIN \"group\" g \
            ON g.id = r.group_id \
            WHERE r.rule_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    pub async fn to_info(&self, pool: &PgPool) -> Result<AclRuleInfo, SqlxError> {
        let aliases = self.get_aliases(pool).await?;
        let networks = self.get_networks(pool).await?;
        let users = self.get_users(pool).await?;
        let groups = self.get_groups(pool).await?;

        Ok(AclRuleInfo {
            id: self.id,
            name: self.name.clone(),
            all_users: self.all_users,
            all_locations: self.all_locations,
            ports: self.ports.clone(),
            expires: self.expires,
            aliases,
            networks,
            users,
            groups,
        })
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
        let mut rule = AclRule::new("rule", false, false, Vec::new(), None)
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

        let _ru = AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user1.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();

        let group1 = Group::new("group1").save(&pool).await.unwrap();
        let _group2 = Group::new("group2").save(&pool).await.unwrap();
        let _rg = AclRuleGroup {
            id: NoId,
            rule_id: rule.id,
            group_id: group1.id,
            allow: true,
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
        assert_eq!(info.users.len(), 1);
        assert_eq!(info.groups.len(), 1);
        assert_eq!(info.networks.len(), 1);

        assert_eq!(info.aliases[0].id, alias1.id); // db modifies datetime precision
        assert_eq!(info.users[0], user1);
        assert_eq!(info.groups[0], group1);
        assert_eq!(info.networks[0], network1);

        // test all_locations flag
        rule.all_locations = true;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_networks(&pool).await.unwrap().len(), 2);

        // test `all_users` flag
        rule.all_users = true;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_users(&pool).await.unwrap().len(), 2);

        // ensure only active users are returned with `all_users = true`
        let _ru = AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user2.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();
        user1.is_active = false;
        user1.save(&pool).await.unwrap();
        let users = rule.get_users(&pool).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], user2);

        // ensure only active users are returned with `all_users = false`
        rule.all_users = false;
        rule.save(&pool).await.unwrap();
        user1.is_active = true;
        user1.save(&pool).await.unwrap();
        let users = rule.get_users(&pool).await.unwrap();
        assert_eq!(users.len(), 2);

        user1.is_active = false;
        user1.save(&pool).await.unwrap();
        let users = rule.get_users(&pool).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], user2);
    }
}
