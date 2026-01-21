use axum::{
    Json,
    extract::{Path, State},
};
use defguard_common::db::Id;
use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::{
        db::models::acl::{AclAlias, AliasKind},
        handlers::{
            LicenseInfo,
            acl::{ApiAclAlias, EditAclAlias},
        },
    },
    handlers::{ApiResponse, ApiResult},
};

/// List ACL destinations.
#[utoipa::path(
    get,
    path = "/api/v1/acl/destination",
    tag = "ACL",
    responses(
        (status = OK, description = "ACL destination"),
    )
)]
pub(crate) async fn list_acl_destinations(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL destinations", session.user.username);
    let aliases = AclAlias::all(&appstate.pool).await?;
    let mut api_aliases: Vec<ApiAclAlias> = Vec::with_capacity(aliases.len());
    for alias in &aliases {
        // TODO: may require optimisation wrt. sql queries
        let info = alias.to_info(&appstate.pool).await.map_err(|err| {
            error!("Error retrieving ACL destination {alias:?}: {err}");
            err
        })?;
        api_aliases.push(info.into());
    }
    info!("User {} listed ACL destinations", session.user.username);
    Ok(ApiResponse {
        json: json!(api_aliases),
        status: StatusCode::OK,
    })
}

/// Get ACL destination.
#[utoipa::path(
    get,
    path = "/api/v1/acl/destination/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL destination",)
    ),
    responses(
        (status = OK, description = "ACL destination"),
    )
)]
pub(crate) async fn get_acl_destination(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
) -> ApiResult {
    debug!(
        "User {} retrieving ACL destination {id}",
        session.user.username
    );
    let (alias, status) = match AclAlias::find_by_id(&appstate.pool, id).await? {
        Some(alias) => (
            json!(ApiAclAlias::from(
                alias.to_info(&appstate.pool).await.map_err(|err| {
                    error!("Error retrieving ACL destination {alias:?}: {err}");
                    err
                })?
            )),
            StatusCode::OK,
        ),
        None => (Value::Null, StatusCode::NOT_FOUND),
    };

    info!(
        "User {} retrieved ACL destination {id}",
        session.user.username
    );
    Ok(ApiResponse {
        json: alias,
        status,
    })
}

/// Create ACL destination.
#[utoipa::path(
    post,
    path = "/api/v1/acl/destination",
    tag = "ACL",
    request_body = EditAclAlias,
    responses(
        (status = CREATED, description = "ACL destination"),
    )
)]
pub(crate) async fn create_acl_destination(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EditAclAlias>,
) -> ApiResult {
    debug!(
        "User {} creating ACL destination {data:?}",
        session.user.username
    );
    let alias = AclAlias::create_from_api(&appstate.pool, &data, AliasKind::Destination)
        .await
        .map_err(|err| {
            error!("Error creating ACL destination {data:?}: {err}");
            err
        })?;
    info!(
        "User {} created ACL destination {}",
        session.user.username, alias.id
    );
    Ok(ApiResponse {
        json: json!(alias),
        status: StatusCode::CREATED,
    })
}

/// Update ACL destination.
#[utoipa::path(
    put,
    path = "/api/v1/acl/destination/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL destination",)
    ),
    responses(
        (status = OK, description = "ACL destination"),
    )
)]
pub(crate) async fn update_acl_destination(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
    Json(data): Json<EditAclAlias>,
) -> ApiResult {
    debug!(
        "User {} updating ACL destination {data:?}",
        session.user.username
    );
    let alias = AclAlias::update_from_api(&appstate.pool, id, &data, AliasKind::Destination)
        .await
        .map_err(|err| {
            error!("Error updating ACL destination {data:?}: {err}");
            err
        })?;
    info!("User {} updated ACL destination", session.user.username);
    Ok(ApiResponse {
        json: json!(alias),
        status: StatusCode::OK,
    })
}

/// Delete ACL destination.
#[utoipa::path(
    delete,
    path = "/api/v1/acl/destination/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL destination",)
    ),
    responses(
        (status = OK, description = "ACL destination"),
    )
)]
pub(crate) async fn delete_acl_destination(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<i64>,
) -> ApiResult {
    debug!(
        "User {} deleting ACL destination {id}",
        session.user.username
    );
    AclAlias::delete_from_api(&appstate.pool, id)
        .await
        .map_err(|err| {
            error!("Error deleting ACL destination {id}: {err}");
            err
        })?;
    info!(
        "User {} deleted ACL destination {id}",
        session.user.username
    );
    Ok(ApiResponse::default())
}
