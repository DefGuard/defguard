pub mod alias;
pub(crate) mod destination;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use defguard_common::db::Id;
use serde_json::{Value, json};
use utoipa::ToSchema;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::acl::{AclAlias, AclRule, AclRuleInfo, Protocol, RuleState},
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};

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
    let rules = AclRule::all(&mut *conn).await?;
    let mut api_rules = Vec::<ApiAclRule>::with_capacity(rules.len());
    for rule in &rules {
        // TODO: may require optimisation wrt. sql queries
        let info = rule.to_info(&mut conn).await.map_err(|err| {
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
            json!(ApiAclRule::from(rule.to_info(&mut conn).await.map_err(
                |err| {
                    error!("Error retrieving ACL rule {rule:?}: {err}");
                    err
                }
            )?)),
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

    let rule = AclRule::create_from_api(&appstate.pool, &data)
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

    let rule = AclRule::update_from_api(&appstate.pool, id, &data)
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
    AclRule::delete_from_api(&appstate.pool, id)
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
    AclRule::apply_rules(&data.rules, &appstate)
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
    AclAlias::apply_aliases(&data.aliases, &appstate)
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
