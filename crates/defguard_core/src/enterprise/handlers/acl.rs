use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::NaiveDateTime;
use serde_json::{Value, json};

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::Id,
    enterprise::db::models::acl::{
        AclAlias, AclAliasInfo, AclRule, AclRuleInfo, AliasKind, AliasState, Protocol, RuleState,
    },
    error::WebError,
    handlers::{ApiResponse, ApiResult},
};

/// API representation of [`AclRule`] used in API responses
/// All relations represented as arrays of ids.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ApiAclRule {
    pub id: Id,
    pub parent_id: Option<Id>,
    pub state: RuleState,
    pub name: String,
    pub all_networks: bool,
    pub networks: Vec<Id>,
    pub expires: Option<NaiveDateTime>,
    pub enabled: bool,
    // source
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub allow_all_network_devices: bool,
    pub deny_all_network_devices: bool,
    pub allowed_users: Vec<Id>,
    pub denied_users: Vec<Id>,
    pub allowed_groups: Vec<Id>,
    pub denied_groups: Vec<Id>,
    pub allowed_devices: Vec<Id>,
    pub denied_devices: Vec<Id>,
    // destination
    pub destination: String,
    pub aliases: Vec<Id>,
    pub ports: String,
    pub protocols: Vec<Protocol>,
}

impl From<AclRuleInfo<Id>> for ApiAclRule {
    fn from(info: AclRuleInfo<Id>) -> Self {
        Self {
            destination: info.format_destination(),
            ports: info.format_ports(),
            id: info.id,
            parent_id: info.parent_id,
            state: info.state,
            name: info.name,
            all_networks: info.all_networks,
            networks: info.networks.iter().map(|v| v.id).collect(),
            expires: info.expires,
            allow_all_users: info.allow_all_users,
            deny_all_users: info.deny_all_users,
            allow_all_network_devices: info.allow_all_network_devices,
            deny_all_network_devices: info.deny_all_network_devices,
            allowed_users: info.allowed_users.iter().map(|v| v.id).collect(),
            denied_users: info.denied_users.iter().map(|v| v.id).collect(),
            allowed_groups: info.allowed_groups.iter().map(|v| v.id).collect(),
            denied_groups: info.denied_groups.iter().map(|v| v.id).collect(),
            allowed_devices: info.allowed_devices.iter().map(|v| v.id).collect(),
            denied_devices: info.denied_devices.iter().map(|v| v.id).collect(),
            aliases: info.aliases.iter().map(|v| v.id).collect(),
            protocols: info.protocols,
            enabled: info.enabled,
        }
    }
}

/// API representation of [`AclRule`] used in API requests for modification operations
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct EditAclRule {
    pub name: String,
    pub all_networks: bool,
    pub networks: Vec<Id>,
    pub expires: Option<NaiveDateTime>,
    pub enabled: bool,
    // source
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub allow_all_network_devices: bool,
    pub deny_all_network_devices: bool,
    pub allowed_users: Vec<Id>,
    pub denied_users: Vec<Id>,
    pub allowed_groups: Vec<Id>,
    pub denied_groups: Vec<Id>,
    pub allowed_devices: Vec<Id>,
    pub denied_devices: Vec<Id>,
    // destination
    pub destination: String,
    pub aliases: Vec<Id>,
    pub ports: String,
    pub protocols: Vec<Protocol>,
}

impl EditAclRule {
    pub fn validate(&self) -> Result<(), WebError> {
        // check if some allowed users/group/devices are configured
        if !(self.allow_all_users
            || self.allow_all_network_devices
            || !self.allowed_users.is_empty()
            || !self.allowed_groups.is_empty()
            || !self.allowed_devices.is_empty())
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
            destination: info.format_destination(),
            ports: info.format_ports(),
            name: info.name,
            all_networks: info.all_networks,
            networks: info.networks.iter().map(|v| v.id).collect(),
            expires: info.expires,
            allow_all_users: info.allow_all_users,
            deny_all_users: info.deny_all_users,
            allow_all_network_devices: info.allow_all_network_devices,
            deny_all_network_devices: info.deny_all_network_devices,
            allowed_users: info.allowed_users.iter().map(|v| v.id).collect(),
            denied_users: info.denied_users.iter().map(|v| v.id).collect(),
            allowed_groups: info.allowed_groups.iter().map(|v| v.id).collect(),
            denied_groups: info.denied_groups.iter().map(|v| v.id).collect(),
            allowed_devices: info.allowed_devices.iter().map(|v| v.id).collect(),
            denied_devices: info.denied_devices.iter().map(|v| v.id).collect(),
            aliases: info.aliases.iter().map(|v| v.id).collect(),
            protocols: info.protocols,
            enabled: info.enabled,
        }
    }
}

/// API representation of [`AclAlias`]
/// All relations represented as arrays of ids.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApiAclAlias {
    #[serde(default)]
    pub id: Id,
    pub parent_id: Option<Id>,
    pub name: String,
    pub kind: AliasKind,
    pub state: AliasState,
    pub destination: String,
    pub ports: String,
    pub protocols: Vec<Protocol>,
    pub rules: Vec<Id>,
}

impl From<AclAliasInfo<Id>> for ApiAclAlias {
    fn from(info: AclAliasInfo<Id>) -> Self {
        Self {
            destination: info.format_destination(),
            ports: info.format_ports(),
            id: info.id,
            parent_id: info.parent_id,
            name: info.name,
            kind: info.kind,
            state: info.state,
            protocols: info.protocols,
            rules: info.rules.iter().map(|v| v.id).collect(),
        }
    }
}

/// API representation of [`AclAlias`] used in API requests for modification operations
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct EditAclAlias {
    pub name: String,
    pub kind: AliasKind,
    pub destination: String,
    pub ports: String,
    pub protocols: Vec<Protocol>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyAclRulesData {
    rules: Vec<Id>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyAclAliasesData {
    aliases: Vec<Id>,
}

pub async fn list_acl_rules(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL rules", session.user.username);
    let mut conn = appstate.pool.acquire().await?;
    let rules = AclRule::all(&mut *conn).await?;
    let mut api_rules: Vec<ApiAclRule> = Vec::with_capacity(rules.len());
    for r in &rules {
        // TODO: may require optimisation wrt. sql queries
        let info = r.to_info(&mut conn).await.map_err(|err| {
            error!("Error retrieving ACL rule {r:?}: {err}");
            err
        })?;
        api_rules.push(info.into());
    }
    info!("User {} listed ACL rules", session.user.username);
    Ok(ApiResponse {
        json: json!(api_rules),
        status: StatusCode::OK,
    })
}

pub async fn get_acl_rule(
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
            json!(Into::<ApiAclRule>::into(
                rule.to_info(&mut conn).await.map_err(|err| {
                    error!("Error retrieving ACL rule {rule:?}: {err}");
                    err
                })?
            )),
            StatusCode::OK,
        ),
        None => (Value::Null, StatusCode::NOT_FOUND),
    };

    info!("User {} retrieved ACL rule {id}", session.user.username);
    Ok(ApiResponse { json: rule, status })
}

pub async fn create_acl_rule(
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
    Ok(ApiResponse {
        json: json!(rule),
        status: StatusCode::CREATED,
    })
}

pub async fn update_acl_rule(
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
    Ok(ApiResponse {
        json: json!(rule),
        status: StatusCode::OK,
    })
}

pub async fn delete_acl_rule(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<i64>,
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

pub async fn list_acl_aliases(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL aliases", session.user.username);
    let aliases = AclAlias::all(&appstate.pool).await?;
    let mut api_aliases: Vec<ApiAclAlias> = Vec::with_capacity(aliases.len());
    for a in &aliases {
        // TODO: may require optimisation wrt. sql queries
        let info = a.to_info(&appstate.pool).await.map_err(|err| {
            error!("Error retrieving ACL alias {a:?}: {err}");
            err
        })?;
        api_aliases.push(info.into());
    }
    info!("User {} listed ACL aliases", session.user.username);
    Ok(ApiResponse {
        json: json!(api_aliases),
        status: StatusCode::OK,
    })
}

pub async fn get_acl_alias(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
) -> ApiResult {
    debug!("User {} retrieving ACL alias {id}", session.user.username);
    let (alias, status) = match AclAlias::find_by_id(&appstate.pool, id).await? {
        Some(alias) => (
            json!(Into::<ApiAclAlias>::into(
                alias.to_info(&appstate.pool).await.map_err(|err| {
                    error!("Error retrieving ACL alias {alias:?}: {err}");
                    err
                })?
            )),
            StatusCode::OK,
        ),
        None => (Value::Null, StatusCode::NOT_FOUND),
    };

    info!("User {} retrieved ACL alias {id}", session.user.username);
    Ok(ApiResponse {
        json: alias,
        status,
    })
}

pub async fn create_acl_alias(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EditAclAlias>,
) -> ApiResult {
    debug!("User {} creating ACL alias {data:?}", session.user.username);
    let alias = AclAlias::create_from_api(&appstate.pool, &data)
        .await
        .map_err(|err| {
            error!("Error creating ACL alias {data:?}: {err}");
            err
        })?;
    info!(
        "User {} created ACL alias {}",
        session.user.username, alias.id
    );
    Ok(ApiResponse {
        json: json!(alias),
        status: StatusCode::CREATED,
    })
}

pub async fn update_acl_alias(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
    Json(data): Json<EditAclAlias>,
) -> ApiResult {
    debug!("User {} updating ACL alias {data:?}", session.user.username);
    let alias = AclAlias::update_from_api(&appstate.pool, id, &data)
        .await
        .map_err(|err| {
            error!("Error updating ACL alias {data:?}: {err}");
            err
        })?;
    info!("User {} updated ACL alias", session.user.username);
    Ok(ApiResponse {
        json: json!(alias),
        status: StatusCode::OK,
    })
}

pub async fn delete_acl_alias(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<i64>,
) -> ApiResult {
    debug!("User {} deleting ACL alias {id}", session.user.username);
    AclAlias::delete_from_api(&appstate.pool, id)
        .await
        .map_err(|err| {
            error!("Error deleting ACL alias {id}: {err}");
            err
        })?;
    info!("User {} deleted ACL alias {id}", session.user.username);
    Ok(ApiResponse::default())
}

pub async fn apply_acl_rules(
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

pub async fn apply_acl_aliases(
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
