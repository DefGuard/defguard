use std::{
    collections::HashSet,
    fmt,
    net::IpAddr,
    ops::{Bound, RangeInclusive},
};

use chrono::NaiveDateTime;
use ipnetwork::{IpNetwork, IpNetworkError};
use model_derive::Model;
use sqlx::{
    error::ErrorKind, postgres::types::PgRange, query, query_as, query_scalar, Error as SqlxError,
    FromRow, PgConnection, PgExecutor, PgPool, Type,
};
use thiserror::Error;

use crate::{
    appstate::AppState,
    db::{Device, GatewayEvent, Group, Id, NoId, User, WireguardNetwork},
    enterprise::{
        firewall::FirewallError,
        handlers::acl::{ApiAclAlias, ApiAclRule, EditAclAlias, EditAclRule},
    },
    DeviceType,
};

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
    #[error("AliasNotFoundError: {0}")]
    AliasNotFoundError(Id),
    #[error("RuleAlreadyAppliedError: {0}")]
    RuleAlreadyAppliedError(Id),
    #[error("AliasAlreadyAppliedError: {0}")]
    AliasAlreadyAppliedError(Id),
    #[error("AliasUsedByRulesError: {0}")]
    AliasUsedByRulesError(Id),
    #[error(transparent)]
    FirewallError(#[from] FirewallError),
    #[error("InvalidIpRangeError: {0}")]
    InvalidIpRangeError(String),
    #[error("CannotModifyDeletedRuleError: {0}")]
    CannotModifyDeletedRuleError(Id),
    #[error("CannotUseModifiedAliasInRuleError: {0:?}")]
    CannotUseModifiedAliasInRuleError(Vec<Id>),
}

/// https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/include/uapi/linux/in.h
pub type Protocol = i32;

/// Representation of port range. Those are stored in the db as [`PgRange<i32>`].
/// Single ports are represented as single-element ranges, e.g. port 80 = PortRange(80, 80)
/// since upper bound is excluded by convention.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortRange(pub RangeInclusive<u16>);

impl fmt::Display for PortRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match (self.0.start(), self.0.end()) {
            (start, end) if end == start => start.to_string(),
            (start, end) => format!("{start}-{end}"),
        };
        write!(f, "{s}")
    }
}

impl PortRange {
    #[must_use]
    pub fn new(start: u16, end: u16) -> Self {
        Self(start..=end)
    }

    /// Returns first port in range.
    #[must_use]
    pub fn first_port(&self) -> u16 {
        *self.0.start()
    }

    /// Returns last port in range.
    #[must_use]
    pub fn last_port(&self) -> u16 {
        *self.0.end()
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
            Bound::Included(end) => end,
            Bound::Excluded(end) => end - 1,
            // should not happen - database constraint
            Bound::Unbounded => panic!("Unbounded port range"),
        };
        Self(start as u16..=end as u16)
    }
}

impl From<PortRange> for PgRange<i32> {
    fn from(range: PortRange) -> PgRange<i32> {
        PgRange {
            start: Bound::Included(i32::from(*range.0.start())),
            end: Bound::Included(i32::from(*range.0.end())),
        }
    }
}

/// ACL rule can be in one of the following states:
/// - New: the rule has been created and not yet applied
/// - Modified: the rule has been modified and not yet applied
/// - Deleted: the rule has been marked for deletion but not yed removed
/// - Applied: the rule was applied
/// - Expired: the rule is past it's expiration date
///
/// Applied state does NOT guarantee that all locations have received the rule
/// and performed appropriate operations, only that the next time configuration
/// is being sent it will include this rule.
#[derive(Clone, Debug, Default, Deserialize, Serialize, Type, PartialEq, Eq, Hash)]
#[sqlx(type_name = "aclrule_state", rename_all = "lowercase")]
pub enum RuleState {
    #[default]
    New,
    Modified,
    Deleted,
    Applied,
    Expired,
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
    pub allow_all_network_devices: bool,
    pub deny_all_network_devices: bool,
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
    /// Constructs a [`String`] of comma-separated addresses and address ranges.
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

    /// Constructs a [`String`] of comma-separated ports and port ranges.
    pub(crate) fn format_ports(&self) -> String {
        self.ports
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
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
#[derive(Clone, Debug, Default, Model, PartialEq, Eq, FromRow)]
pub struct AclRule<I = NoId> {
    pub id: I,
    // if present points to the original rule before modification / deletion
    pub parent_id: Option<Id>,
    #[model(enum)]
    pub state: RuleState,
    pub name: String,
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub allow_all_network_devices: bool,
    pub deny_all_network_devices: bool,
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
        api_rule: &EditAclRule,
    ) -> Result<ApiAclRule, AclError> {
        let mut transaction = pool.begin().await?;

        // save the rule
        let rule: AclRule = api_rule.clone().try_into()?;
        let rule = rule.save(&mut *transaction).await?;

        // create related objects
        rule.create_related_objects(&mut transaction, api_rule)
            .await?;

        let result: ApiAclRule = rule.to_info(&mut transaction).await?.into();

        transaction.commit().await?;

        Ok(result)
    }

    /// Updates [`AclRule`] with all it's related objects based on [`ApiAclRule`]
    ///
    /// State handling:
    ///
    /// - For rules in `RuleState::Applied` state (rules that are currently active):
    ///   1. Any existing modifications of this rule are deleted
    ///   2. A copy of the rule is created with `RuleState::Modified` state and the original rule as parent
    /// - For rules in `RuleState::Deleted` we return an error since those should not be modified
    /// - For rules in other states (`New`, `Modified` ), we directly update the existing rule
    ///   since they haven't been applied.
    ///
    /// This approach allows us to track changes to applied rules while maintaining their history.
    ///
    /// Applied state does NOT guarantee that all locations have received the rule
    /// and performed appropriate operations, only that the next time configuration
    /// is being sent it will include this rule.
    pub(crate) async fn update_from_api(
        pool: &PgPool,
        id: Id,
        api_rule: &EditAclRule,
    ) -> Result<ApiAclRule, AclError> {
        debug!("Updating rule ID {id} with {api_rule:?}");
        let mut transaction = pool.begin().await?;

        // find the existing rule
        let existing_rule = AclRule::find_by_id(&mut *transaction, id)
            .await?
            .ok_or_else(|| {
                warn!("Update of nonexistent rule ({id}) failed");
                AclError::RuleNotFoundError(id)
            })?;

        // convert API rule to model
        let mut rule: AclRule<NoId> = api_rule.clone().try_into()?;

        // perform appropriate updates depending on existing rule's state
        let rule = match existing_rule.state {
            RuleState::Applied | RuleState::Expired => {
                // create new `RuleState::Modified` rule
                debug!(
                    "Rule {id} state is {:?} - creating new `Modified` rule object",
                    existing_rule.state
                );
                // remove old modifications of this rule
                let result = query!("DELETE FROM aclrule WHERE parent_id = $1", id)
                    .execute(&mut *transaction)
                    .await?;
                debug!(
                    "Removed {} old modifications of rule {id}",
                    result.rows_affected(),
                );

                // save as a new rule with appropriate parent_id and state
                rule.state = RuleState::Modified;
                rule.parent_id = Some(id);
                let rule = rule.save(&mut *transaction).await?;

                // create related objects
                rule.create_related_objects(&mut transaction, api_rule)
                    .await?;

                rule
            }
            RuleState::Deleted => {
                error!("Cannot update a deleted ACL rule {id}");
                return Err(AclError::CannotModifyDeletedRuleError(id));
            }
            RuleState::New | RuleState::Modified => {
                debug!(
                    "Rule {id} is a modification to rule {:?} - updating the modification",
                    existing_rule.parent_id,
                );
                // update the not-yet applied modification itself
                let mut rule = rule.with_id(id);
                rule.parent_id = existing_rule.parent_id;
                rule.state = existing_rule.state;
                rule.save(&mut *transaction).await?;

                // recreate related objects
                rule.delete_related_objects(&mut transaction).await?;
                rule.create_related_objects(&mut transaction, api_rule)
                    .await?;

                rule
            }
        };

        let rule_details = rule.to_info(&mut transaction).await?.into();

        transaction.commit().await?;

        info!("Successfully updated rule {rule_details:?}");
        Ok(rule_details)
    }

    /// Deletes [`AclRule`] with all it's related objects.
    ///
    /// State handling:
    ///
    /// - For rules in `RuleState::Applied` state (rules that are currently active):
    ///   1. Any existing modifications of this rule are deleted.
    ///   2. A copy of the rule is created with `RuleState::Deleted` state and the original rule as
    ///      parent.
    ///
    /// This preserves the original rule while tracking the deletion.
    ///
    /// - For rules in other states (`New`, `Modified` or `Deleted`):
    ///   1. All related objects are deleted
    ///   2. The rule itself is deleted from the database
    ///
    /// Since these rules were not yet applied, we can safely remove them.
    pub(crate) async fn delete_from_api(pool: &PgPool, id: Id) -> Result<(), AclError> {
        debug!("Deleting rule {id}");
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
            RuleState::Applied | RuleState::Expired => {
                // create new `RuleState::Deleted` rule
                debug!(
                    "Rule {id} state is {:?} - creating new `Deleted` rule object",
                    existing_rule.state,
                );
                // delete all modifications of this rule
                let result = query!("DELETE FROM aclrule WHERE parent_id = $1", id)
                    .execute(&mut *transaction)
                    .await?;
                debug!(
                    "Removed {} old modifications of rule {id}",
                    result.rows_affected(),
                );

                // prefetch related objects for use later
                let rule_info = existing_rule.to_info(&mut transaction).await?;

                // save as a new rule with appropriate parent_id and state
                let mut rule = existing_rule.as_noid();
                rule.state = RuleState::Deleted;
                rule.parent_id = Some(id);
                let rule = rule.save(&mut *transaction).await?;

                // inherit related objects from parent rule
                rule.create_related_objects(&mut transaction, &rule_info.into())
                    .await?;
            }
            _ => {
                // delete the not-yet applied modification itself
                debug!(
                    "Rule {id} is a modification to rule {:?} - updating the modification",
                    existing_rule.parent_id,
                );
                // delete related objects
                existing_rule
                    .delete_related_objects(&mut transaction)
                    .await?;

                // delete the rule
                existing_rule.delete(&mut *transaction).await?;
            }
        }

        transaction.commit().await?;
        info!("Rule {id} succesfully deleted or marked for deletion");
        Ok(())
    }

    /// Applies pending changes for all specified rules
    ///
    /// # Errors
    ///
    /// - `AclError::RuleNotFoundError`
    pub async fn apply_rules(rules: &[Id], appstate: &AppState) -> Result<(), AclError> {
        debug!("Applying {} ACL rules: {rules:?}", rules.len());
        let mut transaction = appstate.pool.begin().await?;

        // prepare variable for collecting affected locations
        let mut affected_locations = HashSet::new();

        for id in rules {
            let rule = AclRule::find_by_id(&mut *transaction, *id)
                .await?
                .ok_or_else(|| AclError::RuleNotFoundError(*id))?;
            let locations = rule.get_networks(&mut *transaction).await?;
            for location in locations {
                affected_locations.insert(location);
            }
            rule.apply(&mut transaction).await?;
        }
        info!("Applied {} ACL rules: {rules:?}", rules.len());

        let affected_locations: Vec<WireguardNetwork<Id>> =
            affected_locations.into_iter().collect();
        debug!(
            "{} locations affected by applied ACL rules. Sending gateway firewall update events \
            for each location",
            affected_locations.len()
        );

        for location in affected_locations {
            match location.try_get_firewall_config(&mut transaction).await? {
                Some(firewall_config) => {
                    debug!("Sending firewall update event for location {location}");
                    appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                        location.id,
                        firewall_config,
                    ));
                }
                None => {
                    debug!(
                        "No firewall config generated for location {location}. Not sending a \
                        gateway event"
                    );
                }
            }
        }

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
    if !destination.is_empty() {
        for v in destination.split(',') {
            match v.split('-').collect::<Vec<_>>() {
                l if l.len() == 1 => result.addrs.push(l[0].parse::<IpNetwork>()?),
                l if l.len() == 2 => result
                    .ranges
                    .push((l[0].parse::<IpAddr>()?, l[1].parse::<IpAddr>()?)),
                _ => {
                    error!("Failed to parse destination string: \"{destination}\"");
                    Err(IpNetworkError::InvalidAddr(destination.clone()))?;
                }
            }
        }
    }

    debug!("Parsed destination: {result:?}");
    Ok(result)
}

/// Parses ports string into singular ports and port ranges
/// We should be able to parse a string like this one:
/// `22, 23, 8000-9000, 80-90`
pub fn parse_ports(ports: &str) -> Result<Vec<PortRange>, AclError> {
    debug!("Parsing ports string: {ports}");
    let mut result = Vec::new();
    let ports = ports
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();
    if !ports.is_empty() {
        for v in ports.split(',') {
            match v.split('-').collect::<Vec<_>>() {
                l if l.len() == 1 => {
                    let start = l[0].parse::<u16>()?;
                    result.push(PortRange::new(start, start));
                }
                l if l.len() == 2 => {
                    let start = l[0].parse::<u16>()?;
                    let end = l[1].parse::<u16>()?;
                    result.push(PortRange::new(start, end));
                }
                _ => {
                    error!("Failed to parse ports string: \"{ports}\"");
                    return Err(AclError::InvalidPortsFormat(ports.to_string()));
                }
            }
        }
    }

    debug!("Parsed ports: {result:?}");
    Ok(result)
}

/// Maps [`sqlx::Error`] to [`AclError`] while checking for [`ErrorKind::ForeignKeyViolation`].
fn map_relation_error(err: SqlxError, class: &str, id: Id) -> AclError {
    if let SqlxError::Database(dberror) = &err {
        if dberror.kind() == ErrorKind::ForeignKeyViolation {
            error!("Failed to create ACL related object, foreign key violation: {class}({id}): {dberror}");
            return AclError::InvalidRelationError(format!("{class}({id})"));
        }
    }
    error!("Failed to create ACL related object: {err}");
    AclError::DbError(err)
}

impl AclRule<Id> {
    /// Creates relation objects for given [`AclRule`] based on [`EditAclRule`] object
    async fn create_related_objects(
        &self,
        transaction: &mut PgConnection,
        api_rule: &EditAclRule,
    ) -> Result<(), AclError> {
        let rule_id = self.id;
        debug!("Creating related objects for ACL rule {api_rule:?}");
        // save related networks
        debug!("Creating related networks for ACL rule {rule_id}");
        for network_id in &api_rule.networks {
            let obj = AclRuleNetwork {
                id: NoId,
                rule_id,
                network_id: *network_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "WireguardNetwork", *network_id))?;
        }

        // allowed users
        debug!("Creating related allowed users for ACL rule {rule_id}");
        for user_id in &api_rule.allowed_users {
            let obj = AclRuleUser {
                id: NoId,
                allow: true,
                rule_id,
                user_id: *user_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "User", *user_id))?;
        }

        // denied users
        debug!("Creating related denied users for ACL rule {rule_id}");
        for user_id in &api_rule.denied_users {
            let obj = AclRuleUser {
                id: NoId,
                allow: false,
                rule_id,
                user_id: *user_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "User", *user_id))?;
        }

        // allowed groups
        debug!("Creating related allowed groups for ACL rule {rule_id}");
        for group_id in &api_rule.allowed_groups {
            let obj = AclRuleGroup {
                id: NoId,
                allow: true,
                rule_id,
                group_id: *group_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "Group", *group_id))?;
        }

        // denied groups
        debug!("Creating related denied groups for ACL rule {rule_id}");
        for group_id in &api_rule.denied_groups {
            let obj = AclRuleGroup {
                id: NoId,
                allow: false,
                rule_id,
                group_id: *group_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "Group", *group_id))?;
        }

        // save related aliases
        debug!("Creating related aliases for ACL rule {rule_id}");
        // verify if all aliases have a correct state
        // aliases used for tracking modifications (`AliasState::Modified`) cannot be used by ACL
        // rules
        let invalid_alias_ids: Vec<Id> = query_scalar!(
            "SELECT id FROM aclalias WHERE id = ANY($1) AND state != 'applied'::aclalias_state",
            &api_rule.aliases
        )
        .fetch_all(&mut *transaction)
        .await?;
        if !invalid_alias_ids.is_empty() {
            error!("Cannot use aliases which have not been applied in an ACL rule. Invalid aliases: {invalid_alias_ids:?}");
            return Err(AclError::CannotUseModifiedAliasInRuleError(
                invalid_alias_ids,
            ));
        }
        for alias_id in &api_rule.aliases {
            let obj = AclRuleAlias {
                id: NoId,
                rule_id,
                alias_id: *alias_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "AclAlias", *alias_id))?;
        }

        // allowed devices
        debug!("Creating related allowed devices for ACL rule {rule_id}");
        for device_id in &api_rule.allowed_devices {
            let obj = AclRuleDevice {
                id: NoId,
                allow: true,
                rule_id,
                device_id: *device_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "Device", *device_id))?;
        }

        // denied devices
        debug!("Creating related denied devices for ACL rule {rule_id}");
        for device_id in &api_rule.denied_devices {
            let obj = AclRuleDevice {
                id: NoId,
                allow: false,
                rule_id,
                device_id: *device_id,
            };
            obj.save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "Device", *device_id))?;
        }

        // destination
        let destination = parse_destination(&api_rule.destination)?;
        debug!("Creating related destination ranges for ACL rule {rule_id}");
        for range in destination.ranges {
            if range.1 <= range.0 {
                return Err(AclError::InvalidIpRangeError(format!(
                    "{}-{}",
                    range.0, range.1
                )));
            }
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
        &self,
        transaction: &mut PgConnection,
    ) -> Result<(), SqlxError> {
        let rule_id = self.id;
        debug!("Deleting related objects for ACL rule {rule_id}");
        // networks
        let result = query!("DELETE FROM aclrulenetwork WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;
        debug!(
            "Deleted {} aclrulenetwork records related to rule {rule_id}",
            result.rows_affected()
        );

        // users
        let result = query!("DELETE FROM aclruleuser WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;
        debug!(
            "Deleted {} aclruleuser records related to rule {rule_id}",
            result.rows_affected()
        );

        // groups
        let result = query!("DELETE FROM aclrulegroup WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;
        debug!(
            "Deleted {} aclrulegroup records related to rule {rule_id}",
            result.rows_affected()
        );

        // aliases
        let result = query!("DELETE FROM aclrulealias WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;
        debug!(
            "Deleted {} aclrulealias records related to rule {rule_id}",
            result.rows_affected()
        );

        // devices
        let result = query!("DELETE FROM aclruledevice WHERE rule_id = $1", rule_id)
            .execute(&mut *transaction)
            .await?;
        debug!(
            "Deleted {} aclruledevice records related to rule {rule_id}",
            result.rows_affected()
        );

        // destination ranges
        let result = query!(
            "DELETE FROM aclruledestinationrange WHERE rule_id = $1",
            rule_id
        )
        .execute(&mut *transaction)
        .await?;
        debug!(
            "Deleted {} aclruledestinationrange records related to rule {rule_id}",
            result.rows_affected()
        );

        info!("Deleted related objects for ACL rule {rule_id}");
        Ok(())
    }
}

impl TryFrom<EditAclRule> for AclRule<NoId> {
    type Error = AclError;

    fn try_from(rule: EditAclRule) -> Result<Self, Self::Error> {
        Ok(Self {
            destination: parse_destination(&rule.destination)?.addrs,
            ports: parse_ports(&rule.ports)?
                .into_iter()
                .map(Into::into)
                .collect(),
            id: NoId,
            parent_id: None,
            state: Default::default(),
            name: rule.name,
            allow_all_users: rule.allow_all_users,
            deny_all_users: rule.deny_all_users,
            allow_all_network_devices: rule.allow_all_network_devices,
            deny_all_network_devices: rule.deny_all_network_devices,
            all_networks: rule.all_networks,
            protocols: rule.protocols,
            enabled: rule.enabled,
            expires: rule.expires,
        })
    }
}

impl AclRule<Id> {
    /// Applies pending state change if necessary.
    ///
    /// If current state is [`RuleState::New`] or [`RuleState::Modified`] it does the following:
    /// - changes the state of the rule to `Applied`
    /// - clears rule's `parent_id`.
    /// - deletes it's parent rule
    ///
    /// If current state is [`RuleState::Deleted`] it removes the parent rule and the rule itself.
    ///
    /// # Errors
    ///
    /// - `AclError::RuleAreadyApplied`
    pub async fn apply(mut self, transaction: &mut PgConnection) -> Result<(), AclError> {
        let acl_id = self.id;
        debug!("Applying ACL rule {acl_id} pending state change");

        // Ensure the rule is in a state that can be applied
        match self.state {
            RuleState::New | RuleState::Modified => {
                debug!("Changing ACL rule {acl_id} state to applied");
                self.state = RuleState::Applied;
                let parent_id = self.parent_id;
                self.parent_id = None;
                self.save(&mut *transaction).await?;

                // delete parent rule
                if let Some(parent_id) = parent_id {
                    query!("DELETE FROM aclrule WHERE id = $1", parent_id)
                        .execute(&mut *transaction)
                        .await?;
                }
                info!("Changed ACL rule {acl_id} state to applied");
            }
            RuleState::Deleted => {
                debug!("Removing ACL rule {acl_id} which has been marked for deletion",);
                let parent_id = &self
                    .parent_id
                    .expect("ACL rule marked for deletion must have parent ID");

                // delete current ACL rule itself
                self.delete(&mut *transaction).await?;

                // delete parent rule
                query!("DELETE FROM aclrule WHERE id = $1", parent_id)
                    .execute(&mut *transaction)
                    .await?;

                info!("ACL rule {acl_id} was deleted");
            }
            RuleState::Applied | RuleState::Expired => {
                warn!("ACL rule {acl_id} already applied");
                return Err(AclError::RuleAlreadyAppliedError(self.id));
            }
        }

        Ok(())
    }

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
            "SELECT a.id, parent_id, name, kind \"kind: AliasKind\",state \"state: AliasState\", \
            destination, ports, protocols \
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
        Ok(if allowed {
            self.get_allowed_users(executor).await?
        } else {
            self.get_denied_users(executor).await?
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
        query_as!(
            User,
            "SELECT u.id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, totp_secret, email_mfa_enabled, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, from_ldap, \
            ldap_pass_randomized, ldap_rdn \
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

    /// Returns **active** [`User`]s that are denied by the rule
    pub(crate) async fn get_denied_users<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<User<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            User,
            "SELECT u.id, username, password_hash, last_name, first_name, email, phone, \
            mfa_enabled, totp_enabled, totp_secret, email_mfa_enabled, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, from_ldap, \
            ldap_pass_randomized, ldap_rdn \
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
    pub(crate) async fn get_network_devices<'e, E>(
        &self,
        executor: E,
        allowed: bool,
    ) -> Result<Vec<Device<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        if allowed {
            self.get_allowed_network_devices(executor).await
        } else {
            self.get_denied_network_devices(executor).await
        }
    }

    pub(crate) async fn get_allowed_network_devices<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<Device<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Device,
            "SELECT d.id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM aclruledevice r \
            JOIN device d \
            ON d.id = r.device_id \
            WHERE r.rule_id = $1 \
            AND r.allow = true AND d.configured = true",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    pub(crate) async fn get_denied_network_devices<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<Device<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            Device,
            "SELECT d.id, name, wireguard_pubkey, user_id, created, description, \
            device_type \"device_type: DeviceType\", configured \
            FROM aclruledevice r \
            JOIN device d \
            ON d.id = r.device_id \
            WHERE r.rule_id = $1 \
            AND r.allow = false AND d.configured = true",
            self.id,
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
    pub async fn to_info(&self, conn: &mut PgConnection) -> Result<AclRuleInfo<Id>, SqlxError> {
        let aliases = self.get_aliases(&mut *conn).await?;
        let networks = self.get_networks(&mut *conn).await?;
        let allowed_users = self.get_users(&mut *conn, true).await?;
        let denied_users = self.get_users(&mut *conn, false).await?;
        let allowed_groups = self.get_groups(&mut *conn, true).await?;
        let denied_groups = self.get_groups(&mut *conn, false).await?;
        let allowed_devices = self.get_network_devices(&mut *conn, true).await?;
        let denied_devices = self.get_network_devices(&mut *conn, false).await?;
        let destination_ranges = self.get_destination_ranges(&mut *conn).await?;
        let ports = self.ports.clone().into_iter().map(Into::into).collect();

        Ok(AclRuleInfo {
            id: self.id,
            parent_id: self.parent_id,
            state: self.state.clone(),
            name: self.name.clone(),
            allow_all_users: self.allow_all_users,
            deny_all_users: self.deny_all_users,
            allow_all_network_devices: self.allow_all_network_devices,
            deny_all_network_devices: self.deny_all_network_devices,
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
    pub(crate) async fn get_all_allowed_users<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
    ) -> Result<Vec<User<Id>>, SqlxError> {
        debug!(
            "Preparing list of all allowed users for ACL rule {}",
            self.id
        );
        // return all active users if `allow_all_users` flag is enabled
        if self.allow_all_users {
            debug!(
                "allow_all_users flag is enabled for ACL rule {}. Fetching all active users",
                self.id
            );
            let all_active_users = query_as!(
                User,
                "SELECT id, username, password_hash, last_name, first_name, email, \
                phone, mfa_enabled, totp_enabled, totp_secret, \
                email_mfa_enabled, email_mfa_secret, \
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, from_ldap, \
                ldap_pass_randomized, ldap_rdn \
                FROM \"user\" \
                WHERE is_active = true"
            )
            .fetch_all(executor)
            .await;

            return all_active_users;
        }

        // get explicitly allowed users
        let mut allowed_users = self.allowed_users.clone();

        // get allowed groups IDs
        let allowed_group_ids: Vec<Id> = self.allowed_groups.iter().map(|group| group.id).collect();

        // fetch all active members of allowed groups
        let allowed_groups_users: Vec<User<Id>> = query_as!(
            User,
            "SELECT id, username, password_hash, last_name, first_name, email, phone, mfa_enabled, \
            totp_enabled, totp_secret, email_mfa_enabled, email_mfa_secret, \
            mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
            from_ldap, ldap_pass_randomized, ldap_rdn \
            FROM \"user\" u \
            JOIN group_user gu ON u.id=gu.user_id \
            WHERE u.is_active=true AND gu.group_id=ANY($1)",
            &allowed_group_ids
        )
        .fetch_all(executor)
        .await?;

        // get unique users from both lists
        allowed_users.extend(allowed_groups_users);
        let unique_allowed_users: HashSet<_> = allowed_users.into_iter().collect();

        // convert HashSet to output Vec
        Ok(unique_allowed_users.into_iter().collect())
    }

    /// Wrapper function which combines explicitly specified denied users with members of denied
    /// groups to generate a list of all unique denied users for a given ACL.
    pub(crate) async fn get_all_denied_users<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
    ) -> Result<Vec<User<Id>>, SqlxError> {
        debug!(
            "Preparing list of all denied users for ACL rule {}",
            self.id
        );
        // return all active users if `deny_all_users` flag is enabled
        if self.deny_all_users {
            debug!(
                "deny_all_users flag is enabled for ACL rule {}. Fetching all active users",
                self.id
            );
            let all_denied_users = query_as!(
                User,
                "SELECT id, username, password_hash, last_name, first_name, email, \
                phone, mfa_enabled, totp_enabled, totp_secret, \
                email_mfa_enabled, email_mfa_secret, \
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, from_ldap, \
                ldap_pass_randomized, ldap_rdn \
                FROM \"user\" \
                WHERE is_active = true"
            )
            .fetch_all(executor)
            .await;

            return all_denied_users;
        }

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
                mfa_method \"mfa_method: _\", recovery_codes, is_active, openid_sub, \
                from_ldap, ldap_pass_randomized, ldap_rdn \
                FROM \"user\" u \
            JOIN group_user gu ON u.id=gu.user_id \
                WHERE u.is_active=true AND gu.group_id=ANY($1)",
            &denied_group_ids
        )
        .fetch_all(executor)
        .await?;

        // get unique users from both lists
        denied_users.extend(denied_groups_users);
        let unique_denied_users: HashSet<_> = denied_users.into_iter().collect();

        // convert HashSet to output Vec
        Ok(unique_denied_users.into_iter().collect())
    }

    /// Returns the list of explicitly configured allowed network devices or
    /// a list of all devices if 'allow_all_network_devices' flag is enabled.
    pub(crate) async fn get_all_allowed_devices<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
        location_id: Id,
    ) -> Result<Vec<Device<Id>>, SqlxError> {
        debug!(
            "Preparing list of all allowed network devices for ACL rule {}",
            self.id
        );
        // return all active devices if `allow_all_network_devices` flag is enabled
        if self.allow_all_network_devices {
            query_as!(
                Device,
                "SELECT d.id, name, wireguard_pubkey, user_id, created, description, \
                device_type \"device_type: DeviceType\", configured \
                FROM device d \
                JOIN wireguard_network_device wnd \
                ON d.id = wnd.device_id \
                WHERE device_type = 'network'::device_type AND configured = true AND \
                wireguard_network_id = $1",
                location_id
            )
            .fetch_all(executor)
            .await
        } else {
            // return explicitly configured allowed devices otherwise
            Ok(self.allowed_devices.clone())
        }
    }

    /// Returns the list of explicitly configured denied network devices or
    /// a list of all devices if 'deny_all_network_devices' flag is enabled.
    pub(crate) async fn get_all_denied_devices<'e, E: sqlx::PgExecutor<'e>>(
        &self,
        executor: E,
        location_id: Id,
    ) -> Result<Vec<Device<Id>>, SqlxError> {
        debug!(
            "Preparing list of all denied network devices for ACL rule {}",
            self.id
        );
        // return all active devices if `allow_all_network_devices` flag is enabled
        if self.deny_all_network_devices {
            query_as!(
                Device,
                "SELECT d.id, name, wireguard_pubkey, user_id, created, description, \
                device_type \"device_type: DeviceType\", configured \
                FROM device d \
                JOIN wireguard_network_device wnd \
                ON d.id = wnd.device_id \
                WHERE device_type = 'network'::device_type AND configured = true AND \
                wireguard_network_id = $1",
                location_id
            )
            .fetch_all(executor)
            .await
        } else {
            // return explicitly configured denied devices otherwise
            Ok(self.denied_devices.clone())
        }
    }
}

/// Helper struct combining all DB objects related to given [`AclAlias`].
/// All related objects are stored in vectors.
#[derive(Clone, Debug)]
pub struct AclAliasInfo<I = NoId> {
    pub id: I,
    pub parent_id: Option<Id>,
    pub name: String,
    pub kind: AliasKind,
    pub state: AliasState,
    pub destination: Vec<IpNetwork>,
    pub destination_ranges: Vec<AclAliasDestinationRange<Id>>,
    pub ports: Vec<PortRange>,
    pub protocols: Vec<Protocol>,
    pub rules: Vec<AclRule<Id>>,
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
        self.ports
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl TryFrom<EditAclAlias> for AclAlias<NoId> {
    type Error = AclError;

    fn try_from(alias: EditAclAlias) -> Result<Self, Self::Error> {
        Ok(Self {
            destination: parse_destination(&alias.destination)?.addrs,
            ports: parse_ports(&alias.ports)?
                .into_iter()
                .map(Into::into)
                .collect(),
            id: NoId,
            parent_id: None,
            name: alias.name,
            kind: alias.kind,
            state: AliasState::Applied,
            protocols: alias.protocols,
        })
    }
}

/// ACL alias can be in one of the following states:
/// - Applied: the alias can be used in ACL rules
/// - Modified: the alias has been modified and the changes have not yet been applied
///
/// Unlike ACL rules themselves aliases do not require a `New` state,
/// since they do not cause any changes to locations until they
/// are used by a rule.
/// `Deleted` state is also omitted since we don't allow deleting if an alias is used by any rules.
#[derive(Clone, Debug, Default, Deserialize, Serialize, Type, PartialEq, Eq)]
#[sqlx(type_name = "aclalias_state", rename_all = "lowercase")]
pub enum AliasState {
    #[default]
    Applied,
    Modified,
}

/// ACL alias can be of one of the following types:
/// - Destination: the alias defines a complete destination that an ACL rule applies to
/// - Component: the alias defines parts of a destination and will be combined with other parts manually defined in an ACL rule
#[derive(Clone, Debug, Default, Deserialize, Serialize, Type, PartialEq, Eq)]
#[sqlx(type_name = "aclalias_kind", rename_all = "lowercase")]
pub enum AliasKind {
    #[default]
    Destination,
    Component,
}

/// Database representation of an ACL alias. Aliases can be used to define
/// the destination part of an ACL rule so that it's easier to create new
/// rules with common restrictions. In addition to the [`AclAlias`] we provide
/// [`AclAliasInfo`] and [`ApiAclAlias`] that combine all related objects for
/// easier downstream processing.
#[derive(Clone, Debug, Default, Model, PartialEq)]
pub struct AclAlias<I = NoId> {
    pub id: I,
    // if present points to the original alias before modification
    pub parent_id: Option<Id>,
    pub name: String,
    #[model(enum)]
    pub kind: AliasKind,
    #[model(enum)]
    pub state: AliasState,
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
        state: AliasState,
        kind: AliasKind,
        destination: Vec<IpNetwork>,
        ports: Vec<PgRange<i32>>,
        protocols: Vec<Protocol>,
    ) -> Self {
        Self {
            id: NoId,
            parent_id: None,
            name: name.into(),
            kind,
            state,
            destination,
            ports,
            protocols,
        }
    }

    /// Creates new [`AclAlias`] with all related objects based on [`AclAliasInfo`]
    pub(crate) async fn create_from_api(
        pool: &PgPool,
        api_alias: &EditAclAlias,
    ) -> Result<ApiAclAlias, AclError> {
        let mut transaction = pool.begin().await?;

        // save the alias
        let alias: AclAlias<NoId> = api_alias.clone().try_into()?;
        let alias = alias.save(&mut *transaction).await?;

        // create related objects
        Self::create_related_objects(&mut transaction, alias.id, api_alias).await?;

        transaction.commit().await?;
        let result: ApiAclAlias = alias.to_info(pool).await?.into();
        Ok(result)
    }

    /// Updates [`AclAlias`] with all it's related objects based on [`AclAliasInfo`]
    pub(crate) async fn update_from_api(
        pool: &PgPool,
        id: Id,
        api_alias: &EditAclAlias,
    ) -> Result<ApiAclAlias, AclError> {
        let mut transaction = pool.begin().await?;

        // find existing alias
        let existing_alias = AclAlias::find_by_id(&mut *transaction, id)
            .await?
            .ok_or_else(|| {
                warn!("Update of nonexistent alias ({id}) failed");
                AclError::AliasNotFoundError(id)
            })?;

        // convert API alias to model
        let mut alias: AclAlias<NoId> = api_alias.clone().try_into()?;

        // perform appropriate updates depending on existing alias' state
        let alias = match existing_alias.state {
            AliasState::Applied => {
                // create new `AliasState::Modified` alias
                debug!("Alias {id} state is `Applied` - creating new `Modified` alias object",);
                // remove old modifications of this alias
                let result = query!("DELETE FROM aclalias WHERE parent_id = $1", id)
                    .execute(&mut *transaction)
                    .await?;
                debug!(
                    "Removed {} old modifications of alias {id}",
                    result.rows_affected(),
                );

                // save as a new alias with appropriate parent_id and state
                alias.state = AliasState::Modified;
                alias.parent_id = Some(id);
                let alias = alias.save(&mut *transaction).await?;

                // create related objects
                AclAlias::<Id>::create_related_objects(&mut transaction, alias.id, api_alias)
                    .await?;

                alias
            }
            AliasState::Modified => {
                debug!(
                    "Alias {id} is a modification to alias {:?} - updating the modification",
                    existing_alias.parent_id,
                );
                // update the not-yet applied modification itself
                let mut alias = alias.with_id(id);
                alias.parent_id = existing_alias.parent_id;
                alias.save(&mut *transaction).await?;

                // recreate related objects
                Self::delete_related_objects(&mut transaction, alias.id).await?;
                AclAlias::<Id>::create_related_objects(&mut transaction, alias.id, api_alias)
                    .await?;

                alias
            }
        };

        transaction.commit().await?;
        Ok(alias.to_info(pool).await?.into())
    }

    /// Deletes [`AclAlias`] with all it's related objects.
    ///
    /// State handling:
    ///
    /// - For aliases in `AliasState::Applied` state (aliases that are currently active):
    ///   1. Check if the alias is being used by any ACL rules. Return an error if it is
    ///   2. Any existing modifications of this alias are deleted
    ///   3. Delete the alias itself
    ///
    /// - For aliases in `Modified` state (tracking modifications of already applied aliases):
    ///   1. All related objects are deleted
    ///   2. The alias itself is deleted from the database
    ///
    /// Since these aliases were not yet applied, we can safely remove them.
    pub(crate) async fn delete_from_api(pool: &PgPool, id: Id) -> Result<(), AclError> {
        debug!("Deleting alias {id}");
        let mut transaction = pool.begin().await?;

        // find the existing alias
        let existing_alias = AclAlias::find_by_id(&mut *transaction, id)
            .await?
            .ok_or_else(|| {
                error!("Deletion of nonexistent alias ({id}) failed");
                AclError::AliasNotFoundError(id)
            })?;

        // check if any rules are using this alias
        let rules = existing_alias.get_rules(&mut *transaction).await?;
        if !rules.is_empty() {
            error!("Deletion of alias ({id}) failed. Alias is currently used by following ACL rules: {rules:?}");
            return Err(AclError::AliasUsedByRulesError(id));
        }

        // delete all modifications of this alias if any exist
        let result = query!("DELETE FROM aclalias WHERE parent_id = $1", id)
            .execute(&mut *transaction)
            .await?;
        let removed_modifications = result.rows_affected();
        if removed_modifications > 0 {
            debug!("Removed {removed_modifications} old modifications of alias {id}");
        }

        // delete related objects
        Self::delete_related_objects(&mut transaction, id).await?;

        // delete the alias itself
        existing_alias.delete(&mut *transaction).await?;

        transaction.commit().await?;
        Ok(())
    }

    /// Applies pending changes for all specified aliases
    ///
    /// # Errors
    ///
    /// - `AclError::AliasNotFoundError`
    pub async fn apply_aliases(aliases: &[Id], appstate: &AppState) -> Result<(), AclError> {
        debug!("Applying {} ACL aliases: {aliases:?}", aliases.len());
        let mut transaction = appstate.pool.begin().await?;

        // prepare variable for collecting affected rules
        // we are unable to use `HashSet` because `PgRange` does not implement `Hash` trait
        let mut affected_rules = Vec::new();

        for id in aliases {
            let alias = AclAlias::find_by_id(&mut *transaction, *id)
                .await?
                .ok_or_else(|| AclError::AliasNotFoundError(*id))?;
            // run `apply` before fetching relations, since they'll get updated
            alias.clone().apply(&mut transaction).await?;

            // fetch ACL rules which are using this alias
            let rules = alias.get_rules(&mut *transaction).await?;
            affected_rules.extend(rules);
        }
        info!("Applied {} ACL aliases: {aliases:?}", aliases.len());

        // find locations affected by applying selected aliases
        let mut affected_locations = HashSet::new();
        let mut unique_rule_ids = HashSet::new();
        for rule in affected_rules {
            if unique_rule_ids.insert(rule.id) {
                let locations = rule.get_networks(&mut *transaction).await?;
                for location in locations {
                    affected_locations.insert(location);
                }
            }
        }

        let affected_locations: Vec<WireguardNetwork<Id>> =
            affected_locations.into_iter().collect();
        debug!(
            "{} locations affected by applied ACL aliases. Sending gateway firewall update events \
            for each location",
            affected_locations.len()
        );

        for location in affected_locations {
            match location.try_get_firewall_config(&mut transaction).await? {
                Some(firewall_config) => {
                    debug!("Sending firewall update event for location {location}");
                    appstate.send_wireguard_event(GatewayEvent::FirewallConfigChanged(
                        location.id,
                        firewall_config,
                    ));
                }
                None => {
                    debug!(
                        "No firewall config generated for location {location}. Not sending a \
                        gateway event"
                    );
                }
            }
        }

        transaction.commit().await?;
        Ok(())
    }
}

impl<I: fmt::Debug> AclAlias<I> {
    /// Creates relation objects for a given [`AclAlias`] based on [`AclAliasInfo`] object.
    async fn create_related_objects(
        transaction: &mut PgConnection,
        alias_id: Id,
        api_alias: &EditAclAlias,
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

    /// Deletes relation objects for a given [`AclAlias`].
    async fn delete_related_objects(
        transaction: &mut PgConnection,
        alias_id: Id,
    ) -> Result<(), AclError> {
        debug!("Deleting related objects for ACL alias {alias_id}");
        // destination ranges
        let result = query!(
            "DELETE FROM aclaliasdestinationrange WHERE alias_id = $1",
            alias_id
        )
        .execute(&mut *transaction)
        .await?;
        debug!(
            "Deleted {} aclaliasdestinationrange records related to alias {alias_id}",
            result.rows_affected()
        );

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

    /// Returns all [`AclRule`]s which use this alias
    pub(crate) async fn get_rules<'e, E>(&self, executor: E) -> Result<Vec<AclRule<Id>>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        query_as!(
            AclRule,
            "SELECT ar.id, parent_id, state AS \"state: RuleState\", name, allow_all_users, \
            deny_all_users, allow_all_network_devices, deny_all_network_devices, \
            all_networks, destination, ports, protocols, enabled, expires \
            FROM aclrulealias ara \
            JOIN aclrule ar ON ar.id = ara.rule_id \
            WHERE ara.alias_id = $1",
            self.id,
        )
        .fetch_all(executor)
        .await
    }

    /// Retrieves all related objects from the db and converts [`AclAlias`]
    /// instance to [`AclAliasInfo`].
    pub(crate) async fn to_info(&self, pool: &PgPool) -> Result<AclAliasInfo<Id>, SqlxError> {
        let destination_ranges = self.get_destination_ranges(pool).await?;
        let rules = self.get_rules(pool).await?;

        Ok(AclAliasInfo {
            id: self.id,
            parent_id: self.parent_id,
            name: self.name.clone(),
            kind: self.kind.clone(),
            state: self.state.clone(),
            destination: self.destination.clone(),
            ports: self.ports.clone().into_iter().map(Into::into).collect(),
            protocols: self.protocols.clone(),
            destination_ranges,
            rules,
        })
    }

    /// Applies pending state change if necessary.
    ///
    /// If current state is [`AliasState::Modified`] it does the following:
    /// - changes the state of the alias to `Applied`
    /// - clears alias' `parent_id`.
    /// - updates `alias_id` fields in `aclrulealias` table records
    /// - deletes it's parent alias
    ///
    /// # Errors
    ///
    /// - `AclError::AliasAreadyApplied`
    pub async fn apply(mut self, transaction: &mut PgConnection) -> Result<(), AclError> {
        let alias_id = self.id;
        debug!("Applying ACL alias {alias_id} pending state change");

        // Ensure the alias is in a state that can be applied
        match self.state {
            AliasState::Modified => {
                debug!("Changing ACL alias {alias_id} state to applied");
                self.state = AliasState::Applied;
                let parent_id = self.parent_id;
                self.parent_id = None;
                self.save(&mut *transaction).await?;

                if let Some(parent_id) = parent_id {
                    // update ACL -> rule relations
                    query!(
                        "UPDATE aclrulealias SET alias_id = $1 WHERE alias_id = $2",
                        alias_id,
                        parent_id
                    )
                    .execute(&mut *transaction)
                    .await?;

                    // delete parent alias
                    query!("DELETE FROM aclalias WHERE id = $1", parent_id)
                        .execute(&mut *transaction)
                        .await?;
                }

                info!("Changed ACL alias {alias_id} state to applied");
            }
            AliasState::Applied => {
                error!("ACL alias {alias_id} already applied");
                return Err(AclError::AliasAlreadyAppliedError(self.id));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleNetwork<I = NoId> {
    pub id: I,
    pub rule_id: Id,
    pub network_id: Id,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleUser<I = NoId> {
    pub id: I,
    pub rule_id: Id,
    pub user_id: Id,
    pub allow: bool,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleGroup<I = NoId> {
    pub id: I,
    pub rule_id: Id,
    pub group_id: Id,
    pub allow: bool,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleAlias<I = NoId> {
    pub id: I,
    pub rule_id: Id,
    pub alias_id: Id,
}

#[derive(Clone, Debug, Model, PartialEq)]
pub struct AclRuleDevice<I = NoId> {
    pub id: I,
    pub rule_id: Id,
    pub device_id: Id,
    pub allow: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AclRuleDestinationRange<I = NoId> {
    pub id: I,
    pub rule_id: Id,
    pub start: IpAddr,
    pub end: IpAddr,
}

impl<I> AclRuleDestinationRange<I> {
    pub(crate) fn fits_in_network(&self, ipnet: &IpNetwork) -> bool {
        ipnet.contains(self.start) && ipnet.contains(self.end)
    }
}

impl AclRuleDestinationRange<NoId> {
    pub async fn save<'e, E>(self, executor: E) -> Result<AclRuleDestinationRange<Id>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let id = query_scalar!(
            "INSERT INTO aclruledestinationrange (rule_id, \"start\", \"end\") \
            VALUES ($1, $2, $3) RETURNING id",
            self.rule_id,
            IpNetwork::from(self.start),
            IpNetwork::from(self.end),
        )
        .fetch_one(executor)
        .await?;

        Ok(AclRuleDestinationRange {
            id,
            rule_id: self.rule_id,
            start: self.start,
            end: self.end,
        })
    }
}

impl<I> From<&AclRuleDestinationRange<I>> for RangeInclusive<IpAddr> {
    fn from(value: &AclRuleDestinationRange<I>) -> Self {
        value.start..=value.end
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AclAliasDestinationRange<I = NoId> {
    pub id: I,
    pub alias_id: Id,
    pub start: IpAddr,
    pub end: IpAddr,
}

impl<I> AclAliasDestinationRange<I> {
    pub(crate) fn fits_in_network(&self, ipnet: &IpNetwork) -> bool {
        ipnet.contains(self.start) && ipnet.contains(self.end)
    }
}

impl AclAliasDestinationRange<NoId> {
    pub async fn save<'e, E>(self, executor: E) -> Result<AclAliasDestinationRange<Id>, SqlxError>
    where
        E: PgExecutor<'e>,
    {
        let id = query_scalar!(
            "INSERT INTO aclaliasdestinationrange (alias_id, \"start\", \"end\") \
            VALUES ($1, $2, $3) RETURNING id",
            self.alias_id,
            IpNetwork::from(self.start),
            IpNetwork::from(self.end),
        )
        .fetch_one(executor)
        .await?;

        Ok(AclAliasDestinationRange {
            id,
            alias_id: self.alias_id,
            start: self.start,
            end: self.end,
        })
    }
}

impl<I> From<&AclAliasDestinationRange<I>> for RangeInclusive<IpAddr> {
    fn from(value: &AclAliasDestinationRange<I>) -> Self {
        value.start..=value.end
    }
}

#[cfg(test)]
mod tests;
