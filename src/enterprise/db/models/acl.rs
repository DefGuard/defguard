use crate::{
    db::{Device, Group, Id, NoId, User, WireguardNetwork},
    enterprise::handlers::acl::ApiAclRule,
    DeviceType,
};
use chrono::{NaiveDateTime, Utc};
use ipnetwork::IpNetwork;
use model_derive::Model;
use sqlx::{
    postgres::types::PgRange, query, query_as, Error as SqlxError, PgConnection, PgExecutor, PgPool,
};
use std::ops::{Bound, Range};

/// https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/include/uapi/linux/in.h
pub type Protocol = i32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortRange(Range<i32>);

impl From<PgRange<i32>> for PortRange {
    fn from(range: PgRange<i32>) -> Self {
        let start = match range.start {
            Bound::Included(start) => start,
            Bound::Excluded(start) => start + 1,
            // should not happen if the value is created from PortRange
            Bound::Unbounded => 0,
        };
        let end = match range.end {
            Bound::Included(end) => end + 1,
            Bound::Excluded(end) => end,
            // should not happen if the value is created from PortRange
            Bound::Unbounded => 0,
        };
        Self(Range { start, end })
    }
}

impl From<PortRange> for PgRange<i32> {
    fn from(range: PortRange) -> PgRange<i32> {
        PgRange {
            start: Bound::Included(range.0.start),
            end: Bound::Excluded(range.0.end),
        }
    }
}

/// Helper struct combining all DB objects related to given [`AclRule`]
#[derive(Clone, Debug)]
pub struct AclRuleInfo<I = NoId> {
    pub id: I,
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
    pub allowed_devices: Vec<Device<Id>>,
    pub denied_devices: Vec<Device<Id>>,
    // destination
    pub destination: Vec<IpNetwork>,
    pub destination_ranges: Vec<AclRuleDestinationRange<Id>>,
    pub aliases: Vec<AclAlias<Id>>,
    pub ports: Vec<PortRange>,
    pub protocols: Vec<Protocol>,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRule<I = NoId> {
    pub id: I,
    pub name: String,
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub all_networks: bool,
    #[model(ref)]
    pub destination: Vec<IpNetwork>,
    #[model(ref)]
    pub ports: Vec<PgRange<i32>>,
    #[model(ref)]
    pub protocols: Vec<Protocol>,
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
        protocols: Vec<Protocol>,
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
            protocols,
            expires,
        }
    }

    /// Creates new [`AclRule`] with all related objects based on [`ApiAclRule`]
    pub(crate) async fn create_from_api(
        pool: &PgPool,
        api_rule: &ApiAclRule<NoId>,
    ) -> Result<ApiAclRule<Id>, SqlxError> {
        let mut transaction = pool.begin().await?;

        // save the rule
        let rule: AclRule<NoId> = api_rule.clone().into();
        let rule = rule.save(&mut *transaction).await?;

        // create related objects
        Self::create_related_objects(&mut transaction, rule.id, api_rule).await?;

        transaction.commit().await?;
        Ok(rule.to_info(pool).await?.into())
    }

    /// Updates [`AclRule`] with all it's related objects based on [`ApiAclRule`]
    pub(crate) async fn update_from_api(
        pool: &PgPool,
        id: Id,
        api_rule: &ApiAclRule<Id>,
    ) -> Result<ApiAclRule<Id>, SqlxError> {
        let mut transaction = pool.begin().await?;

        // save the rule
        let mut rule: AclRule<Id> = api_rule.clone().into();
        rule.id = id; // frontend may PUT an object with incorrect id
        rule.save(&mut *transaction).await?;

        // delete related objects
        Self::delete_related_objects(&mut transaction, rule.id).await?;

        // create related objects
        AclRule::<Id>::create_related_objects(&mut transaction, rule.id, api_rule).await?;

        transaction.commit().await?;
        Ok(rule.to_info(pool).await?.into())
    }

    /// Deletes [`AclRule`] with all it's related objects
    pub(crate) async fn delete_from_api(pool: &PgPool, id: Id) -> Result<(), SqlxError> {
        let mut transaction = pool.begin().await?;

        // delete related objects
        Self::delete_related_objects(&mut transaction, id).await?;

        // delete the rule
        query!("DELETE FROM aclrule WHERE id = $1", id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(())
    }
}

impl<I> AclRule<I> {
    /// Creates relation objects for given [`AclRule`] based on [`ApiAclRule`] object
    async fn create_related_objects(
        transaction: &mut PgConnection,
        rule_id: Id,
        api_rule: &ApiAclRule<I>,
    ) -> Result<(), SqlxError> {
        // save related networks
        for network_id in &api_rule.networks {
            let obj = AclRuleNetwork {
                id: NoId,
                rule_id,
                network_id: *network_id,
            };
            obj.save(&mut *transaction).await?;
        }

        // allowed users
        for user_id in &api_rule.allowed_users {
            let obj = AclRuleUser {
                id: NoId,
                allow: true,
                rule_id,
                user_id: *user_id,
            };
            obj.save(&mut *transaction).await?;
        }

        // denied users
        for user_id in &api_rule.denied_users {
            let obj = AclRuleUser {
                id: NoId,
                allow: false,
                rule_id,
                user_id: *user_id,
            };
            obj.save(&mut *transaction).await?;
        }

        // allowed groups
        for group_id in &api_rule.allowed_groups {
            let obj = AclRuleGroup {
                id: NoId,
                allow: true,
                rule_id,
                group_id: *group_id,
            };
            obj.save(&mut *transaction).await?;
        }

        // denied groups
        for group_id in &api_rule.denied_groups {
            let obj = AclRuleGroup {
                id: NoId,
                allow: false,
                rule_id,
                group_id: *group_id,
            };
            obj.save(&mut *transaction).await?;
        }

        // save related aliases
        for alias_id in &api_rule.aliases {
            let obj = AclRuleAlias {
                id: NoId,
                rule_id,
                alias_id: *alias_id,
            };
            obj.save(&mut *transaction).await?;
        }

        // allowed devices
        for device_id in &api_rule.allowed_devices {
            let obj = AclRuleDevice {
                id: NoId,
                allow: true,
                rule_id,
                device_id: *device_id,
            };
            obj.save(&mut *transaction).await?;
        }

        // denied devices
        for device_id in &api_rule.denied_devices {
            let obj = AclRuleDevice {
                id: NoId,
                allow: false,
                rule_id,
                device_id: *device_id,
            };
            obj.save(&mut *transaction).await?;
        }

        // destination ranges
        for range in &api_rule.destination_ranges {
            let obj = AclRuleDestinationRange {
                id: NoId,
                rule_id,
                start: range.start,
                end: range.end,
            };
            obj.save(&mut *transaction).await?;
        }

        Ok(())
    }

    /// Deletes relation objects for given [`AclRule`]
    async fn delete_related_objects(
        transaction: &mut PgConnection,
        rule_id: Id,
    ) -> Result<(), SqlxError> {
        // networks
        query!("DELETE FROM aclrulenetwork WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;

        // users
        query!("DELETE FROM aclruleuser WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;

        // groups
        query!("DELETE FROM aclrulegroup WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;

        // aliases
        query!("DELETE FROM aclrulealias WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;

        // devices
        query!("DELETE FROM aclruledevice WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;

        // destination ranges
        query!(
            "DELETE FROM aclruledestinationrange WHERE rule_id = $1",
            rule_id
        )
        .execute(&mut *transaction)
        .await?;

        Ok(())
    }
}

impl<I> From<ApiAclRule<I>> for AclRule<I> {
    fn from(rule: ApiAclRule<I>) -> Self {
        Self {
            ports: rule.ports.into_iter().map(Into::into).collect(),
            id: rule.id,
            name: rule.name,
            allow_all_users: rule.allow_all_users,
            deny_all_users: rule.deny_all_users,
            all_networks: rule.all_networks,
            destination: rule.destination,
            protocols: rule.protocols,
            expires: rule.expires,
        }
    }
}

impl AclRule<Id> {
    /// Returns all [`WireguardNetwork`]s the rule applies to
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

    /// Returns all [`AclAlias`]es the rule applies to
    pub(crate) async fn get_aliases<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AclAlias<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            AclAlias,
            "SELECT a.id, name, destination, ports, protocols, created_at \
            FROM aclrulealias r \
            JOIN aclalias a \
            ON a.id = r.alias_id \
            WHERE r.rule_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    /// Returns **active** [`User`]s that are allowed or denied by the rule
    async fn get_users<'e, E>(&self, executor: E, allowed: bool) -> Result<Vec<User<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        Ok(match allowed {
            true => self.get_allowed_users(executor).await?,
            false => self.get_denied_users(executor).await?,
        })
    }

    /// Returns **active** [`User`]s that are allowed by the rule
    async fn get_allowed_users<'e, E>(&self, executor: E) -> Result<Vec<User<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if self.deny_all_users {
            Ok(Vec::new())
        } else if self.allow_all_users {
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

    /// Returns **active** [`User`]s that are denied by the rule
    async fn get_denied_users<'e, E>(&self, executor: E) -> Result<Vec<User<Id>>, SqlxError>
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
        } else if self.allow_all_users {
            Ok(Vec::new())
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

    /// Returns [`Group`]s that are allowed or denied by the rule
    pub(crate) async fn get_groups<'e, E>(
        &self,
        executor: E,
        allowed: bool,
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
            AND r.allow = $2",
            self.id,
            allowed,
        )
        .fetch_all(executor)
        .await
    }

    /// Returns [`Device`]s that are allowed or denied by the rule
    pub(crate) async fn get_devices<'e, E>(
        &self,
        executor: E,
        allowed: bool,
    ) -> Result<Vec<Device<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Device,
            "SELECT d.id, name, wireguard_pubkey, user_id, created, description, device_type \"device_type: DeviceType\", \
            configured \
            FROM aclruledevice r \
            JOIN device d \
            ON d.id = r.device_id \
            WHERE r.rule_id = $1 \
            AND r.allow = $2",
            self.id,
            allowed,
        )
        .fetch_all(executor)
        .await
    }

    /// Returns all [`AclRuleDestinationRanges`]es the rule applies to
    pub(crate) async fn get_destination_ranges<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AclRuleDestinationRange<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            AclRuleDestinationRange,
            "SELECT id, rule_id, \"start\", \"end\" \
            FROM aclruledestinationrange r \
            WHERE rule_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    /// Converts [`AclRule`] instance to [`AclRuleInfo`]
    pub async fn to_info(&self, pool: &PgPool) -> Result<AclRuleInfo<Id>, SqlxError> {
        let aliases = self.get_aliases(pool).await?;
        let networks = self.get_networks(pool).await?;
        let allowed_users = self.get_users(pool, true).await?;
        let denied_users = self.get_users(pool, false).await?;
        let allowed_groups = self.get_groups(pool, true).await?;
        let denied_groups = self.get_groups(pool, false).await?;
        let allowed_devices = self.get_devices(pool, true).await?;
        let denied_devices = self.get_devices(pool, false).await?;
        let destination_ranges = self
            .get_destination_ranges(pool)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();
        let ports = self.ports.clone().into_iter().map(Into::into).collect();

        Ok(AclRuleInfo {
            id: self.id,
            name: self.name.clone(),
            allow_all_users: self.allow_all_users,
            deny_all_users: self.deny_all_users,
            all_networks: self.all_networks,
            destination: self.destination.clone(),
            protocols: self.protocols.clone(),
            expires: self.expires,
            destination_ranges,
            ports,
            aliases,
            networks,
            allowed_users,
            denied_users,
            allowed_groups,
            denied_groups,
            allowed_devices,
            denied_devices,
        })
    }
}

/// API representation of [`AclRuleDestinationRange`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AclRuleDestinationRangeInfo {
    pub start: IpNetwork,
    pub end: IpNetwork,
}

impl<I> From<AclRuleDestinationRange<I>> for AclRuleDestinationRangeInfo {
    fn from(rule: AclRuleDestinationRange<I>) -> Self {
        Self {
            start: rule.start,
            end: rule.end,
        }
    }
}

/// Helper struct combining all DB objects related to given [`AclAlias`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AclAliasInfo<I = NoId> {
    pub id: I,
    pub name: String,
    pub destination: Vec<IpNetwork>,
    pub destination_ranges: Vec<AclAliasDestinationRangeInfo>,
    pub ports: Vec<PortRange>,
    pub protocols: Vec<Protocol>,
    pub created_at: NaiveDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AclAliasDestinationRangeInfo {
    pub start: IpNetwork,
    pub end: IpNetwork,
}

impl<I> From<AclAliasDestinationRange<I>> for AclAliasDestinationRangeInfo {
    fn from(range: AclAliasDestinationRange<I>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

/// Defines an alias for ACL destination. Aliases can be
/// used to define the destination part of the rule.
#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclAlias<I = NoId> {
    pub id: I,
    pub name: String,
    #[model(ref)]
    pub destination: Vec<IpNetwork>,
    #[model(ref)]
    pub ports: Vec<PgRange<i32>>,
    #[model(ref)]
    pub protocols: Vec<Protocol>,
    pub created_at: NaiveDateTime,
}

impl<I> From<AclAliasInfo<I>> for AclAlias<I> {
    fn from(rule: AclAliasInfo<I>) -> Self {
        Self {
            ports: rule.ports.into_iter().map(Into::into).collect(),
            id: rule.id,
            name: rule.name,
            destination: rule.destination,
            protocols: rule.protocols,
            created_at: rule.created_at,
        }
    }
}

impl AclAlias {
    #[must_use]
    pub fn new<S: Into<String>>(
        name: S,
        destination: Vec<IpNetwork>,
        ports: Vec<PgRange<i32>>,
        protocols: Vec<Protocol>,
    ) -> Self {
        Self {
            id: NoId,
            name: name.into(),
            destination,
            ports,
            protocols,
            created_at: Utc::now().naive_utc(),
        }
    }

    /// Creates new [`AclAlias`] with all related objects based on [`AclAliasInfo`]
    pub(crate) async fn create_from_api(
        pool: &PgPool,
        api_alias: &AclAliasInfo<NoId>,
    ) -> Result<AclAliasInfo<Id>, SqlxError> {
        let mut transaction = pool.begin().await?;

        // save the alias
        let alias: AclAlias<NoId> = api_alias.clone().into();
        let alias = alias.save(&mut *transaction).await?;

        // create related objects
        Self::create_related_objects(&mut transaction, alias.id, api_alias).await?;

        transaction.commit().await?;
        Ok(alias.to_info(pool).await?.into())
    }

    /// Updates [`AclAlias`] with all it's related objects based on [`AclAliasInfo`]
    pub(crate) async fn update_from_api(
        pool: &PgPool,
        id: Id,
        api_alias: &AclAliasInfo<Id>,
    ) -> Result<AclAliasInfo<Id>, SqlxError> {
        let mut transaction = pool.begin().await?;

        // save the alias
        let mut alias: AclAlias<Id> = api_alias.clone().into();
        alias.id = id; // frontend may PUT an object with incorrect id
        alias.save(&mut *transaction).await?;

        // delete related objects
        Self::delete_related_objects(&mut transaction, alias.id).await?;

        // create related objects
        AclAlias::<Id>::create_related_objects(&mut transaction, alias.id, api_alias).await?;

        transaction.commit().await?;
        Ok(alias.to_info(pool).await?.into())
    }

    /// Deletes [`AclAlias`] with all it's related objects
    pub(crate) async fn delete_from_api(pool: &PgPool, id: Id) -> Result<(), SqlxError> {
        let mut transaction = pool.begin().await?;

        // delete related objects
        Self::delete_related_objects(&mut transaction, id).await?;

        // delete the alias
        query!("DELETE FROM aclalias WHERE id = $1", id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(())
    }
}

impl<I> AclAlias<I> {
    /// Creates relation objects for given [`AclAlias`] based on [`AclAliasInfo`] object
    async fn create_related_objects(
        transaction: &mut PgConnection,
        alias_id: Id,
        api_alias: &AclAliasInfo<I>,
    ) -> Result<(), SqlxError> {
        // save related destination ranges
        for range in &api_alias.destination_ranges {
            let obj = AclAliasDestinationRange {
                id: NoId,
                alias_id,
                start: range.start,
                end: range.end,
            };
            obj.save(&mut *transaction).await?;
        }

        Ok(())
    }

    /// Deletes relation objects for given [`AclAlias`]
    async fn delete_related_objects(
        transaction: &mut PgConnection,
        alias_id: Id,
    ) -> Result<(), SqlxError> {
        // destination ranges
        query!(
            "DELETE FROM aclaliasdestinationrange WHERE alias_id = $1",
            alias_id
        )
        .execute(&mut *transaction)
        .await?;

        Ok(())
    }
}

impl AclAlias<Id> {
    /// Returns all [`AclAliasDestinationRanges`]es the alias applies to
    pub(crate) async fn get_destination_ranges<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<AclAliasDestinationRange<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            AclAliasDestinationRange,
            "SELECT id, alias_id, \"start\", \"end\" \
            FROM aclaliasdestinationrange r \
            WHERE alias_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    pub(crate) async fn to_info(&self, pool: &PgPool) -> Result<AclAliasInfo<Id>, SqlxError> {
        let destination_ranges = self
            .get_destination_ranges(pool)
            .await?
            .into_iter()
            .map(Into::into)
            .collect();

        Ok(AclAliasInfo {
            id: self.id,
            name: self.name.clone(),
            destination: self.destination.clone(),
            ports: self.ports.clone().into_iter().map(Into::into).collect(),
            protocols: self.protocols.clone(),
            created_at: self.created_at,
            destination_ranges,
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

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleDevice<I = NoId> {
    pub id: I,
    pub rule_id: i64,
    pub device_id: i64,
    pub allow: bool,
}

#[derive(Clone, Debug, Model, PartialEq, Serialize, Deserialize)]
pub struct AclRuleDestinationRange<I = NoId> {
    pub id: I,
    pub rule_id: i64,
    pub start: IpNetwork,
    pub end: IpNetwork,
}

#[derive(Clone, Debug, Model, PartialEq, Serialize, Deserialize)]
pub struct AclAliasDestinationRange<I = NoId> {
    pub id: I,
    pub alias_id: i64,
    pub start: IpNetwork,
    pub end: IpNetwork,
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
        let alias = AclAlias::new("alias", destination.clone(), ports.clone(), vec![20, 30])
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
        // create the rule
        let mut rule = AclRule::new(
            "rule",
            false,
            false,
            false,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            None,
        )
        .save(&pool)
        .await
        .unwrap();

        // create 2 networks
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

        // rule only applied to network1
        let _rn = AclRuleNetwork {
            id: NoId,
            rule_id: rule.id,
            network_id: network1.id,
        }
        .save(&pool)
        .await
        .unwrap();

        // create 2 users
        let mut user1 = User::new("user1", None, "", "", "u1@mail.com", None)
            .save(&pool)
            .await
            .unwrap();
        let user2 = User::new("user2", None, "", "", "u2@mail.com", None)
            .save(&pool)
            .await
            .unwrap();

        // user1 allowed
        let _ru1 = AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user1.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();

        // user2 denied
        let mut ru2 = AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user2.id,
            allow: false,
        }
        .save(&pool)
        .await
        .unwrap();

        // create 2 grups
        let group1 = Group::new("group1").save(&pool).await.unwrap();
        let group2 = Group::new("group2").save(&pool).await.unwrap();

        // group1 allowed
        let _rg = AclRuleGroup {
            id: NoId,
            rule_id: rule.id,
            group_id: group1.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();

        // group2 denied
        let _rg = AclRuleGroup {
            id: NoId,
            rule_id: rule.id,
            group_id: group2.id,
            allow: false,
        }
        .save(&pool)
        .await
        .unwrap();

        // create 2 devices
        let device1 = Device::new(
            "device1".to_string(),
            String::new(),
            1,
            DeviceType::Network,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();
        let device2 = Device::new(
            "device2".to_string(),
            String::new(),
            1,
            DeviceType::Network,
            None,
            true,
        )
        .save(&pool)
        .await
        .unwrap();

        // device1 allowed
        let _rd = AclRuleDevice {
            id: NoId,
            rule_id: rule.id,
            device_id: device1.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();

        // device2 denied
        let _rd = AclRuleDevice {
            id: NoId,
            rule_id: rule.id,
            device_id: device2.id,
            allow: false,
        }
        .save(&pool)
        .await
        .unwrap();

        // create 2 aliases
        let alias1 = AclAlias::new("alias1", Vec::new(), Vec::new(), Vec::new())
            .save(&pool)
            .await
            .unwrap();
        let _alias2 = AclAlias::new("alias2", Vec::new(), Vec::new(), Vec::new())
            .save(&pool)
            .await
            .unwrap();

        // only alias1 applies to the rule
        let _ra = AclRuleAlias {
            id: NoId,
            rule_id: rule.id,
            alias_id: alias1.id,
        }
        .save(&pool)
        .await
        .unwrap();

        // convert to [`AclRuleInfo`] and verify results
        let info = rule.to_info(&pool).await.unwrap();

        assert_eq!(info.aliases.len(), 1);
        assert_eq!(info.aliases[0].id, alias1.id); // db modifies datetime precision

        assert_eq!(info.allowed_users.len(), 1);
        assert_eq!(info.allowed_users[0], user1);

        assert_eq!(info.denied_users.len(), 1);
        assert_eq!(info.denied_users[0], user2);

        assert_eq!(info.allowed_groups.len(), 1);
        assert_eq!(info.allowed_groups[0], group1);

        assert_eq!(info.denied_groups.len(), 1);
        assert_eq!(info.denied_groups[0], group2);

        assert_eq!(info.allowed_devices.len(), 1);
        assert_eq!(info.allowed_devices[0].id, device1.id); // db modifies datetime precision

        assert_eq!(info.denied_devices.len(), 1);
        assert_eq!(info.denied_devices[0].id, device2.id); // db modifies datetime precision

        assert_eq!(info.networks.len(), 1);
        assert_eq!(info.networks[0], network1);

        // test all_networks flag
        rule.all_networks = true;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_networks(&pool).await.unwrap().len(), 2);

        // test allowed/denied users
        let allowed_users = rule.get_users(&pool, true).await.unwrap();
        let denied_users = rule.get_users(&pool, false).await.unwrap();
        assert_eq!(allowed_users.len(), 1);
        assert_eq!(allowed_users[0], user1);
        assert_eq!(denied_users.len(), 1);
        assert_eq!(denied_users[0], user2);

        // test `allow_all_users` flag
        rule.allow_all_users = true;
        rule.deny_all_users = false;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_users(&pool, true).await.unwrap().len(), 2);
        assert_eq!(rule.get_users(&pool, false).await.unwrap().len(), 0);

        // test `deny_all_users` flag
        rule.allow_all_users = false;
        rule.deny_all_users = true;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_users(&pool, true).await.unwrap().len(), 0);
        assert_eq!(rule.get_users(&pool, false).await.unwrap().len(), 2);

        // TODO: what if both `allow_all_users` and `deny_all_users` are true?

        // deactivate user1
        user1.is_active = false;
        user1.save(&pool).await.unwrap();

        // ensure only active users are allowed when `allow_all_users = true`
        rule.allow_all_users = true;
        rule.deny_all_users = false;
        rule.save(&pool).await.unwrap();

        let allowed_users = rule.get_users(&pool, true).await.unwrap();
        let denied_users = rule.get_users(&pool, false).await.unwrap();
        assert_eq!(allowed_users.len(), 1);
        assert_eq!(allowed_users[0], user2);
        assert_eq!(denied_users.len(), 0);

        // ensure only active users are allowed when `allow_all_users = false`
        rule.allow_all_users = false;
        rule.deny_all_users = false;
        rule.save(&pool).await.unwrap();
        ru2.allow = true; // allow user2
        ru2.save(&pool).await.unwrap();
        let allowed_users = rule.get_users(&pool, true).await.unwrap();
        let denied_users = rule.get_users(&pool, false).await.unwrap();
        assert_eq!(allowed_users.len(), 1);
        assert_eq!(allowed_users[0], user2);
        assert_eq!(denied_users.len(), 0);

        // ensure only active users are denied when `deny_all_users = true`
        rule.allow_all_users = false;
        rule.deny_all_users = true;
        rule.save(&pool).await.unwrap();

        let allowed_users = rule.get_users(&pool, true).await.unwrap();
        let denied_users = rule.get_users(&pool, false).await.unwrap();
        assert_eq!(allowed_users.len(), 0);
        assert_eq!(denied_users.len(), 1);
        assert_eq!(denied_users[0], user2);

        // ensure only active users are denied when `deny_all_users = false`
        rule.allow_all_users = false;
        rule.deny_all_users = false;
        rule.save(&pool).await.unwrap();
        ru2.allow = false; // deny user2
        ru2.save(&pool).await.unwrap();
        let allowed_users = rule.get_users(&pool, true).await.unwrap();
        let denied_users = rule.get_users(&pool, false).await.unwrap();
        assert_eq!(allowed_users.len(), 0);
        assert_eq!(denied_users.len(), 1);
        assert_eq!(denied_users[0], user2);
    }
}
