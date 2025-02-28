use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::NaiveDateTime;

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    db::{Id, NoId},
    enterprise::db::models::acl::{AclAlias, AclAliasInfo, AclRule, AclRuleInfo, Protocol},
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
    pub destination: String,
    pub aliases: Vec<Id>,
    pub ports: String,
    pub protocols: Vec<Protocol>,
}

impl<I> From<AclRuleInfo<I>> for ApiAclRule<I> {
    fn from(info: AclRuleInfo<I>) -> Self {
        Self {
            destination: info.format_destination(),
            ports: info.format_ports(),
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
            aliases: info.aliases.iter().map(|v| v.id).collect(),
            protocols: info.protocols,
        }
    }
}

/// API representation of [`AclAlias`]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiAclAlias<I = NoId> {
    pub id: I,
    pub name: String,
    pub destination: String,
    pub ports: String,
    pub protocols: Vec<Protocol>,
}

impl<I> From<AclAliasInfo<I>> for ApiAclAlias<I> {
    fn from(info: AclAliasInfo<I>) -> Self {
        Self {
            destination: info.format_destination(),
            ports: info.format_ports(),
            id: info.id,
            name: info.name,
            protocols: info.protocols,
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
    let mut api_aliases: Vec<ApiAclAlias<Id>> = Vec::with_capacity(aliases.len());
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
            json!(Into::<ApiAclAlias<Id>>::into(
                alias.to_info(&appstate.pool).await?
            )),
            StatusCode::OK,
        ),
        None => (Value::Null, StatusCode::NOT_FOUND),
    };

    info!("User {} retrieved ACL alias {id}", session.user.username);
    Ok(ApiResponse { json: alias, status })
}

pub async fn create_acl_alias(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<ApiAclAlias>,
) -> ApiResult {
    debug!("User {} creating ACL alias {data:?}", session.user.username);
    let alias = AclAlias::create_from_api(&appstate.pool, &data).await?;
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
    Json(data): Json<ApiAclAlias<Id>>,
) -> ApiResult {
    debug!("User {} updating ACL alias {data:?}", session.user.username);
    let alias = AclAlias::update_from_api(&appstate.pool, id, &data).await?;
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
    AclAlias::delete_from_api(&appstate.pool, id).await?;
    info!("User {} deleted ACL alias {id}", session.user.username);
    Ok(ApiResponse::default())
}

