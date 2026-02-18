use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use defguard_common::db::{Id, NoId};
use serde_json::{Value, json};
use sqlx::{PgConnection, PgPool, query};
use utoipa::ToSchema;

use super::LicenseInfo;
use crate::{
    appstate::AppState,
    auth::{AdminRole, SessionInfo},
    enterprise::db::models::acl::{
        AclAlias, AclAliasDestinationRange, AclAliasInfo, AclError, AliasKind, AliasState,
        Protocol, acl_delete_related_objects, parse_destination_addresses,
    },
    handlers::{ApiResponse, ApiResult},
};

/// API representation of [`AclAlias`] used in API requests for modification operations.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
pub struct EditAclAlias {
    pub name: String,
    pub addresses: String,
    pub ports: String,
    pub protocols: Vec<Protocol>,
}

impl EditAclAlias {
    /// Creates relation objects for a given [`AclAlias`] based on [`AclAliasInfo`] object.
    pub(crate) async fn create_related_objects(
        &self,
        transaction: &mut PgConnection,
        alias_id: Id,
    ) -> Result<(), AclError> {
        debug!("Creating related objects for ACL alias {self:?}");
        // save related destination ranges
        let destination = parse_destination_addresses(&self.addresses)?;
        for range in destination.ranges {
            let obj = AclAliasDestinationRange {
                id: NoId,
                alias_id,
                start: range.0,
                end: range.1,
            };
            obj.save(&mut *transaction).await?;
        }

        info!("Created related objects for ACL alias {self:?}");
        Ok(())
    }
}

/// API representation of [`AclAlias`] for "Alias Component" (not "Destination").
/// All relations represented as arrays of IDs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct ApiAclAlias {
    #[serde(default)]
    pub id: Id,
    pub parent_id: Option<Id>,
    pub name: String,
    pub kind: AliasKind,
    pub state: AliasState,
    pub addresses: String,
    pub ports: String,
    pub protocols: Vec<Protocol>,
    pub rules: Vec<Id>,
}

impl ApiAclAlias {
    /// Creates new [`AclAlias`] with all related objects based on [`AclAliasInfo`].
    pub(crate) async fn create_from_api(
        pool: &PgPool,
        api_alias: &EditAclAlias,
    ) -> Result<Self, AclError> {
        let mut transaction = pool.begin().await?;

        let alias = AclAlias::try_from(api_alias)?
            .save(&mut *transaction)
            .await?;

        api_alias
            .create_related_objects(&mut transaction, alias.id)
            .await?;

        transaction.commit().await?;
        let result = Self::from(alias.to_info(pool).await?);
        Ok(result)
    }

    /// Updates [`AclAlias`] with all it's related objects based on [`AclAliasInfo`].
    pub(crate) async fn update_from_api(
        pool: &PgPool,
        id: Id,
        api_alias: &EditAclAlias,
    ) -> Result<Self, AclError> {
        let mut transaction = pool.begin().await?;

        // find existing alias
        let existing_alias =
            AclAlias::find_by_id_and_kind(&mut *transaction, id, AliasKind::Component)
                .await?
                .ok_or_else(|| {
                    warn!("Update of nonexistent alias ({id}) failed");
                    AclError::AliasNotFoundError(id)
                })?;

        // Convert alias from API to model.
        let mut alias = AclAlias::try_from(api_alias)?;

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
                api_alias
                    .create_related_objects(&mut transaction, alias.id)
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
                acl_delete_related_objects(&mut transaction, alias.id).await?;
                api_alias
                    .create_related_objects(&mut transaction, alias.id)
                    .await?;

                alias
            }
        };

        transaction.commit().await?;
        Ok(alias.to_info(pool).await?.into())
    }
}

impl From<AclAliasInfo> for ApiAclAlias {
    fn from(info: AclAliasInfo) -> Self {
        Self {
            addresses: info.format_destination(),
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

/// List all ACL aliases.
#[utoipa::path(
    get,
    path = "/api/v1/acl/alias",
    tag = "ACL",
    responses(
        (status = OK, description = "ACL alias", body = Vec<ApiAclAlias>),
    ),
)]
pub(crate) async fn list_acl_aliases(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL aliases", session.user.username);
    let aliases = AclAlias::all_of_kind(&appstate.pool, AliasKind::Component).await?;
    let mut api_aliases = Vec::<ApiAclAlias>::with_capacity(aliases.len());
    for alias in &aliases {
        // TODO: may require optimisation wrt. sql queries
        let info = alias.to_info(&appstate.pool).await.map_err(|err| {
            error!("Error retrieving ACL alias {alias:?}: {err}");
            err
        })?;
        api_aliases.push(info.into());
    }
    info!("User {} listed ACL aliases", session.user.username);
    Ok(ApiResponse::json(api_aliases, StatusCode::OK))
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
        (status = OK, description = "ACL alias", body = ApiAclAlias),
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
    let (alias, status) =
        match AclAlias::find_by_id_and_kind(&appstate.pool, id, AliasKind::Component).await? {
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
    Ok(ApiResponse::new(alias, status))
}

/// Create ACL alias.
#[utoipa::path(
    post,
    path = "/api/v1/acl/alias",
    tag = "ACL",
    request_body = EditAclAlias,
    responses(
        (status = CREATED, description = "ACL alias", body = ApiAclAlias),
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
    let alias = ApiAclAlias::create_from_api(&appstate.pool, &data)
        .await
        .map_err(|err| {
            error!("Error creating ACL alias {data:?}: {err}");
            err
        })?;
    info!(
        "User {} created ACL alias {}",
        session.user.username, alias.id
    );
    Ok(ApiResponse::json(alias, StatusCode::CREATED))
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
        (status = OK, description = "ACL alias", body = ApiAclAlias),
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
    let alias = ApiAclAlias::update_from_api(&appstate.pool, id, &data)
        .await
        .map_err(|err| {
            error!("Error updating ACL alias {data:?}: {err}");
            err
        })?;
    info!("User {} updated ACL alias", session.user.username);
    Ok(ApiResponse::json(alias, StatusCode::OK))
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
