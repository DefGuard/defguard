use crate::{
    db::{Device, Group, Id, NoId, User, WireguardNetwork},
    enterprise::handlers::acl::{ApiAclAlias, ApiAclRule},
    DeviceType,
};
use chrono::NaiveDateTime;
use ipnetwork::{IpNetwork, IpNetworkError};
use model_derive::Model;
use sqlx::{
    postgres::types::PgRange, query, query_as, Error as SqlxError, PgConnection, PgExecutor, PgPool,
};
use std::{
    collections::HashSet,
    fmt,
    ops::{Bound, Range},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AclError {
    #[error("InvalidPortsFormat: {0}")]
    InvalidPortsFormat(String),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    IpNetworkError(#[from] ipnetwork::IpNetworkError),
    #[error(transparent)]
    DbError(#[from] SqlxError),
}

/// https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/include/uapi/linux/in.h
pub type Protocol = i32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortRange(pub Range<i32>);

impl fmt::Display for PortRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match (self.0.start, self.0.end) {
            (start, end) if end == (start + 1) => start.to_string(),
            (start, end) => format!("{start}-{}", end - 1),
        };
        write!(f, "{}", s)
    }
}

impl From<PgRange<i32>> for PortRange {
    fn from(range: PgRange<i32>) -> Self {
        let start = match range.start {
            Bound::Included(start) => start,
            Bound::Excluded(start) => start + 1,
            // should not happen - database constraint
            Bound::Unbounded => panic!("Unbounded port range"),
        };
        let end = match range.end {
            Bound::Included(end) => end + 1,
            Bound::Excluded(end) => end,
            // should not happen - database constraint
            Bound::Unbounded => panic!("Unbounded port range"),
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

impl<I> AclRuleInfo<I> {
    pub fn format_destination(&self) -> String {
        let addrs = match &self.destination {
            d if d.is_empty() => String::new(),
            d => d.iter().map(|a| a.to_string() + ", ").collect::<String>(),
        };
        let ranges = match &self.destination_ranges {
            r if r.is_empty() => String::new(),
            r => r.iter().fold(String::new(), |acc, r| {
                acc + &format!("{}-{}, ", r.start, r.end)
            }),
        };

        let destination = (addrs + &ranges).replace("/32", "");
        if destination.is_empty() {
            destination
        } else {
            destination[..destination.len() - 2].to_string()
        }
    }

    pub fn format_ports(&self) -> String {
        if self.ports.is_empty() {
            String::new()
        } else {
            let ports = self
                .ports
                .iter()
                .map(|r| r.to_string() + ", ")
                .collect::<String>();
            ports[..ports.len() - 2].to_string()
        }
    }
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
    /// Creates new [`AclRule`] with all related objects based on [`ApiAclRule`]
    pub(crate) async fn create_from_api(
        pool: &PgPool,
        api_rule: &ApiAclRule<NoId>,
    ) -> Result<ApiAclRule<Id>, AclError> {
        let mut transaction = pool.begin().await?;

        // save the rule
        let rule: AclRule<NoId> = api_rule.clone().try_into()?;
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
    ) -> Result<ApiAclRule<Id>, AclError> {
        let mut transaction = pool.begin().await?;

        // save the rule
        let mut rule: AclRule<Id> = api_rule.clone().try_into()?;
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

pub fn parse_destination(
    destination: &str,
) -> Result<(Vec<IpNetwork>, Vec<(IpNetwork, IpNetwork)>), AclError> {
    let mut addrs = Vec::new();
    let mut ranges = Vec::new();
    let destination: String = destination.chars().filter(|c| !c.is_whitespace()).collect();
    for v in destination.split(',') {
        match v.split('-').collect::<Vec<_>>() {
            l if l.len() == 1 => addrs.push(l[0].parse::<IpNetwork>()?),
            l if l.len() == 2 => {
                ranges.push((l[0].parse::<IpNetwork>()?, l[1].parse::<IpNetwork>()?))
            }
            _ => return Err(IpNetworkError::InvalidAddr(destination))?,
        };
    }

    Ok((addrs, ranges))
}

pub fn parse_ports(ports: &str) -> Result<Vec<PortRange>, AclError> {
    let mut result = Vec::new();
    let p: String = ports.chars().filter(|c| !c.is_whitespace()).collect();
    for v in p.split(',') {
        match v.split('-').collect::<Vec<_>>() {
            l if l.len() == 1 => result.push(PortRange(Range {
                start: l[0].parse::<i32>()?,
                end: l[0].parse::<i32>()? + 1,
            })),
            l if l.len() == 2 => result.push(PortRange(Range {
                start: l[0].parse::<i32>()?,
                end: l[1].parse::<i32>()? + 1,
            })),
            _ => return Err(AclError::InvalidPortsFormat(ports.to_string())),
        };
    }

    Ok(result)
}

impl<I> AclRule<I> {
    /// Creates relation objects for given [`AclRule`] based on [`ApiAclRule`] object
    async fn create_related_objects(
        transaction: &mut PgConnection,
        rule_id: Id,
        api_rule: &ApiAclRule<I>,
    ) -> Result<(), AclError> {
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

        // destination
        let (_, ranges) = parse_destination(&api_rule.destination)?;
        for range in ranges {
            let obj = AclRuleDestinationRange {
                id: NoId,
                rule_id,
                start: range.0,
                end: range.1,
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

impl<I> TryFrom<ApiAclRule<I>> for AclRule<I> {
    type Error = AclError;
    fn try_from(rule: ApiAclRule<I>) -> Result<Self, Self::Error> {
        Ok(Self {
            destination: parse_destination(&rule.destination)?.0,
            ports: parse_ports(&rule.ports)?
                .into_iter()
                .map(Into::into)
                .collect(),
            id: rule.id,
            name: rule.name,
            allow_all_users: rule.allow_all_users,
            deny_all_users: rule.deny_all_users,
            all_networks: rule.all_networks,
            protocols: rule.protocols,
            expires: rule.expires,
        })
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
            "SELECT a.id, name, destination, ports, protocols \
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

    /// Wrapper function which combines explicitly specified allowed users with members of allowed
    /// groups to generate a list of all unique allowed users for a given ACL.
    pub(crate) async fn get_all_allowed_users(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<User<Id>>, SqlxError> {
        // fetch explicitly allowed users
        let mut allowed_users = self.get_allowed_users(pool).await?;

        // fetch allowed groups
        let allowed_groups = self.get_groups(pool, true).await?;
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
        let denied_groups = self.get_groups(pool, false).await?;
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
        let destination_ranges = self.get_destination_ranges(pool).await?;
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
}

impl<I> AclAliasInfo<I> {
    pub fn format_destination(&self) -> String {
        let addrs = match &self.destination {
            d if d.is_empty() => String::new(),
            d => d.iter().map(|a| a.to_string() + ", ").collect::<String>(),
        };
        let ranges = match &self.destination_ranges {
            r if r.is_empty() => String::new(),
            r => r.iter().fold(String::new(), |acc, r| {
                acc + &format!("{}-{}, ", r.start, r.end)
            }),
        };

        let destination = (addrs + &ranges).replace("/32", "");
        if destination.is_empty() {
            destination
        } else {
            destination[..destination.len() - 2].to_string()
        }
    }

    pub fn format_ports(&self) -> String {
        if self.ports.is_empty() {
            String::new()
        } else {
            let ports = self
                .ports
                .iter()
                .map(|r| r.to_string() + ", ")
                .collect::<String>();
            ports[..ports.len() - 2].to_string()
        }
    }
}

impl<I> TryFrom<ApiAclAlias<I>> for AclAlias<I> {
    type Error = AclError;
    fn try_from(alias: ApiAclAlias<I>) -> Result<Self, Self::Error> {
        Ok(Self {
            destination: parse_destination(&alias.destination)?.0,
            ports: parse_ports(&alias.ports)?
                .into_iter()
                .map(Into::into)
                .collect(),
            id: alias.id,
            name: alias.name,
            protocols: alias.protocols,
        })
    }
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
        }
    }

    /// Creates new [`AclAlias`] with all related objects based on [`AclAliasInfo`]
    pub(crate) async fn create_from_api(
        pool: &PgPool,
        api_alias: &ApiAclAlias<NoId>,
    ) -> Result<ApiAclAlias<Id>, AclError> {
        let mut transaction = pool.begin().await?;

        // save the alias
        let alias: AclAlias<NoId> = api_alias.clone().try_into()?;
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
        api_alias: &ApiAclAlias<Id>,
    ) -> Result<ApiAclAlias<Id>, AclError> {
        let mut transaction = pool.begin().await?;

        // save the alias
        let mut alias: AclAlias<Id> = api_alias.clone().try_into()?;
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
    pub(crate) async fn delete_from_api(pool: &PgPool, id: Id) -> Result<(), AclError> {
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
        api_alias: &ApiAclAlias<I>,
    ) -> Result<(), AclError> {
        // save related destination ranges
        let (_, ranges) = parse_destination(&api_alias.destination)?;
        for range in ranges {
            let obj = AclAliasDestinationRange {
                id: NoId,
                alias_id,
                start: range.0,
                end: range.1,
            };
            obj.save(&mut *transaction).await?;
        }

        Ok(())
    }

    /// Deletes relation objects for given [`AclAlias`]
    async fn delete_related_objects(
        transaction: &mut PgConnection,
        alias_id: Id,
    ) -> Result<(), AclError> {
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
        let mut rule = AclRule {
            id: NoId,
            name: "rule".to_string(),
            allow_all_users: false,
            deny_all_users: false,
            all_networks: false,
            destination: Vec::new(),
            ports: Vec::new(),
            protocols: Vec::new(),
            expires: None,
        }
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

    // #[sqlx::test]
    // async fn test_all_allowed_users(pool: PgPool) {
    //     unimplemented!()
    // }

    // #[sqlx::test]
    // async fn test_all_denied_users(pool: PgPool) {
    //     unimplemented!()
    // }
}
