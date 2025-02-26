use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use sqlx::postgres::types::PgRange;
use std::ops::{Bound, Range};

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{Id, NoId},
    enterprise::db::models::acl::{AclAlias, AclAliasInfo, AclRule, AclRuleDestinationRange, AclRuleInfo, Protocol},
    handlers::{ApiResponse, ApiResult},
};
use serde_json::{json, Value};

use super::LicenseInfo;

/// API representation of [`AclRule`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiAclRule<I = NoId> {
    pub id: I,
    pub name: String,
    pub all_networks: bool,
    pub networks: Vec<Id>,
    pub expires: Option<NaiveDateTime>,
    // source
    pub allow_all_users: bool,
    pub deny_all_users: bool,
    pub allowed_users: Vec<Id>,
    pub denied_users: Vec<Id>,
    pub allowed_groups: Vec<Id>,
    pub denied_groups: Vec<Id>,
    pub allowed_devices: Vec<Id>,
    pub denied_devices: Vec<Id>,
    // destination
    pub destination: Vec<IpNetwork>,
    pub destination_ranges: Vec<ApiAclRuleDestinationRange>,
    pub aliases: Vec<Id>,
    pub ports: Vec<Range<i32>>,
    pub protocols: Vec<Protocol>,
}

impl<I> ApiAclRule<I> {
    pub fn get_ports(&self) -> Vec<PgRange<i32>> {
        self.ports
            .iter()
            .map(|r| PgRange {
                start: Bound::Included(r.start),
                end: Bound::Included(r.end),
            })
            .collect()
    }
}

impl<I> From<AclRuleInfo<I>> for ApiAclRule<I> {
    fn from(info: AclRuleInfo<I>) -> Self {
        Self {
            id: info.id,
            name: info.name,
            all_networks: info.all_networks,
            networks: info.networks.iter().map(|v| v.id).collect(),
            expires: info.expires,
            allow_all_users: info.allow_all_users,
            deny_all_users: info.deny_all_users,
            allowed_users: info.allowed_users.iter().map(|v| v.id).collect(),
            denied_users: info.denied_users.iter().map(|v| v.id).collect(),
            allowed_groups: info.allowed_groups.iter().map(|v| v.id).collect(),
            denied_groups: info.denied_groups.iter().map(|v| v.id).collect(),
            allowed_devices: info.allowed_devices.iter().map(|v| v.id).collect(),
            denied_devices: info.denied_devices.iter().map(|v| v.id).collect(),
            destination: info.destination,
            destination_ranges: info
                .destination_ranges
                .into_iter()
                .map(Into::into)
                .collect(),
            aliases: info.aliases.iter().map(|v| v.id).collect(),
            ports: info.ports,
            protocols: info.protocols,
        }
    }
}

/// API representation of [`AclRuleDestinationRange`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiAclRuleDestinationRange {
    pub start: IpNetwork,
    pub end: IpNetwork,
}

impl ApiAclRuleDestinationRange {
    pub fn to_db(&self, rule_id: i64) -> AclRuleDestinationRange<NoId> {
        AclRuleDestinationRange {
            id: NoId,
            start: self.start,
            end: self.end,
            rule_id,
        }
    }
}

impl<I> From<AclRuleDestinationRange<I>> for ApiAclRuleDestinationRange {
    fn from(rule: AclRuleDestinationRange<I>) -> Self {
        Self {
            start: rule.start,
            end: rule.end,
        }
    }
}

pub async fn list_acl_rules(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL rules", session.user.username);
    let rules = AclRule::all(&appstate.pool).await?;
    let mut api_rules: Vec<ApiAclRule<Id>> = Vec::with_capacity(rules.len());
    for r in rules.iter() {
        // TODO: may require optimisation wrt. sql queries
        let info = r.to_info(&appstate.pool).await?;
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
    let (rule, status) = match AclRule::find_by_id(&appstate.pool, id).await? {
        Some(rule) => (
            json!(Into::<ApiAclRule<Id>>::into(
                rule.to_info(&appstate.pool).await?
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
    Json(data): Json<ApiAclRule>,
) -> ApiResult {
    debug!("User {} creating ACL rule {data:?}", session.user.username);
    let rule = AclRule::create_from_api(&appstate.pool, &data).await?;
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
    Json(data): Json<ApiAclRule<Id>>,
) -> ApiResult {
    debug!("User {} updating ACL rule {data:?}", session.user.username);
    let rule = AclRule::update_from_api(&appstate.pool, id, &data).await?;
    info!("User {} updated ACL rule", session.user.username);
    Ok(ApiResponse {
        json: json!(rule),
        status: StatusCode::CREATED,
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
    AclRule::delete_from_api(&appstate.pool, id).await?;
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
    let mut api_aliases: Vec<AclAliasInfo<Id>> = Vec::with_capacity(aliases.len());
    for r in aliases.iter() {
        // TODO: may require optimisation wrt. sql queries
        let info = r.to_info(&appstate.pool).await?;
        api_aliases.push(info.into());
    }
    info!("User {} listed ACL aliases", session.user.username);
    Ok(ApiResponse {
        json: json!(api_aliases),
        status: StatusCode::OK,
    })
}
