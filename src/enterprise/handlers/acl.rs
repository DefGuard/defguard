use axum::{extract::State, http::StatusCode};
use chrono::NaiveDateTime;
use futures::future::try_join_all;
use ipnetwork::IpNetwork;
use sqlx::postgres::types::PgRange;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::Id,
    enterprise::db::models::acl::{AclRule, AclRuleInfo},
    handlers::{ApiResponse, ApiResult},
};
use serde_json::json;

use super::LicenseInfo;

/// API representation of [`AclRule`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiAclRule {
    pub id: Id,
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
    pub ports: Vec<PgRange<i32>>,
}

impl From<AclRuleInfo> for ApiAclRule {
    fn from(info: AclRuleInfo) -> Self {
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
    let mut api_rules: Vec<ApiAclRule> = Vec::with_capacity(rules.len());
    for r in rules.iter() {
        let info = r.to_info(&appstate.pool).await?;
        api_rules.push(info.into());
    }
    // let rules: Vec<ApiAclRule> = try_join_all(rules.iter().map(|r| r.to_info(&appstate.pool)))
    //     .await?
    //     .iter()
    //     .map(Into::into)
    //     .collect();
    // let rules: Vec<ApiAclRule> = AclRule::all(&appstate.pool)
    //     .await?
    //     .iter()
    //     .map(async |r| r.to_info(&appstate.pool).await?)
    //     .map(Into::into)
    //     .collect();
    info!("User {} listed ACL rules", session.user.username);
    Ok(ApiResponse {
        json: json!(api_rules),
        status: StatusCode::OK,
    })
}
