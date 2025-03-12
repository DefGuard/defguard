use crate::{
    db::{Device, Group, Id, NoId, User, WireguardNetwork},
    enterprise::handlers::acl::{ApiAclAlias, ApiAclRule},
    DeviceType,
};
use chrono::NaiveDateTime;
use ipnetwork::{IpNetwork, IpNetworkError};
use model_derive::Model;
use sqlx::{
    error::ErrorKind, postgres::types::PgRange, query, query_as, Error as SqlxError, FromRow,
    PgConnection, PgExecutor, PgPool, Type,
};
use std::{
    collections::HashSet,
    fmt,
    net::{IpAddr, Ipv4Addr},
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
    AddrParseError(#[from] std::net::AddrParseError),
    #[error(transparent)]
    DbError(#[from] SqlxError),
    #[error("InvalidRelationError: {0}")]
    InvalidRelationError(String),
    #[error("RuleNotFoundError: {0}")]
    RuleNotFoundError(Id),
}

/// https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/include/uapi/linux/in.h
pub type Protocol = i32;

/// Representation of port range. Those are stored in the db as [`PgRange<i32>`].
/// Single ports are represented as single-element ranges, e.g. port 80 = Range(80, 81)
/// since upper bound is excluded by convention.
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

impl PortRange {
    pub fn new(start: i32, end: i32) -> Self {
        Self(start..(end + 1))
    }

    // Returns first port in range
    pub fn first_port(&self) -> i32 {
        self.0.start
    }

    // Returns last port in range
    pub fn last_port(&self) -> i32 {
        self.0.end - 1
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
        Self(start..end)
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

/// ACL rule can be in one of the following states:
/// - New: the rule has been created and not yet applied on the gateway
/// - Modified: the rule has been modified and not yet applied
/// - Deleted: the rule has been marked for deletion but not yed removed from the gateway
/// - Applied: the rule was applied on the gateways
#[derive(Clone, Debug, Default, Deserialize, Serialize, Type, PartialEq, Eq)]
#[sqlx(type_name = "aclrule_state", rename_all = "lowercase")]
pub enum RuleState {
    #[default]
    New,
    Modified,
    Deleted,
    Applied,
}

/// Helper struct combining all DB objects related to given [`AclRule`].
/// All related objects are stored in vectors.
#[derive(Clone, Debug)]
pub struct AclRuleInfo<I = NoId> {
    pub id: I,
    pub parent_id: Option<Id>,
    pub state: RuleState,
    pub name: String,
    pub all_networks: bool,
    pub networks: Vec<WireguardNetwork<Id>>,
    pub expires: Option<NaiveDateTime>,
    pub enabled: bool,
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
    /// Constructs a [`String`] of comma-separated addresses and address ranges
    pub(crate) fn format_destination(&self) -> String {
        // process single addresses
        let addrs = match &self.destination {
            d if d.is_empty() => String::new(),
            d => d.iter().map(|a| a.to_string() + ", ").collect::<String>(),
        };
        // process address ranges
        let ranges = match &self.destination_ranges {
            r if r.is_empty() => String::new(),
            r => r.iter().fold(String::new(), |acc, r| {
                acc + &format!("{}-{}, ", r.start, r.end)
            }),
        };

        // remove full mask from resulting string
        let destination = (addrs + &ranges).replace("/32", "");
        if destination.is_empty() {
            destination
        } else {
            // trim the last last ', '
            destination[..destination.len() - 2].to_string()
        }
    }

    /// Constructs a [`String`] of comma-separated ports and port ranges
    pub(crate) fn format_ports(&self) -> String {
        if self.ports.is_empty() {
            String::new()
        } else {
            let ports = self
                .ports
                .iter()
                .map(|r| r.to_string() + ", ")
                .collect::<String>();
            // trim the last last ', '
            ports[..ports.len() - 2].to_string()
        }
    }
}

/// Database representation of an ACL rule. ACL rule has many related objects:
/// * networks
/// * users
/// * groups
/// * aliases
/// * devices
/// * ...
///
/// Those objects have their dedicated tables and structures so we provide
/// [`AclRuleInfo`] and [`ApiAclRule`] structs that implement appropriate methods
/// to combine all the related objects for easier downstream processing.
#[derive(Clone, Debug, Model, PartialEq, Eq, FromRow)]
pub struct AclRule<I = NoId> {
    pub id: I,
    // if present points to the original rule before modification / deletion
    pub parent_id: Option<Id>,
    #[model(enum)]
    pub state: RuleState,
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
    pub enabled: bool,
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
        let rule: AclRule = api_rule.clone().try_into()?;
        let rule = rule.save(&mut *transaction).await?;

        // create related objects
        Self::create_related_objects(&mut transaction, rule.id, api_rule).await?;

        transaction.commit().await?;
        Ok(rule.to_info(pool).await?.into())
    }

    /// Updates [`AclRule`] with all it's related objects based on [`ApiAclRule`]
    ///
    /// State handling:
    ///
    /// - For rules in `RuleState::Applied` state (rules that are currently active):
    ///   1. Any existing modifications of this rule are deleted
    ///   2. A copy of the rule is created with `RuleState::Modified` state and the original rule as parent
    /// - For rules in other states (`New`, `Modified` or `Deleted`), we directly update the existing rule
    ///   since they haven't been applied to the gateways yet.
    ///
    /// This approach allows us to track changes to applied rules while maintaining their history.
    pub(crate) async fn update_from_api(
        pool: &PgPool,
        id: Id,
        api_rule: &ApiAclRule<Id>,
    ) -> Result<ApiAclRule<Id>, AclError> {
        let mut transaction = pool.begin().await?;

        // find the existing rule
        let existing_rule = AclRule::find_by_id(&mut *transaction, id)
            .await?
            .ok_or_else(|| {
                warn!("Update of nonexistent rule ({id}) failed");
                AclError::RuleNotFoundError(id)
            })?;

        // convert API rule to model
        let mut rule: AclRule<Id> = api_rule.clone().try_into()?;

        // perform appropriate updates depending on existing rule's state
        match existing_rule.state {
            RuleState::Applied => {
                // create new `RuleState::Modified` rule
                // remove old modifications of this rule
                query!("DELETE FROM aclrule WHERE parent_id = $1", id)
                    .execute(&mut *transaction)
                    .await?;

                // save as a new rule with appropriate parent_id and state
                let mut rule = rule.as_noid();
                rule.state = RuleState::Modified;
                rule.parent_id = Some(id);
                let rule = rule.save(&mut *transaction).await?;

                // create related objects
                AclRule::create_related_objects(&mut transaction, rule.id, api_rule).await?;
            }
            _ => {
                // update the not-yet applied modification itself
                rule.id = id; // frontend may PUT an object with incorrect id
                rule.save(&mut *transaction).await?;

                // recreate related objects
                Self::delete_related_objects(&mut transaction, rule.id).await?;
                AclRule::create_related_objects(&mut transaction, rule.id, api_rule).await?;
            }
        };

        transaction.commit().await?;
        Ok(api_rule.clone())
    }

    /// Deletes [`AclRule`] with all it's related objects.
    ///
    /// State handling:
    ///
    /// - For rules in `RuleState::Applied` state (rules that are currently active):
    ///   1. Any existing modifications of this rule are deleted
    ///   2. A copy of the rule is created with `RuleState::Deleted` state and the original rule as parent
    /// 
    /// This preserves the original rule while tracking the deletion.
    ///
    /// - For rules in other states (`New`, `Modified` or `Deleted`):
    ///   1. All related objects are deleted
    ///   2. The rule itself is deleted from the database
    /// 
    /// Since these rules were not yet applied, we can safely remove them.
    pub(crate) async fn delete_from_api(pool: &PgPool, id: Id) -> Result<(), AclError> {
        let mut transaction = pool.begin().await?;

        // find the existing rule
        let existing_rule = AclRule::find_by_id(&mut *transaction, id)
            .await?
            .ok_or_else(|| {
                warn!("Deletion of nonexistent rule ({id}) failed");
                AclError::RuleNotFoundError(id)
            })?;

        // perform appropriate modifications depending on existing rule's state
        match existing_rule.state {
            RuleState::Applied => {
                // create new `RuleState::Modified` rule
                // delete all modifications of this rule
                query!("DELETE FROM aclrule WHERE parent_id = $1", id)
                    .execute(&mut *transaction)
                    .await?;

                // save as a new rule with appropriate parent_id and state
                let mut rule = existing_rule.as_noid();
                rule.state = RuleState::Deleted;
                rule.parent_id = Some(id);
                rule.save(&mut *transaction).await?;
            }
            _ => {
                // delete the not-yet applied modification itself
                // delete related objects
                Self::delete_related_objects(&mut transaction, id).await?;

                // delete the rule
                existing_rule.delete(&mut *transaction).await?;
            }
        };

        transaction.commit().await?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct ParsedDestination {
    addrs: Vec<IpNetwork>,
    ranges: Vec<(IpAddr, IpAddr)>,
}

/// Perses a destination string into singular ip addresses or networks and address
/// ranges. We should be able to parse a string like this one:
/// `10.0.0.1/24, 10.1.1.10-10.1.1.20, 192.168.1.10, 10.1.1.1-10.10.1.1`
pub fn parse_destination(destination: &str) -> Result<ParsedDestination, AclError> {
    debug!("Parsing destination string: {destination}");
    let destination: String = destination.chars().filter(|c| !c.is_whitespace()).collect();
    let mut result = ParsedDestination::default();
    if destination.is_empty() {
        return Ok(result);
    }
    for v in destination.split(',') {
        match v.split('-').collect::<Vec<_>>() {
            l if l.len() == 1 => result.addrs.push(l[0].parse::<IpNetwork>()?),
            l if l.len() == 2 => result
                .ranges
                .push((l[0].parse::<IpAddr>()?, l[1].parse::<IpAddr>()?)),
            _ => {
                error!("Failed to parse destination string: \"{destination}\"");
                return Err(IpNetworkError::InvalidAddr(destination))?;
            }
        };
    }

    debug!("Parsed destination: {result:?}");
    Ok(result)
}

/// Perses a ports string into singular ports and port ranges
/// We should be able to parse a string like this one:
/// `22, 23, 8000-9000, 80-90`
pub fn parse_ports(ports: &str) -> Result<Vec<PortRange>, AclError> {
    debug!("Parsing ports string: {ports}");
    let mut result = Vec::new();
    let ports: String = ports.chars().filter(|c| !c.is_whitespace()).collect();
    if ports.is_empty() {
        return Ok(result);
    }
    for v in ports.split(',') {
        match v.split('-').collect::<Vec<_>>() {
            l if l.len() == 1 => result.push(PortRange(Range {
                start: l[0].parse::<i32>()?,
                end: l[0].parse::<i32>()? + 1,
            })),
            l if l.len() == 2 => result.push(PortRange(Range {
                start: l[0].parse::<i32>()?,
                end: l[1].parse::<i32>()? + 1,
            })),
            _ => {
                error!("Failed to parse ports string: \"{ports}\"");
                return Err(AclError::InvalidPortsFormat(ports.to_string()));
            }
        };
    }

    debug!("Parsed ports: {result:?}");
    Ok(result)
}

/// Maps [`sqlx::Error`] to [`AclError`] while checking for [`ErrorKind::ForeignKeyViolation`].
fn map_relation_error(err: SqlxError, class: &str, id: &Id) -> AclError {
    if let SqlxError::Database(dberror) = &err {
        if dberror.kind() == ErrorKind::ForeignKeyViolation {
            error!("Failed to create ACL related object, foreign key violation: {class}({id}): {dberror}");
            return AclError::InvalidRelationError(format!("{class}({id})"));
        }
    }
    error!("Failed to create ACL related object: {err}");
    AclError::DbError(err)
}

impl<I: std::fmt::Debug> AclRule<I> {
    /// Creates relation objects for given [`AclRule`] based on [`ApiAclRule`] object
    async fn create_related_objects(
        transaction: &mut PgConnection,
        rule_id: Id,
        api_rule: &ApiAclRule<I>,
    ) -> Result<(), AclError> {
        debug!("Creating related objects for ACL rule {api_rule:?}");
        // save related networks
        for network_id in &api_rule.networks {
            let obj = AclRuleNetwork {
                id: NoId,
                rule_id,
                network_id: *network_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "WireguardNetwork", network_id))?;
        }

        // allowed users
        for user_id in &api_rule.allowed_users {
            let obj = AclRuleUser {
                id: NoId,
                allow: true,
                rule_id,
                user_id: *user_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "User", user_id))?;
        }

        // denied users
        for user_id in &api_rule.denied_users {
            let obj = AclRuleUser {
                id: NoId,
                allow: false,
                rule_id,
                user_id: *user_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "User", user_id))?;
        }

        // allowed groups
        for group_id in &api_rule.allowed_groups {
            let obj = AclRuleGroup {
                id: NoId,
                allow: true,
                rule_id,
                group_id: *group_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "Group", group_id))?;
        }

        // denied groups
        for group_id in &api_rule.denied_groups {
            let obj = AclRuleGroup {
                id: NoId,
                allow: false,
                rule_id,
                group_id: *group_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "Group", group_id))?;
        }

        // save related aliases
        for alias_id in &api_rule.aliases {
            let obj = AclRuleAlias {
                id: NoId,
                rule_id,
                alias_id: *alias_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "AclAlias", alias_id))?;
        }

        // allowed devices
        for device_id in &api_rule.allowed_devices {
            let obj = AclRuleDevice {
                id: NoId,
                allow: true,
                rule_id,
                device_id: *device_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "Device", device_id))?;
        }

        // denied devices
        for device_id in &api_rule.denied_devices {
            let obj = AclRuleDevice {
                id: NoId,
                allow: false,
                rule_id,
                device_id: *device_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "Device", device_id))?;
        }

        // destination
        let destination = parse_destination(&api_rule.destination)?;
        for range in destination.ranges {
            let obj = AclRuleDestinationRange {
                id: NoId,
                rule_id,
                start: range.0,
                end: range.1,
            };
            obj.save(&mut *transaction).await?;
        }

        info!("Created related objects for ACL rule {api_rule:?}");
        Ok(())
    }

    /// Deletes relation objects for given [`AclRule`]
    async fn delete_related_objects(
        transaction: &mut PgConnection,
        rule_id: Id,
    ) -> Result<(), SqlxError> {
        debug!("Deleting related objects for ACL rule {rule_id}");
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

        info!("Deleted related objects for ACL rule {rule_id}");
        Ok(())
    }
}

impl<I> TryFrom<ApiAclRule<I>> for AclRule<I> {
    type Error = AclError;
    fn try_from(rule: ApiAclRule<I>) -> Result<Self, Self::Error> {
        Ok(Self {
            destination: parse_destination(&rule.destination)?.addrs,
            ports: parse_ports(&rule.ports)?
                .into_iter()
                .map(Into::into)
                .collect(),
            id: rule.id,
            parent_id: rule.parent_id,
            state: rule.state,
            name: rule.name,
            allow_all_users: rule.allow_all_users,
            deny_all_users: rule.deny_all_users,
            all_networks: rule.all_networks,
            protocols: rule.protocols,
            enabled: rule.enabled,
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
                connected_at, mfa_enabled, keepalive_interval, peer_disconnect_threshold, \
                acl_enabled, acl_default_allow \
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
    pub(crate) async fn get_users<'e, E>(
        &self,
        executor: E,
        allowed: bool,
    ) -> Result<Vec<User<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        Ok(match allowed {
            true => self.get_allowed_users(executor).await?,
            false => self.get_denied_users(executor).await?,
        })
    }

    /// Returns **active** [`User`]s that are allowed by the rule
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

    /// Returns **active** [`User`]s that are denied by the rule
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
            "SELECT id, rule_id, \"start\" \"start: IpAddr\", \"end\" \"end: IpAddr\" \
            FROM aclruledestinationrange \
            WHERE rule_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    /// Retrieves all related objects from the db and converts [`AclRule`]
    /// instance to [`AclRuleInfo`].
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
            parent_id: self.parent_id,
            state: self.state.clone(),
            name: self.name.clone(),
            allow_all_users: self.allow_all_users,
            deny_all_users: self.deny_all_users,
            all_networks: self.all_networks,
            destination: self.destination.clone(),
            protocols: self.protocols.clone(),
            enabled: self.enabled,
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

impl AclRuleInfo<Id> {
    /// Wrapper function which combines explicitly specified allowed users with members of allowed
    /// groups to generate a list of all unique allowed users for a given ACL.
    pub(crate) async fn get_all_allowed_users(
        &self,
        pool: &PgPool,
    ) -> Result<Vec<User<Id>>, SqlxError> {
        // get explicitly allowed users
        let mut allowed_users = self.allowed_users.clone();

        // get allowed groups IDs
        let allowed_group_ids: Vec<Id> = self.allowed_groups.iter().map(|group| group.id).collect();

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
        // get explicitly denied users
        let mut denied_users = self.denied_users.clone();

        // get denied groups IDs
        let denied_group_ids: Vec<Id> = self.denied_groups.iter().map(|group| group.id).collect();

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
}

/// Helper struct combining all DB objects related to given [`AclAlias`].
/// All related objects are stored in vectors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AclAliasInfo<I = NoId> {
    pub id: I,
    pub name: String,
    pub destination: Vec<IpNetwork>,
    pub destination_ranges: Vec<AclAliasDestinationRange<Id>>,
    pub ports: Vec<PortRange>,
    pub protocols: Vec<Protocol>,
}

impl<I> AclAliasInfo<I> {
    /// Constructs a [`String`] of comma-separated addresses and address ranges
    pub fn format_destination(&self) -> String {
        // process single addresses
        let addrs = match &self.destination {
            d if d.is_empty() => String::new(),
            d => d.iter().map(|a| a.to_string() + ", ").collect::<String>(),
        };
        // process address ranges
        let ranges = match &self.destination_ranges {
            r if r.is_empty() => String::new(),
            r => r.iter().fold(String::new(), |acc, r| {
                acc + &format!("{}-{}, ", r.start, r.end)
            }),
        };

        // remove full mask from resulting string
        let destination = (addrs + &ranges).replace("/32", "");
        if destination.is_empty() {
            destination
        } else {
            // trim the last last ', '
            destination[..destination.len() - 2].to_string()
        }
    }

    /// Constructs a [`String`] of comma-separated ports and port ranges
    pub fn format_ports(&self) -> String {
        if self.ports.is_empty() {
            String::new()
        } else {
            let ports = self
                .ports
                .iter()
                .map(|r| r.to_string() + ", ")
                .collect::<String>();
            // trim the last last ', '
            ports[..ports.len() - 2].to_string()
        }
    }
}

impl<I> TryFrom<ApiAclAlias<I>> for AclAlias<I> {
    type Error = AclError;
    fn try_from(alias: ApiAclAlias<I>) -> Result<Self, Self::Error> {
        Ok(Self {
            destination: parse_destination(&alias.destination)?.addrs,
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

/// Database representation of an ACL alias. Aliases can be used to define
/// the destination part of an ACL rule so that it's easier to create new
/// rules with common restrictions. In addition to the [`AclAlias`] we provide
/// [`AclAliasInfo`] and [`ApiAclAlias`] that combine all related objects for
/// easier downstream processing.
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

impl<I: std::fmt::Debug> AclAlias<I> {
    /// Creates relation objects for given [`AclAlias`] based on [`AclAliasInfo`] object
    async fn create_related_objects(
        transaction: &mut PgConnection,
        alias_id: Id,
        api_alias: &ApiAclAlias<I>,
    ) -> Result<(), AclError> {
        debug!("Creating related objects for ACL alias {api_alias:?}");
        // save related destination ranges
        let destination = parse_destination(&api_alias.destination)?;
        for range in destination.ranges {
            let obj = AclAliasDestinationRange {
                id: NoId,
                alias_id,
                start: range.0,
                end: range.1,
            };
            obj.save(&mut *transaction).await?;
        }

        info!("Created related objects for ACL alias {api_alias:?}");
        Ok(())
    }

    /// Deletes relation objects for given [`AclAlias`]
    async fn delete_related_objects(
        transaction: &mut PgConnection,
        alias_id: Id,
    ) -> Result<(), AclError> {
        debug!("Deleting related objects for ACL alias {alias_id}");
        // destination ranges
        query!(
            "DELETE FROM aclaliasdestinationrange WHERE alias_id = $1",
            alias_id
        )
        .execute(&mut *transaction)
        .await?;

        info!("Deleted related objects for ACL alias {alias_id}");
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
            "SELECT id, alias_id, \"start\" \"start: IpAddr\", \"end\" \"end: IpAddr\" \
            FROM aclaliasdestinationrange \
            WHERE alias_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    /// Retrieves all related objects from the db and converts [`AclAlias`]
    /// instance to [`AclAliasInfo`].
    pub(crate) async fn to_info(&self, pool: &PgPool) -> Result<AclAliasInfo<Id>, SqlxError> {
        let destination_ranges = self.get_destination_ranges(pool).await?;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AclRuleDestinationRange<I = NoId> {
    pub id: I,
    pub rule_id: Id,
    pub start: IpAddr,
    pub end: IpAddr,
}

impl Default for AclRuleDestinationRange<Id> {
    fn default() -> Self {
        Self {
            id: Id::default(),
            rule_id: Id::default(),
            start: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            end: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        }
    }
}

impl AclRuleDestinationRange<NoId> {
    pub async fn save<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO aclruledestinationrange \
            (rule_id, \"start\", \"end\") \
            VALUES ($1, $2, $3)",
            self.rule_id,
            IpNetwork::from(self.start),
            IpNetwork::from(self.end),
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AclAliasDestinationRange<I = NoId> {
    pub id: I,
    pub alias_id: Id,
    pub start: IpAddr,
    pub end: IpAddr,
}

impl Default for AclAliasDestinationRange<Id> {
    fn default() -> Self {
        Self {
            id: Id::default(),
            alias_id: Id::default(),
            start: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            end: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        }
    }
}

impl AclAliasDestinationRange<NoId> {
    pub async fn save<'e, E>(&self, executor: E) -> Result<(), SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query!(
            "INSERT INTO aclaliasdestinationrange \
            (alias_id, \"start\", \"end\") \
            VALUES ($1, $2, $3)",
            self.alias_id,
            IpNetwork::from(self.start),
            IpNetwork::from(self.end),
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use rand::{thread_rng, Rng};

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
            parent_id: Default::default(),
            state: Default::default(),
            name: "rule".to_string(),
            enabled: true,
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
            false,
            false,
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
            false,
            false,
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
        assert_eq!(info.aliases[0], alias1);

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
        assert_eq!(rule.get_users(&pool, false).await.unwrap().len(), 1);

        // test `deny_all_users` flag
        rule.allow_all_users = false;
        rule.deny_all_users = true;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_users(&pool, true).await.unwrap().len(), 1);
        assert_eq!(rule.get_users(&pool, false).await.unwrap().len(), 2);

        // test both flags
        rule.allow_all_users = true;
        rule.deny_all_users = true;
        rule.save(&pool).await.unwrap();
        assert_eq!(rule.get_users(&pool, true).await.unwrap().len(), 2);
        assert_eq!(rule.get_users(&pool, false).await.unwrap().len(), 2);

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
        assert_eq!(denied_users.len(), 1);

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
        assert_eq!(allowed_users.len(), 1);
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

    #[sqlx::test]
    async fn test_all_allowed_users(pool: PgPool) {
        let mut rng = thread_rng();

        // Create test users
        let user_1: User<NoId> = rng.gen();
        let user_1 = user_1.save(&pool).await.unwrap();
        let user_2: User<NoId> = rng.gen();
        let user_2 = user_2.save(&pool).await.unwrap();
        let user_3: User<NoId> = rng.gen();
        let user_3 = user_3.save(&pool).await.unwrap();
        // inactive user
        let mut user_4: User<NoId> = rng.gen();
        user_4.is_active = false;
        let user_4 = user_4.save(&pool).await.unwrap();

        // Create test groups
        let group_1 = Group {
            id: NoId,
            name: "group_1".into(),
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();
        let group_2 = Group {
            id: NoId,
            name: "group_2".into(),
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();

        // Assign users to groups:
        // Group 1: users 1,2
        // Group 2: user 3,4
        let group_assignments = vec![
            (&group_1, vec![&user_1, &user_2]),
            (&group_2, vec![&user_3, &user_4]),
        ];

        for (group, users) in group_assignments {
            for user in users {
                query!(
                    "INSERT INTO group_user (user_id, group_id) VALUES ($1, $2)",
                    user.id,
                    group.id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
        }

        // Create ACL rule
        let rule = AclRule {
            id: NoId,
            name: "test_rule".to_string(),
            allow_all_users: false,
            deny_all_users: false,
            all_networks: false,
            destination: Vec::new(),
            ports: Vec::new(),
            protocols: Vec::new(),
            expires: None,
            enabled: true,
        }
        .save(&pool)
        .await
        .unwrap();

        // Allow user_1 explicitly and group_2
        AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user_1.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();

        AclRuleGroup {
            id: NoId,
            rule_id: rule.id,
            group_id: group_2.id,
            allow: true,
        }
        .save(&pool)
        .await
        .unwrap();

        // Get rule info
        let rule_info = rule.to_info(&pool).await.unwrap();
        assert_eq!(rule_info.allowed_users.len(), 1);
        assert_eq!(rule_info.allowed_groups.len(), 1);

        // Get all allowed users
        let allowed_users = rule_info.get_all_allowed_users(&pool).await.unwrap();

        // Should contain user1 (explicit) and user3 (from group2), but not inactive user_4
        assert_eq!(allowed_users.len(), 2);
        assert!(allowed_users.iter().any(|u| u.id == user_1.id));
        assert!(allowed_users.iter().any(|u| u.id == user_3.id));
        assert!(!allowed_users.iter().any(|u| u.id == user_4.id));
    }

    #[sqlx::test]
    async fn test_all_denied_users(pool: PgPool) {
        let mut rng = thread_rng();

        // Create test users
        let user_1: User<NoId> = rng.gen();
        let user_1 = user_1.save(&pool).await.unwrap();
        let user_2: User<NoId> = rng.gen();
        let user_2 = user_2.save(&pool).await.unwrap();
        let user_3: User<NoId> = rng.gen();
        let user_3 = user_3.save(&pool).await.unwrap();
        // inactive user
        let mut user_4: User<NoId> = rng.gen();
        user_4.is_active = false;
        let user_4 = user_4.save(&pool).await.unwrap();

        // Create test groups
        let group_1 = Group {
            id: NoId,
            name: "group_1".into(),
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();
        let group_2 = Group {
            id: NoId,
            name: "group_2".into(),
            ..Default::default()
        }
        .save(&pool)
        .await
        .unwrap();

        // Assign users to groups:
        // Group 1: users 2,3,4
        // Group 2: user 1
        let group_assignments = vec![
            (&group_1, vec![&user_2, &user_3, &user_4]),
            (&group_2, vec![&user_1]),
        ];

        for (group, users) in group_assignments {
            for user in users {
                query!(
                    "INSERT INTO group_user (user_id, group_id) VALUES ($1, $2)",
                    user.id,
                    group.id
                )
                .execute(&pool)
                .await
                .unwrap();
            }
        }

        // Create ACL rule
        let rule = AclRule {
            id: NoId,
            name: "test_rule".to_string(),
            allow_all_users: false,
            deny_all_users: false,
            all_networks: false,
            destination: Vec::new(),
            ports: Vec::new(),
            protocols: Vec::new(),
            expires: None,
            enabled: true,
        }
        .save(&pool)
        .await
        .unwrap();

        // Deny user_1, user_3 explicitly and group_1
        AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user_1.id,
            allow: false,
        }
        .save(&pool)
        .await
        .unwrap();
        AclRuleUser {
            id: NoId,
            rule_id: rule.id,
            user_id: user_3.id,
            allow: false,
        }
        .save(&pool)
        .await
        .unwrap();

        AclRuleGroup {
            id: NoId,
            rule_id: rule.id,
            group_id: group_1.id,
            allow: false,
        }
        .save(&pool)
        .await
        .unwrap();

        // Get rule info
        let rule_info = rule.to_info(&pool).await.unwrap();
        assert_eq!(rule_info.denied_users.len(), 2);
        assert_eq!(rule_info.denied_groups.len(), 1);

        // Get all denied users
        let denied_users = rule_info.get_all_denied_users(&pool).await.unwrap();

        // Should contain user_1 (explicit), user_2 and user_3 (from group_1), but not inactive user_4
        assert_eq!(denied_users.len(), 3);
        assert!(denied_users.iter().any(|u| u.id == user_1.id));
        assert!(denied_users.iter().any(|u| u.id == user_2.id));
        assert!(denied_users.iter().any(|u| u.id == user_3.id));
        assert!(!denied_users.iter().any(|u| u.id == user_4.id));
    }
}
