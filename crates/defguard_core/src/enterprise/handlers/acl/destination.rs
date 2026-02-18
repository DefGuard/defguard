use axum::{
    Json,
    extract::{Path, State},
};
use defguard_common::db::{Id, NoId};
use reqwest::StatusCode;
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

/// API representation of [`AclAlias`] used in API requests for modification operations
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
pub(crate) struct EditAclDestination {
    pub name: String,
    pub addresses: String,
    pub ports: String,
    pub protocols: Vec<Protocol>,
    pub any_address: bool,
    pub any_port: bool,
    pub any_protocol: bool,
}

impl EditAclDestination {
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

/// API representation of [`AclAlias`] for "Destination" (not "Alias Component").
/// All relations represented as arrays of IDs.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, ToSchema)]
pub(crate) struct ApiAclDestination {
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
    pub any_address: bool,
    pub any_port: bool,
    pub any_protocol: bool,
}

impl ApiAclDestination {
    /// Creates new [`AclAlias`] with all related objects based on [`AclAliasInfo`].
    pub(crate) async fn create_from_api(
        pool: &PgPool,
        api_alias: &EditAclDestination,
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
        api_alias: &EditAclDestination,
    ) -> Result<Self, AclError> {
        let mut transaction = pool.begin().await?;

        // find existing alias
        let existing_alias =
            AclAlias::find_by_id_and_kind(&mut *transaction, id, AliasKind::Destination)
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

impl From<AclAliasInfo> for ApiAclDestination {
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
            any_address: info.any_address,
            any_port: info.any_port,
            any_protocol: info.any_protocol,
        }
    }
}

/// List ACL destinations.
#[utoipa::path(
    get,
    path = "/api/v1/acl/destination",
    tag = "ACL",
    responses(
        (status = OK, description = "ACL destination", body = Vec<ApiAclDestination>),
    )
)]
pub(crate) async fn list_acl_destinations(
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
) -> ApiResult {
    debug!("User {} listing ACL destinations", session.user.username);
    let aliases = AclAlias::all_of_kind(&appstate.pool, AliasKind::Destination).await?;
    let mut api_aliases = Vec::<ApiAclDestination>::with_capacity(aliases.len());
    for alias in &aliases {
        // TODO: may require optimisation wrt. sql queries
        let info = alias.to_info(&appstate.pool).await.map_err(|err| {
            error!("Error retrieving ACL destination {alias:?}: {err}");
            err
        })?;
        api_aliases.push(info.into());
    }
    info!("User {} listed ACL destinations", session.user.username);
    Ok(ApiResponse::json(api_aliases, StatusCode::OK))
}

/// Get ACL destination.
#[utoipa::path(
    get,
    path = "/api/v1/acl/destination/{id}",
    tag = "ACL",
    params(
        ("id" = Id, Path, description = "ID of ACL destination")
    ),
    responses(
        (status = OK, description = "ACL destination", body = ApiAclDestination),
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
    let (alias, status) =
        match AclAlias::find_by_id_and_kind(&appstate.pool, id, AliasKind::Destination).await? {
            Some(alias) => (
                json!(ApiAclDestination::from(
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
    Ok(ApiResponse::new(alias, status))
}

/// Create ACL destination.
#[utoipa::path(
    post,
    path = "/api/v1/acl/destination",
    tag = "ACL",
    request_body = EditAclDestination,
    responses(
        (status = CREATED, description = "ACL destination", body = ApiAclDestination),
    )
)]
pub(crate) async fn create_acl_destination(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Json(data): Json<EditAclDestination>,
) -> ApiResult {
    debug!(
        "User {} creating ACL destination {data:?}",
        session.user.username
    );
    let alias = ApiAclDestination::create_from_api(&appstate.pool, &data)
        .await
        .map_err(|err| {
            error!("Error creating ACL destination {data:?}: {err}");
            err
        })?;
    info!(
        "User {} created ACL destination {}",
        session.user.username, alias.id
    );
    Ok(ApiResponse::json(alias, StatusCode::CREATED))
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
        (status = OK, description = "ACL destination", body = ApiAclDestination),
    )
)]
pub(crate) async fn update_acl_destination(
    _license: LicenseInfo,
    _admin: AdminRole,
    State(appstate): State<AppState>,
    session: SessionInfo,
    Path(id): Path<Id>,
    Json(data): Json<EditAclDestination>,
) -> ApiResult {
    debug!(
        "User {} updating ACL destination {data:?}",
        session.user.username
    );
    let alias = ApiAclDestination::update_from_api(&appstate.pool, id, &data)
        .await
        .map_err(|err| {
            error!("Error updating ACL destination {data:?}: {err}");
            err
        })?;
    info!("User {} updated ACL destination", session.user.username);
    Ok(ApiResponse::json(alias, StatusCode::OK))
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
    Path(id): Path<Id>,
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
