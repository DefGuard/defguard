use axum::{extract::State, http::StatusCode, Json};
use chrono::NaiveDateTime;
use ipnetwork::IpNetwork;
use sqlx::postgres::types::PgRange;
use std::ops::{Bound, Range};

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{Id, NoId},
    enterprise::db::models::acl::{AclRule, AclRuleInfo, Protocol},
    handlers::{ApiResponse, ApiResult},
};
use serde_json::json;

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
    // destination
    pub destination: Vec<IpNetwork>, // TODO: does not solve the "IP range" case
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
            destination: info.destination,
            aliases: info.aliases.iter().map(|v| v.id).collect(),
            ports: info.ports,
            protocols: info.protocols,
        }
    }
}

pub async fn get_acl_rules(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL rules", session.user.username);
    let rules = AclRule::all(&appstate.pool).await?;
    let mut api_rules: Vec<ApiAclRule<Id>> = Vec::with_capacity(rules.len());
    for r in rules.iter() {
        let info = r.to_info(&appstate.pool).await?;
        api_rules.push(info.into());
    }
    info!("User {} listed ACL rules", session.user.username);
    Ok(ApiResponse {
        json: json!(api_rules),
        status: StatusCode::OK,
    })
}

pub async fn create_acl_rule(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<ApiAclRule>,
) -> ApiResult {
    debug!("User {} creating ACL rule {data:?}", session.user.username);
    let rule = AclRule::create(&appstate.pool, &data).await?;
    info!("User {} created ACL rule", session.user.username);
    Ok(ApiResponse {
        json: json!(rule),
        status: StatusCode::OK,
    })
}

pub async fn update_acl_rule(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<ApiAclRule<Id>>,
) -> ApiResult {
    debug!("User {} updating ACL rule {data:?}", session.user.username);
    let rule = AclRule::update(&appstate.pool, &data).await?;
    info!("User {} updated ACL rule", session.user.username);
    Ok(ApiResponse {
        json: json!(rule),
        status: StatusCode::OK,
    })
}
