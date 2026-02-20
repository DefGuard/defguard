pub mod alias;
pub(crate) mod destination;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use defguard_common::db::{Id, NoId};
use serde_json::{Value, json};
use utoipa::ToSchema;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};
use defguard_common::db::models::WireguardNetwork;
use defguard_enterprise_db::models::acl::{
    AclAlias, AclError, AclRule, AclRuleAlias, AclRuleDestinationRange, AclRuleDevice,
    AclRuleGroup, AclRuleInfo, AclRuleNetwork, AclRuleUser, AliasKind, AliasState, Protocol,
    RuleState, parse_destination_addresses, parse_ports,
};
use sqlx::error::ErrorKind;
use sqlx::postgres::types::PgRange;
use sqlx::{PgConnection, PgPool, query};
use std::net::IpAddr;

/// API representation of [`AclRule`] used in API responses.
/// All relations represented as arrays of IDs.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ApiAclRule {
    pub id: Id,
    pub parent_id: Option<Id>,
    pub state: RuleState,
    pub name: String,
    pub all_locations: bool,
    pub locations: Vec<Id>,
    pub expires: Option<NaiveDateTime>,
    pub enabled: bool,
    // source
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub allow_all_groups: bool,
    pub deny_all_groups: bool,
    pub allow_all_network_devices: bool,
    pub deny_all_network_devices: bool,
    pub allowed_users: Vec<Id>,
    pub denied_users: Vec<Id>,
    pub allowed_groups: Vec<Id>,
    pub denied_groups: Vec<Id>,
    pub allowed_network_devices: Vec<Id>,
    pub denied_network_devices: Vec<Id>,
    // destination
    pub use_manual_destination_settings: bool,
    pub addresses: String,
    pub ports: String,
    pub protocols: Vec<Protocol>,
    pub any_address: bool,
    pub any_port: bool,
    pub any_protocol: bool,
    // aliases
    pub aliases: Vec<Id>,
    pub destinations: Vec<Id>,
}

impl From<AclRuleInfo<Id>> for ApiAclRule {
    fn from(info: AclRuleInfo<Id>) -> Self {
        Self {
            addresses: info.format_destination(),
            ports: info.format_ports(),
            id: info.id,
            parent_id: info.parent_id,
            state: info.state,
            name: info.name,
            all_locations: info.all_locations,
            locations: info.locations.iter().map(|v| v.id).collect(),
            expires: info.expires,
            allow_all_users: info.allow_all_users,
            deny_all_users: info.deny_all_users,
            allow_all_groups: info.allow_all_groups,
            deny_all_groups: info.deny_all_groups,
            allow_all_network_devices: info.allow_all_network_devices,
            deny_all_network_devices: info.deny_all_network_devices,
            allowed_users: info.allowed_users.iter().map(|v| v.id).collect(),
            denied_users: info.denied_users.iter().map(|v| v.id).collect(),
            allowed_groups: info.allowed_groups.iter().map(|v| v.id).collect(),
            denied_groups: info.denied_groups.iter().map(|v| v.id).collect(),
            allowed_network_devices: info.allowed_network_devices.iter().map(|v| v.id).collect(),
            denied_network_devices: info.denied_network_devices.iter().map(|v| v.id).collect(),
            aliases: info.aliases.iter().map(|v| v.id).collect(),
            destinations: info.destinations.iter().map(|v| v.id).collect(),
            protocols: info.protocols,
            enabled: info.enabled,
            any_address: info.any_address,
            any_port: info.any_port,
            any_protocol: info.any_protocol,
            use_manual_destination_settings: info.use_manual_destination_settings,
        }
    }
}

/// API representation of [`AclRule`] used in API requests for modification operations
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
pub struct EditAclRule {
    pub name: String,
    pub all_locations: bool,
    pub locations: Vec<Id>,
    pub expires: Option<NaiveDateTime>,
    pub enabled: bool,
    // source
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub allow_all_groups: bool,
    pub deny_all_groups: bool,
    pub allow_all_network_devices: bool,
    pub deny_all_network_devices: bool,
    pub allowed_users: Vec<Id>,
    pub denied_users: Vec<Id>,
    pub allowed_groups: Vec<Id>,
    pub denied_groups: Vec<Id>,
    pub allowed_network_devices: Vec<Id>,
    pub denied_network_devices: Vec<Id>,
    // destination
    pub use_manual_destination_settings: bool,
    pub addresses: String,
    pub ports: String,
    pub protocols: Vec<Protocol>,
    pub any_address: bool,
    pub any_port: bool,
    pub any_protocol: bool,
    // aliases & destinations
    pub aliases: Vec<Id>,
    pub destinations: Vec<Id>,
}

impl EditAclRule {
    pub fn validate(&self) -> Result<(), WebError> {
        // FIXME: validate that destination is defined
        // check if some allowed users/group/devices are configured
        if !self.allow_all_users
            && !self.allow_all_groups
            && !self.allow_all_network_devices
            && self.allowed_users.is_empty()
            && self.allowed_groups.is_empty()
            && self.allowed_network_devices.is_empty()
        {
            return Err(WebError::BadRequest(
                "Must provide some allowed users, groups or devices".to_string(),
            ));
        }

        Ok(())
    }
}

impl From<AclRuleInfo<Id>> for EditAclRule {
    fn from(info: AclRuleInfo<Id>) -> Self {
        Self {
            addresses: info.format_destination(),
            ports: info.format_ports(),
            name: info.name,
            all_locations: info.all_locations,
            locations: info.locations.iter().map(|v| v.id).collect(),
            expires: info.expires,
            allow_all_users: info.allow_all_users,
            deny_all_users: info.deny_all_users,
            allow_all_groups: info.allow_all_groups,
            deny_all_groups: info.deny_all_groups,
            allow_all_network_devices: info.allow_all_network_devices,
            deny_all_network_devices: info.deny_all_network_devices,
            allowed_users: info.allowed_users.iter().map(|v| v.id).collect(),
            denied_users: info.denied_users.iter().map(|v| v.id).collect(),
            allowed_groups: info.allowed_groups.iter().map(|v| v.id).collect(),
            denied_groups: info.denied_groups.iter().map(|v| v.id).collect(),
            allowed_network_devices: info.allowed_network_devices.iter().map(|v| v.id).collect(),
            denied_network_devices: info.denied_network_devices.iter().map(|v| v.id).collect(),
            aliases: info.aliases.iter().map(|v| v.id).collect(),
            destinations: info.destinations.iter().map(|v| v.id).collect(),
            protocols: info.protocols,
            enabled: info.enabled,
            any_address: info.any_address,
            any_port: info.any_port,
            any_protocol: info.any_protocol,
            use_manual_destination_settings: info.use_manual_destination_settings,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ApplyAclRulesData {
    rules: Vec<Id>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct ApplyAclAliasesData {
    aliases: Vec<Id>,
}

/// List all ACL rules.
#[utoipa::path(
    get,
    path = "/api/v1/acl/rule",
    tag = "ACL",
    responses(
        (status = OK, description = "ACL rules"),
    ),
)]
pub(crate) async fn list_acl_rules(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL rules", session.user.username);
    let mut conn = appstate.pool.acquire().await?;
    let rules: Vec<AclRule<Id>> = sqlx::query_as!(
        AclRule,
        "SELECT id, parent_id, state \"state: _\", name, allow_all_users, deny_all_users, \
        allow_all_groups, deny_all_groups, allow_all_network_devices, deny_all_network_devices, \
        all_locations, addresses, ports, protocols, enabled, expires, any_address, any_port, \
        any_protocol, use_manual_destination_settings FROM aclrule"
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut api_rules = Vec::<ApiAclRule>::with_capacity(rules.len());
    for rule in &rules {
        // TODO: may require optimisation wrt. sql queries
        let info = AclRule::<Id>::to_info(rule, &mut conn)
            .await
            .map_err(|err| {
                error!("Error retrieving ACL rule {rule:?}: {err}");
                err
            })?;
        api_rules.push(info.into());
    }
    info!("User {} listed ACL rules", session.user.username);
    Ok(ApiResponse::json(api_rules, StatusCode::OK))
}

/// Get ACL rule.
#[utoipa::path(
    get,
    path = "/api/v1/acl/rule/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL rule",)
    ),
    responses(
        (status = OK, description = "ACL rule"),
    )
)]
pub(crate) async fn get_acl_rule(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
) -> ApiResult {
    debug!("User {} retrieving ACL rule {id}", session.user.username);
    let mut conn = appstate.pool.acquire().await?;
    let (rule, status) = match AclRule::find_by_id(&mut *conn, id).await? {
        Some(rule) => (
            json!(ApiAclRule::from(
                AclRule::<Id>::to_info(&rule, &mut conn)
                    .await
                    .map_err(|err| {
                        error!("Error retrieving ACL rule {rule:?}: {err}");
                        err
                    })?
            )),
            StatusCode::OK,
        ),
        None => (Value::Null, StatusCode::NOT_FOUND),
    };

    info!("User {} retrieved ACL rule {id}", session.user.username);
    Ok(ApiResponse::new(rule, status))
}

/// Create ACL rule.
#[utoipa::path(
    post,
    path = "/api/v1/acl/rule",
    tag = "ACL",
    request_body = EditAclRule,
    responses(
        (status = OK, description = "ACL rule"),
    )
)]
pub(crate) async fn create_acl_rule(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EditAclRule>,
) -> ApiResult {
    debug!("User {} creating ACL rule {data:?}", session.user.username);

    // validate submitted ACL rule
    data.validate()?;

    let rule = create_rule_from_api(&appstate.pool, &data)
        .await
        .map_err(|err| {
            error!("Error creating ACL rule {data:?}: {err}");
            err
        })?;
    info!(
        "User {} created ACL rule {}",
        session.user.username, rule.id
    );
    Ok(ApiResponse::json(rule, StatusCode::CREATED))
}

/// Update ACL rule.
#[utoipa::path(
    put,
    path = "/api/v1/acl/rule/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL rule",)
    ),
    request_body = EditAclRule,
    responses(
        (status = OK, description = "ACL rule"),
    )
)]
pub(crate) async fn update_acl_rule(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
    Json(data): Json<EditAclRule>,
) -> ApiResult {
    debug!("User {} updating ACL rule {data:?}", session.user.username);

    // validate submitted ACL rule
    data.validate()?;

    let rule = update_rule_from_api(&appstate.pool, id, &data)
        .await
        .map_err(|err| {
            error!("Error updating ACL rule {data:?}: {err}");
            err
        })?;
    info!("User {} updated ACL rule", session.user.username);
    Ok(ApiResponse::json(rule, StatusCode::OK))
}

/// Delete ACL rule.
#[utoipa::path(
    delete,
    path = "/api/v1/acl/rule/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL rule",)
    ),
    responses(
        (status = OK, description = "ACL rule"),
    )
)]
pub(crate) async fn delete_acl_rule(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
) -> ApiResult {
    debug!("User {} deleting ACL rule {id}", session.user.username);
    delete_rule_from_api(&appstate.pool, id)
        .await
        .map_err(|err| {
            error!("Error deleting ACL rule {id}: {err}");
            err
        })?;
    info!("User {} deleted ACL rule {id}", session.user.username);
    Ok(ApiResponse::default())
}

/// Apply ACL alias.
#[utoipa::path(
    put,
    path = "/api/v1/acl/rule/apply",
    request_body = ApplyAclRulesData,
    responses(
        (status = OK, description = "ACL alias"),
    )
)]
pub(crate) async fn apply_acl_rules(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<ApplyAclRulesData>,
) -> ApiResult {
    debug!(
        "User {} applying ACL rules: {:?}",
        session.user.username, data.rules
    );
    apply_rules_from_api(&appstate.pool, &appstate, &data.rules)
        .await
        .map_err(|err| {
            error!("Error applying ACL rules {data:?}: {err}");
            err
        })?;
    info!(
        "User {} applied ACL rules: {:?}",
        session.user.username, data.rules
    );
    Ok(ApiResponse::default())
}

/// Apply ACL aliases.
#[utoipa::path(
    put,
    path = "/api/v1/acl/alias/apply",
    request_body = ApplyAclAliasesData,
    responses(
        (status = OK, description = "ACL alias"),
    )
)]
pub(crate) async fn apply_acl_aliases(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<ApplyAclAliasesData>,
) -> ApiResult {
    debug!(
        "User {} applying ACL aliases: {:?}",
        session.user.username, data.aliases
    );
    apply_aliases_from_api(&appstate.pool, &data.aliases)
        .await
        .map_err(|err| {
            error!("Error applying ACL aliases {data:?}: {err}");
            err
        })?;
    info!(
        "User {} applied ACL aliases: {:?}",
        session.user.username, data.aliases
    );
    Ok(ApiResponse::default())
}

async fn create_rule_from_api(pool: &PgPool, data: &EditAclRule) -> Result<ApiAclRule, AclError> {
    let mut transaction = pool.begin().await?;
    let (rule, ranges) = build_rule_from_api(data, RuleState::New)?;
    let rule: AclRule<Id> = rule.save(&mut *transaction).await?;
    create_rule_relations(&mut transaction, rule.id, data, &ranges).await?;
    transaction.commit().await?;
    let mut conn = pool.acquire().await?;
    Ok(AclRule::<Id>::to_info(&rule, &mut conn).await?.into())
}

async fn update_rule_from_api(
    pool: &PgPool,
    id: Id,
    data: &EditAclRule,
) -> Result<ApiAclRule, AclError> {
    let mut transaction = pool.begin().await?;
    let existing: AclRule<Id> = AclRule::find_by_id(&mut *transaction, id)
        .await?
        .ok_or_else(|| {
            warn!("Update of nonexistent rule ({id}) failed");
            AclError::RuleNotFoundError(id)
        })?;

    if existing.state == RuleState::Deleted {
        return Err(AclError::CannotModifyDeletedRuleError(id));
    }

    let target_rule = match existing.state {
        RuleState::Applied => {
            let result = query!("DELETE FROM aclrule WHERE parent_id = $1", id)
                .execute(&mut *transaction)
                .await?;
            debug!(
                "Removed {} old modifications of rule {id}",
                result.rows_affected()
            );

            let (mut rule, ranges) = build_rule_from_api(data, RuleState::Modified)?;
            rule.parent_id = Some(id);
            let rule: AclRule<Id> = rule.save(&mut *transaction).await?;
            create_rule_relations(&mut transaction, rule.id, data, &ranges).await?;
            rule
        }
        RuleState::New | RuleState::Modified | RuleState::Expired => {
            let (rule, ranges) = build_rule_from_api(data, existing.state.clone())?;
            let mut rule = rule.with_id(existing.id);
            rule.parent_id = existing.parent_id;
            rule.save(&mut *transaction).await?;
            rule.delete_related_objects(&mut transaction).await?;
            create_rule_relations(&mut transaction, rule.id, data, &ranges).await?;
            rule
        }
        RuleState::Deleted => {
            return Err(AclError::CannotModifyDeletedRuleError(id));
        }
    };

    transaction.commit().await?;
    let mut conn = pool.acquire().await?;
    Ok(AclRule::<Id>::to_info(&target_rule, &mut conn)
        .await?
        .into())
}

async fn delete_rule_from_api(pool: &PgPool, id: Id) -> Result<(), AclError> {
    let mut transaction = pool.begin().await?;
    let existing: AclRule<Id> = AclRule::find_by_id(&mut *transaction, id)
        .await?
        .ok_or_else(|| AclError::RuleNotFoundError(id))?;

    match existing.state {
        RuleState::New => {
            existing.delete_related_objects(&mut transaction).await?;
            existing.delete(&mut *transaction).await?;
        }
        RuleState::Applied => {
            let result = query!("DELETE FROM aclrule WHERE parent_id = $1", id)
                .execute(&mut *transaction)
                .await?;
            debug!(
                "Removed {} old modifications of rule {id}",
                result.rows_affected()
            );

            let mut deleted_rule = existing.clone();
            deleted_rule.state = RuleState::Deleted;
            deleted_rule.parent_id = Some(id);
            let deleted_rule = deleted_rule.as_noid();
            let deleted_rule = deleted_rule.save(&mut *transaction).await?;
            create_rule_relations_from_rule(&mut transaction, deleted_rule.id, &existing).await?;
        }
        RuleState::Modified | RuleState::Deleted | RuleState::Expired => {
            existing.delete_related_objects(&mut transaction).await?;
            existing.delete(&mut *transaction).await?;
        }
    }

    transaction.commit().await?;
    Ok(())
}

async fn apply_rules_from_api(
    pool: &PgPool,
    appstate: &AppState,
    rule_ids: &[Id],
) -> Result<(), AclError> {
    if rule_ids.is_empty() {
        return Ok(());
    }

    let mut transaction = pool.begin().await?;
    let mut affected_location_ids: Vec<Id> = Vec::new();

    for rule_id in rule_ids {
        let rule: AclRule<Id> = AclRule::find_by_id(&mut *transaction, *rule_id)
            .await?
            .ok_or_else(|| AclError::RuleNotFoundError(*rule_id))?;
        let location_ids: Vec<Id> = if rule.all_locations {
            let locations: Vec<WireguardNetwork<Id>> =
                WireguardNetwork::all(&mut *transaction).await?;
            locations.into_iter().map(|location| location.id).collect()
        } else {
            let locations: Vec<WireguardNetwork<Id>> = rule.get_networks(&mut *transaction).await?;
            locations.into_iter().map(|location| location.id).collect()
        };
        rule.apply(&mut transaction).await?;
        affected_location_ids.extend(location_ids);
    }

    transaction.commit().await?;

    affected_location_ids.sort_unstable();
    affected_location_ids.dedup();
    for location_id in affected_location_ids {
        if let Some(location) = WireguardNetwork::find_by_id(pool, location_id).await? {
            let mut conn = pool.acquire().await?;
            if let Some(firewall_config) =
                defguard_enterprise_firewall::try_get_location_firewall_config(&location, &mut conn)
                    .await
                    .map_err(|err| AclError::FirewallError(err.to_string()))?
            {
                appstate.send_wireguard_event(crate::grpc::GatewayEvent::FirewallConfigChanged(
                    location.id,
                    firewall_config,
                ));
            }
        }
    }
    Ok(())
}

async fn apply_aliases_from_api(pool: &PgPool, alias_ids: &[Id]) -> Result<(), AclError> {
    if alias_ids.is_empty() {
        return Ok(());
    }

    let mut transaction = pool.begin().await?;
    for alias_id in alias_ids {
        let alias: AclAlias<Id> = AclAlias::find_by_id(&mut *transaction, *alias_id)
            .await?
            .ok_or_else(|| AclError::AliasNotFoundError(*alias_id))?;
        if alias.state == AliasState::Applied {
            return Err(AclError::AliasAlreadyAppliedError(*alias_id));
        }
        alias.apply(&mut transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}

fn build_rule_from_api(
    data: &EditAclRule,
    state: RuleState,
) -> Result<(AclRule, Vec<(IpAddr, IpAddr)>), AclError> {
    let destination = parse_destination_addresses(&data.addresses)?;
    validate_destination_ranges(&destination.ranges)?;
    let ports = parse_ports(&data.ports)?;

    let rule = AclRule {
        id: NoId,
        parent_id: None,
        state,
        name: data.name.clone(),
        allow_all_users: data.allow_all_users,
        deny_all_users: data.deny_all_users,
        allow_all_groups: data.allow_all_groups,
        deny_all_groups: data.deny_all_groups,
        allow_all_network_devices: data.allow_all_network_devices,
        deny_all_network_devices: data.deny_all_network_devices,
        all_locations: data.all_locations,
        addresses: destination.addrs,
        ports: ports
            .into_iter()
            .map(Into::into)
            .collect::<Vec<PgRange<i32>>>(),
        protocols: data.protocols.clone(),
        enabled: data.enabled,
        expires: data.expires,
        any_address: data.any_address,
        any_port: data.any_port,
        any_protocol: data.any_protocol,
        use_manual_destination_settings: data.use_manual_destination_settings,
    };

    Ok((rule, destination.ranges))
}

fn validate_destination_ranges(ranges: &[(IpAddr, IpAddr)]) -> Result<(), AclError> {
    for (start, end) in ranges {
        if start > end {
            return Err(AclError::InvalidIpRangeError(format!("{start}-{end}")));
        }
    }
    Ok(())
}

async fn create_rule_relations(
    transaction: &mut PgConnection,
    rule_id: Id,
    data: &EditAclRule,
    ranges: &[(IpAddr, IpAddr)],
) -> Result<(), AclError> {
    for location_id in &data.locations {
        AclRuleNetwork::new(rule_id, *location_id)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "wireguard_network", *location_id))?;
    }

    for user_id in &data.allowed_users {
        AclRuleUser::new(rule_id, *user_id, true)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "user", *user_id))?;
    }
    for user_id in &data.denied_users {
        AclRuleUser::new(rule_id, *user_id, false)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "user", *user_id))?;
    }

    for group_id in &data.allowed_groups {
        AclRuleGroup::new(rule_id, *group_id, true)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "group", *group_id))?;
    }
    for group_id in &data.denied_groups {
        AclRuleGroup::new(rule_id, *group_id, false)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "group", *group_id))?;
    }

    for device_id in &data.allowed_network_devices {
        AclRuleDevice::new(rule_id, *device_id, true)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "device", *device_id))?;
    }
    for device_id in &data.denied_network_devices {
        AclRuleDevice::new(rule_id, *device_id, false)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "device", *device_id))?;
    }

    let mut modified_aliases = Vec::new();
    for alias_id in &data.aliases {
        let alias: AclAlias<Id> =
            AclAlias::find_by_id_and_kind(&mut *transaction, *alias_id, AliasKind::Component)
                .await?
                .ok_or_else(|| AclError::InvalidRelationError(format!("aclalias({alias_id})")))?;
        if alias.state == AliasState::Modified {
            modified_aliases.push(*alias_id);
            continue;
        }
        AclRuleAlias::new(rule_id, *alias_id)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "aclalias", *alias_id))?;
    }
    for alias_id in &data.destinations {
        let alias: AclAlias<Id> =
            AclAlias::find_by_id_and_kind(&mut *transaction, *alias_id, AliasKind::Destination)
                .await?
                .ok_or_else(|| AclError::InvalidRelationError(format!("aclalias({alias_id})")))?;
        if alias.state == AliasState::Modified {
            modified_aliases.push(*alias_id);
            continue;
        }
        AclRuleAlias::new(rule_id, *alias_id)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "aclalias", *alias_id))?;
    }

    if !modified_aliases.is_empty() {
        return Err(AclError::CannotUseModifiedAliasInRuleError(
            modified_aliases,
        ));
    }

    for range in ranges {
        AclRuleDestinationRange {
            id: NoId,
            rule_id,
            start: range.0,
            end: range.1,
        }
        .save(&mut *transaction)
        .await?;
    }

    Ok(())
}

async fn create_rule_relations_from_rule(
    transaction: &mut PgConnection,
    rule_id: Id,
    source_rule: &AclRule<Id>,
) -> Result<(), AclError> {
    if !source_rule.all_locations {
        let networks = source_rule.get_networks(&mut *transaction).await?;
        for network in networks {
            AclRuleNetwork::new(rule_id, network.id)
                .save(&mut *transaction)
                .await
                .map_err(|err| map_relation_error(err, "wireguard_network", network.id))?;
        }
    }

    let allowed_users = source_rule.get_users(&mut *transaction, true).await?;
    for user in allowed_users {
        AclRuleUser::new(rule_id, user.id, true)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "user", user.id))?;
    }
    let denied_users = source_rule.get_users(&mut *transaction, false).await?;
    for user in denied_users {
        AclRuleUser::new(rule_id, user.id, false)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "user", user.id))?;
    }

    let allowed_groups = source_rule.get_groups(&mut *transaction, true).await?;
    for group in allowed_groups {
        AclRuleGroup::new(rule_id, group.id, true)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "group", group.id))?;
    }
    let denied_groups = source_rule.get_groups(&mut *transaction, false).await?;
    for group in denied_groups {
        AclRuleGroup::new(rule_id, group.id, false)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "group", group.id))?;
    }

    let allowed_devices = source_rule
        .get_network_devices(&mut *transaction, true)
        .await?;
    for device in allowed_devices {
        AclRuleDevice::new(rule_id, device.id, true)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "device", device.id))?;
    }
    let denied_devices = source_rule
        .get_network_devices(&mut *transaction, false)
        .await?;
    for device in denied_devices {
        AclRuleDevice::new(rule_id, device.id, false)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "device", device.id))?;
    }

    let aliases = source_rule.get_aliases(&mut *transaction).await?;
    for alias in aliases {
        AclRuleAlias::new(rule_id, alias.id)
            .save(&mut *transaction)
            .await
            .map_err(|err| map_relation_error(err, "aclalias", alias.id))?;
    }

    let ranges = source_rule
        .get_destination_address_ranges(&mut *transaction)
        .await?;
    for range in ranges {
        AclRuleDestinationRange {
            id: NoId,
            rule_id,
            start: range.start,
            end: range.end,
        }
        .save(&mut *transaction)
        .await?;
    }

    Ok(())
}

/// Maps [`sqlx::Error`] to [`AclError`] while checking for [`ErrorKind::ForeignKeyViolation`].
fn map_relation_error(err: sqlx::Error, class: &str, id: Id) -> AclError {
    if let sqlx::Error::Database(dberror) = &err {
        if dberror.kind() == ErrorKind::ForeignKeyViolation {
            error!(
                "Failed to create ACL related object, foreign key violation: {class}({id}): {dberror}"
            );
            return AclError::InvalidRelationError(format!("{class}({id})"));
        }
    }
    error!("Failed to create ACL related object: {err}");
    AclError::DbError(err)
}
