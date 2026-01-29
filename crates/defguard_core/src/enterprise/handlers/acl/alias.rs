use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use defguard_common::db::Id;
use serde_json::{Value, json};

use super::{ApiAclAlias, EditAclAlias, LicenseInfo};
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::acl::{AclAlias, AliasKind},
    handlers::{ApiResponse, ApiResult},
};

/// List all ACL aliases.
#[utoipa::path(
    get,
    path = "/api/v1/acl/alias",
    tag = "ACL",
    responses(
        (status = OK, description = "ACL alias"),
    ),
)]
pub(crate) async fn list_acl_aliases(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL aliases", session.user.username);
    let aliases = AclAlias::all(&appstate.pool).await?;
    let mut api_aliases: Vec<ApiAclAlias> = Vec::with_capacity(aliases.len());
    for alias in &aliases {
        // TODO: may require optimisation wrt. sql queries
        let info = alias.to_info(&appstate.pool).await.map_err(|err| {
            error!("Error retrieving ACL alias {alias:?}: {err}");
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

/// Get ACL alias.
#[utoipa::path(
    get,
    path = "/api/v1/acl/alias/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL alias",)
    ),
    responses(
        (status = OK, description = "ACL alias"),
    )
)]
pub(crate) async fn get_acl_alias(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
) -> ApiResult {
    debug!("User {} retrieving ACL alias {id}", session.user.username);
    let (alias, status) = match AclAlias::find_by_id(&appstate.pool, id).await? {
        Some(alias) => (
            json!(ApiAclAlias::from(
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

/// Create ACL alias.
#[utoipa::path(
    post,
    path = "/api/v1/acl/alias",
    tag = "ACL",
    request_body = EditAclAlias,
    responses(
        (status = CREATED, description = "ACL alias"),
    )
)]
pub(crate) async fn create_acl_alias(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EditAclAlias>,
) -> ApiResult {
    debug!("User {} creating ACL alias {data:?}", session.user.username);
    let alias = AclAlias::create_from_api(&appstate.pool, &data, AliasKind::Component)
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

/// Update ACL alias.
#[utoipa::path(
    put,
    path = "/api/v1/acl/alias/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL alias",)
    ),
    request_body = EditAclAlias,
    responses(
        (status = OK, description = "ACL alias"),
    )
)]
pub(crate) async fn update_acl_alias(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
    Json(data): Json<EditAclAlias>,
) -> ApiResult {
    debug!("User {} updating ACL alias {data:?}", session.user.username);
    let alias = AclAlias::update_from_api(&appstate.pool, id, &data, AliasKind::Component)
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

/// Delete ACL alias.
#[utoipa::path(
    delete,
    path = "/api/v1/acl/alias/{id}",
    params(
        ("id" = Id, Path, description = "ID of ACL alias",)
    ),
    responses(
        (status = OK, description = "ACL alias"),
    )
)]
pub(crate) async fn delete_acl_alias(
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
