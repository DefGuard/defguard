use std::collections::HashSet;

use chrono::{NaiveDateTime, Utc};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection, PgExecutor, query, query_as, query_scalar};
use thiserror::Error;
use utoipa::ToSchema;

use crate::db::{
    Id, NoId,
    models::{vpn_client_session::VpnClientMfaMethod, wireguard::LocationMfaMode},
};

/// An MFA flow is a named, ordered list of MFA steps.
#[derive(Clone, Debug, Deserialize, FromRow, Model, PartialEq, Serialize, ToSchema)]
#[table(mfa_flow)]
pub struct MfaFlow<I = NoId> {
    pub id: I,
    pub title: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// A single step within an MFA flow.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, ToSchema)]
pub struct MfaFlowStep<I = NoId> {
    pub id: I,
    pub flow_id: Id,
    pub position: i32,
    pub methods: Vec<VpnClientMfaMethod>,
}

/// DB query result: a flow row plus its server-computed `step_count`.
#[derive(Clone, Debug, Serialize)]
pub struct MfaFlowWithStepCount {
    pub id: Id,
    pub title: String,
    pub step_count: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// A point-in-time snapshot of an MFA flow and its steps, used as the
/// payload for audit events.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MfaFlowSnapshot {
    pub flow: MfaFlow<Id>,
    pub steps: Vec<MfaFlowStep<Id>>,
}

/// Assignment of an MFA flow to a location, enriched for API consumption.
#[derive(Clone, Debug, Serialize)]
pub struct LocationMfaFlowItem {
    pub id: Id,
    pub title: String,
    pub step_count: i64,
    pub group_names: Vec<String>,
    pub position: i32,
    pub is_default: bool,
}

/// Input for a single flow assignment to a location.
#[derive(Clone, Debug)]
pub struct LocationMfaFlowAssignment {
    pub flow_id: Id,
    pub is_default: bool,
    pub group_ids: Vec<Id>,
}

/// A single assignment as recorded in the audit log, including the position the server derived
/// from the submitted order.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocationMfaFlowAssignmentSnapshot {
    pub flow_id: Id,
    pub position: i32,
    pub is_default: bool,
    pub group_ids: Vec<Id>,
}

impl LocationMfaFlowAssignment {
    /// Build the audit snapshot for an ordered assignment list, stamping each entry with the
    /// position it was stored at.
    #[must_use]
    pub fn snapshot(assignments: &[Self]) -> Vec<LocationMfaFlowAssignmentSnapshot> {
        assignments
            .iter()
            .enumerate()
            .map(|(i, a)| LocationMfaFlowAssignmentSnapshot {
                flow_id: a.flow_id,
                position: i as i32,
                is_default: a.is_default,
                group_ids: a.group_ids.clone(),
            })
            .collect()
    }
}

/// Errors that can occur during MFA flow assignment.
#[derive(Debug, Error)]
pub enum MfaFlowAssignmentError {
    #[error("No default MFA flow designated for this location")]
    NoDefaultDesignated,
    #[error("More than one MFA flow designated as the default for this location")]
    MultipleDefaultsDesignated,
    #[error("The default MFA flow assignment must not be scoped to any groups")]
    DefaultHasGroups,
    #[error("MFA flow {0} is a non-default assignment scoped to no groups")]
    NonDefaultWithoutGroups(Id),
    #[error("MFA flow {0} is assigned more than once to this location")]
    DuplicateFlow(Id),
    #[error("MFA flow {0} does not exist")]
    UnknownFlow(Id),
    #[error("Group {0} does not exist")]
    UnknownGroup(Id),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

/// Errors that can occur when updating an MFA flow.
#[derive(Debug, Error)]
pub enum MfaFlowUpdateError {
    #[error("Step {0} does not belong to this MFA flow")]
    UnknownStep(Id),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

/// Errors that can occur when deleting an MFA flow.
#[derive(Debug, Error)]
pub enum MfaFlowDeleteError {
    #[error("MFA flow is the only assignment for location(s): {}", .0.join(", "))]
    LocationRequiresFlow(Vec<String>),
    #[error("MFA flow is the designated default for location(s): {}", .0.join(", "))]
    FlowIsDefault(Vec<String>),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

/// A single structured validation error for an MFA flow input.
#[derive(Clone, Debug)]
pub struct MfaFlowValidationField {
    pub field: String,
    pub code: String,
}

/// Maximum number of steps allowed in a single MFA flow.
pub const MAX_MFA_FLOW_STEPS: usize = 20;

/// Maximum length of an MFA flow title.
pub const MAX_MFA_FLOW_TITLE_LEN: usize = 255;

/// Validates the structural rules for an MFA flow input (title + step methods).
/// License, SMTP and OIDC checks are applied separately by the handler.
pub fn validate_flow_input(
    title: &str,
    step_methods: &[Vec<VpnClientMfaMethod>],
) -> Vec<MfaFlowValidationField> {
    let mut errors = Vec::new();

    if title.trim().is_empty() {
        errors.push(MfaFlowValidationField {
            field: "title".into(),
            code: "required".into(),
        });
    } else if title.len() > MAX_MFA_FLOW_TITLE_LEN {
        errors.push(MfaFlowValidationField {
            field: "title".into(),
            code: "max_length".into(),
        });
    }

    if step_methods.is_empty() {
        errors.push(MfaFlowValidationField {
            field: "steps".into(),
            code: "min_items".into(),
        });
    } else if step_methods.len() > MAX_MFA_FLOW_STEPS {
        errors.push(MfaFlowValidationField {
            field: "steps".into(),
            code: "max_items".into(),
        });
    }

    for (i, methods) in step_methods.iter().enumerate() {
        if methods.is_empty() {
            errors.push(MfaFlowValidationField {
                field: format!("steps[{i}].methods"),
                code: "min_items".into(),
            });
        }

        let mut seen = HashSet::new();
        for method in methods {
            if !seen.insert(*method) {
                errors.push(MfaFlowValidationField {
                    field: format!("steps[{i}].methods"),
                    code: "duplicate".into(),
                });
                break;
            }
        }
    }

    errors
}

/// Offset applied to existing step positions during a swap so that
/// intermediate positions never conflict with the `UNIQUE (flow_id, position)` constraint.
pub const POSITION_SWAP_OFFSET: i32 = 10_000;

/// Internal row type for the `resolve_for_user` query.
struct ResolveAssignmentRow {
    flow_id: Id,
    is_default: bool,
    group_ids: Vec<Id>,
}

impl MfaFlow<NoId> {
    /// Creates a new flow with its steps in a single transaction.
    /// `step_methods` is one `Vec` per step; positions are assigned 0-based
    /// from the outer array order.
    pub async fn create(
        conn: &mut PgConnection,
        title: String,
        step_methods: Vec<Vec<VpnClientMfaMethod>>,
    ) -> sqlx::Result<(MfaFlow<Id>, Vec<MfaFlowStep<Id>>)> {
        let now = Utc::now().naive_utc();
        let flow = MfaFlow {
            id: NoId,
            title,
            created_at: now,
            updated_at: now,
        }
        .save(&mut *conn)
        .await?;

        let steps = MfaFlowStep::insert_batch(&mut *conn, flow.id, &step_methods).await?;

        Ok((flow, steps))
    }
}

impl MfaFlow<Id> {
    /// Updates the title and `updated_at` for a flow row.
    pub async fn update_title(
        conn: &mut PgConnection,
        flow_id: Id,
        title: &str,
    ) -> sqlx::Result<()> {
        // `updated_at` is bound from Rust rather than set with SQL `now()`: the column is
        // `timestamp without time zone`, so `now()` would be cast using the session time
        // zone, while inserts write `Utc::now().naive_utc()`. Binding keeps both UTC.
        query!(
            "UPDATE mfa_flow SET title = $1, updated_at = $2 WHERE id = $3",
            title,
            Utc::now().naive_utc(),
            flow_id,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// Returns whether at least one MFA flow exists.
    ///
    /// Used as the `mfa_enabled` precondition: a location cannot enable MFA until there is a
    /// flow available to assign to it.
    pub async fn any_exist<'e, E: PgExecutor<'e>>(executor: E) -> sqlx::Result<bool> {
        let exists = query_scalar!("SELECT EXISTS (SELECT 1 FROM mfa_flow)")
            .fetch_one(executor)
            .await?;
        Ok(exists.unwrap_or(false))
    }

    /// Returns whether the location has a designated default assignment.
    ///
    /// The `mfa_enabled` precondition uses this: a location cannot be enabled until it has a
    /// default flow to enforce, so "enabled with no policy" is unrepresentable.
    pub async fn has_default_assignment<'e, E: PgExecutor<'e>>(
        executor: E,
        location_id: Id,
    ) -> sqlx::Result<bool> {
        let exists = query_scalar!(
            "SELECT EXISTS (SELECT 1 FROM location_mfa_flow WHERE location_id = $1 AND is_default = true)",
            location_id,
        )
        .fetch_one(executor)
        .await?;
        Ok(exists.unwrap_or(false))
    }

    /// Lists all flows enriched with `step_count`.
    pub async fn list_with_step_count<'e, E: PgExecutor<'e>>(
        executor: E,
    ) -> sqlx::Result<Vec<MfaFlowWithStepCount>> {
        query_as!(
            MfaFlowWithStepCount,
            "SELECT mf.id, mf.title, mf.created_at, mf.updated_at, \
             COALESCE(s.step_count, 0) AS \"step_count!: i64\" \
             FROM mfa_flow mf \
             LEFT JOIN ( \
                 SELECT flow_id, COUNT(*) AS step_count \
                 FROM mfa_flow_step \
                 GROUP BY flow_id \
             ) s ON s.flow_id = mf.id \
             ORDER BY mf.id"
        )
        .fetch_all(executor)
        .await
    }

    /// Updates a flow's title and reconciles its steps in one operation.
    ///
    /// `step_updates` is the full ordered list the caller wants after the
    /// update. Each entry is `(Option<id>, methods)`: `Some(id)` indicates
    /// an existing step to UPDATE (position derived from its index), `None`
    /// indicates a new step to INSERT.
    ///
    /// Steps in the DB that are absent from `step_updates` are DELETEd.
    /// Position swaps are handled by offsetting existing steps into a
    /// disjoint range before moving them to final positions, avoiding
    /// transient UNIQUE conflicts.
    pub async fn update_with_steps(
        conn: &mut PgConnection,
        flow_id: Id,
        title: String,
        step_updates: Vec<(Option<Id>, Vec<VpnClientMfaMethod>)>,
    ) -> Result<(MfaFlow<Id>, Vec<MfaFlowStep<Id>>), MfaFlowUpdateError> {
        let incoming_ids: Vec<Id> = step_updates.iter().filter_map(|(id, _)| *id).collect();

        // Every submitted step id must already belong to this flow. Without this check an id
        // borrowed from another flow would be silently UPDATEd, rewriting that flow's step and
        // reporting it in this flow's response.
        if !incoming_ids.is_empty() {
            let owned: HashSet<Id> =
                query_scalar!("SELECT id FROM mfa_flow_step WHERE flow_id = $1", flow_id)
                    .fetch_all(&mut *conn)
                    .await?
                    .into_iter()
                    .collect();

            if let Some(foreign) = incoming_ids.iter().find(|id| !owned.contains(id)) {
                return Err(MfaFlowUpdateError::UnknownStep(*foreign));
            }
        }

        Self::update_title(&mut *conn, flow_id, &title).await?;

        if incoming_ids.is_empty() {
            MfaFlowStep::delete_by_flow(&mut *conn, flow_id).await?;
        } else {
            MfaFlowStep::delete_by_flow_except(&mut *conn, flow_id, &incoming_ids).await?;
            MfaFlowStep::offset_positions(&mut *conn, flow_id, POSITION_SWAP_OFFSET, &incoming_ids)
                .await?;
        }

        let mut resulting_steps = Vec::with_capacity(step_updates.len());
        for (index, (maybe_id, methods)) in step_updates.into_iter().enumerate() {
            let position = index as i32;
            let id = if let Some(step_id) = maybe_id {
                MfaFlowStep::update_single(&mut *conn, flow_id, step_id, position, &methods)
                    .await?;
                step_id
            } else {
                MfaFlowStep::insert_single(&mut *conn, flow_id, position, &methods).await?
            };

            resulting_steps.push(MfaFlowStep {
                id,
                flow_id,
                position,
                methods,
            });
        }

        let flow = MfaFlow::find_by_id(&mut *conn, flow_id)
            .await?
            .expect("flow was just updated");

        Ok((flow, resulting_steps))
    }

    /// Replaces all MFA flow assignments for a location.
    pub async fn assign_to_location(
        conn: &mut PgConnection,
        location_id: Id,
        assignments: &[LocationMfaFlowAssignment],
    ) -> Result<(), MfaFlowAssignmentError> {
        let mfa_enabled: bool = query_scalar!(
            "SELECT mfa_enabled FROM wireguard_network WHERE id = $1",
            location_id,
        )
        .fetch_one(&mut *conn)
        .await?;

        // Clearing a location's assignment list is only valid while MFA is off: an MFA-enabled
        // location must always carry a default to enforce, but a disabled one has nothing to
        // protect, and clearing is the only way to take it back from "has assignments" to "has
        // none". An empty list has zero defaults, so without this early return it would be
        // refused as `NoDefaultDesignated`.
        if assignments.is_empty() && !mfa_enabled {
            query!(
                "DELETE FROM location_mfa_flow WHERE location_id = $1",
                location_id,
            )
            .execute(&mut *conn)
            .await?;
            return Ok(());
        }

        // Exactly one assignment must be flagged as the default, at every licence tier, and it is
        // never inferred from position or from being the only entry.
        let default_count = assignments.iter().filter(|a| a.is_default).count();
        match default_count {
            1 => {}
            0 => return Err(MfaFlowAssignmentError::NoDefaultDesignated),
            _ => return Err(MfaFlowAssignmentError::MultipleDefaultsDesignated),
        }
        if let Some(default) = assignments.iter().find(|a| a.is_default)
            && !default.group_ids.is_empty()
        {
            return Err(MfaFlowAssignmentError::DefaultHasGroups);
        }

        // `location_mfa_flow` is keyed on (location_id, flow_id), so a repeated flow would raise a
        // primary-key violation. Reject it as bad input instead of surfacing a 500.
        let mut seen_flows = HashSet::new();
        for a in assignments {
            if !seen_flows.insert(a.flow_id) {
                return Err(MfaFlowAssignmentError::DuplicateFlow(a.flow_id));
            }
        }

        // The mirror of the default rule: a non-default assignment scoped to no groups can never
        // overlap any user, so it can never match and would be inert. Reject it rather than let an
        // admin save an assignment that never fires.
        if let Some(inert) = assignments
            .iter()
            .find(|a| !a.is_default && a.group_ids.is_empty())
        {
            return Err(MfaFlowAssignmentError::NonDefaultWithoutGroups(
                inert.flow_id,
            ));
        }

        // Referenced flows and groups must exist, otherwise the INSERTs below fail on a foreign
        // key and the caller sees a 500 rather than a validation error.
        // `FOR SHARE` holds the referenced flows against concurrent deletion for the rest of this
        // transaction, and conversely blocks here while a delete of one of them is in flight. See
        // the note on `check_deletable`. If that delete commits first, the row is gone by the time
        // this query runs and the caller gets `UnknownFlow` rather than a foreign-key 500.
        let flow_ids: Vec<Id> = assignments.iter().map(|a| a.flow_id).collect();
        let existing_flows: HashSet<Id> = query_scalar!(
            "SELECT id FROM mfa_flow WHERE id = ANY($1) FOR SHARE",
            &flow_ids
        )
        .fetch_all(&mut *conn)
        .await?
        .into_iter()
        .collect();
        if let Some(missing) = flow_ids.iter().find(|id| !existing_flows.contains(id)) {
            return Err(MfaFlowAssignmentError::UnknownFlow(*missing));
        }

        let group_ids: Vec<Id> = assignments
            .iter()
            .flat_map(|a| a.group_ids.iter().copied())
            .collect();
        if !group_ids.is_empty() {
            let existing_groups: HashSet<Id> =
                query_scalar!("SELECT id FROM \"group\" WHERE id = ANY($1)", &group_ids)
                    .fetch_all(&mut *conn)
                    .await?
                    .into_iter()
                    .collect();
            if let Some(missing) = group_ids.iter().find(|id| !existing_groups.contains(id)) {
                return Err(MfaFlowAssignmentError::UnknownGroup(*missing));
            }
        }

        query!(
            "DELETE FROM location_mfa_flow WHERE location_id = $1",
            location_id,
        )
        .execute(&mut *conn)
        .await?;

        for (i, a) in assignments.iter().enumerate() {
            let position = i as i32;
            query!(
                "INSERT INTO location_mfa_flow (location_id, flow_id, position, is_default) \
                 VALUES ($1, $2, $3, $4)",
                location_id,
                a.flow_id,
                position,
                a.is_default,
            )
            .execute(&mut *conn)
            .await?;

            if !a.group_ids.is_empty() {
                query!(
                    "INSERT INTO location_mfa_flow_group (location_id, flow_id, group_id) \
                     SELECT $1, $2, unnest($3::bigint[])",
                    location_id,
                    a.flow_id,
                    &a.group_ids,
                )
                .execute(&mut *conn)
                .await?;
            }
        }

        Ok(())
    }

    /// Returns the enriched assignment list for a location, ordered by position.
    pub async fn for_location<'e, E: PgExecutor<'e>>(
        executor: E,
        location_id: Id,
    ) -> sqlx::Result<Vec<LocationMfaFlowItem>> {
        query_as!(
            LocationMfaFlowItem,
            "SELECT mf.id, mf.title, \
             COALESCE(s.step_count, 0) AS \"step_count!: i64\", \
             COALESCE(array_agg(g.name ORDER BY g.name) \
                      FILTER (WHERE g.name IS NOT NULL), '{}') \
                      AS \"group_names!: Vec<String>\", \
             lmf.position, lmf.is_default \
             FROM location_mfa_flow lmf \
             JOIN mfa_flow mf ON mf.id = lmf.flow_id \
             LEFT JOIN ( \
                 SELECT flow_id, COUNT(*) AS step_count \
                 FROM mfa_flow_step \
                 GROUP BY flow_id \
             ) s ON s.flow_id = mf.id \
             LEFT JOIN location_mfa_flow_group lmfg \
                 ON lmfg.location_id = lmf.location_id \
                 AND lmfg.flow_id = lmf.flow_id \
             LEFT JOIN \"group\" g ON g.id = lmfg.group_id \
             WHERE lmf.location_id = $1 \
             GROUP BY mf.id, mf.title, s.step_count, lmf.position, lmf.is_default \
             ORDER BY lmf.position",
            location_id
        )
        .fetch_all(executor)
        .await
    }

    /// Checks whether a flow can be deleted, returning an error naming the
    /// affected locations if it cannot.
    ///
    /// [`MfaFlowDeleteError::LocationRequiresFlow`] is scoped to MFA-enabled locations: a flow
    /// that is the only assignment for an MFA-disabled location can be deleted.
    ///
    /// Must be called on the same connection as the subsequent DELETE, and inside its
    /// transaction: the checks below and the delete have to be atomic with respect to
    /// [`Self::assign_to_location`], or a concurrent assignment could make this flow a location's
    /// sole default in the window between checking and deleting, leaving that location with no
    /// MFA policy. The `FOR UPDATE` below takes the flow-identity lock that
    /// `assign_to_location` contends on with `FOR SHARE`.
    pub async fn check_deletable(
        conn: &mut PgConnection,
        flow_id: Id,
    ) -> Result<(), MfaFlowDeleteError> {
        query_scalar!("SELECT id FROM mfa_flow WHERE id = $1 FOR UPDATE", flow_id)
            .fetch_optional(&mut *conn)
            .await?;

        // Flow is the only assignment for any MFA-enabled location?
        let orphaned: Vec<String> = query_scalar!(
            "SELECT wn.name \
             FROM location_mfa_flow lmf \
             JOIN wireguard_network wn ON wn.id = lmf.location_id \
             WHERE lmf.flow_id = $1 \
             AND wn.mfa_enabled = true \
             AND (SELECT COUNT(*) FROM location_mfa_flow \
                  WHERE location_id = lmf.location_id) = 1",
            flow_id
        )
        .fetch_all(&mut *conn)
        .await?;

        if !orphaned.is_empty() {
            return Err(MfaFlowDeleteError::LocationRequiresFlow(orphaned));
        }

        // Flow is the designated default for any location?
        let defaults: Vec<String> = query_scalar!(
            "SELECT wn.name \
             FROM location_mfa_flow lmf \
             JOIN wireguard_network wn ON wn.id = lmf.location_id \
             WHERE lmf.flow_id = $1 AND lmf.is_default = true",
            flow_id
        )
        .fetch_all(&mut *conn)
        .await?;

        if !defaults.is_empty() {
            return Err(MfaFlowDeleteError::FlowIsDefault(defaults));
        }

        Ok(())
    }

    /// Resolves the MFA flow that applies to a user at a location.
    ///
    /// Ordered first-match over `location_mfa_flow.position`: the first assignment whose group set
    /// intersects the user's groups wins, otherwise the assignment flagged `is_default` wins.
    /// Because the default is mandatory and carries an empty group set, a location that has a
    /// policy always resolves, so "user matches nothing" is unrepresentable.
    ///
    /// `None` is therefore not a resolution failure but an absence of policy, and callers must
    /// **fail closed** on it rather than treat it as "no MFA required". It occurs in exactly two
    /// cases:
    ///
    /// 1. The location has no assignments at all. This is legitimate and transient: a location can
    ///    be `mfa_enabled` before its policy has been built.
    /// 2. The location has assignments but none is flagged default. The API makes this
    ///    unreachable, since [`Self::assign_to_location`] rejects it and [`Self::check_deletable`]
    ///    refuses to remove a default, so this arm only guards data predating those rules.
    pub async fn resolve_for_user(
        executor: &mut PgConnection,
        location_id: Id,
        user_id: Id,
    ) -> sqlx::Result<Option<(MfaFlow<Id>, Vec<MfaFlowStep<Id>>)>> {
        let assignments = query_as!(
            ResolveAssignmentRow,
            "SELECT lmf.flow_id, lmf.is_default, \
             COALESCE(array_agg(lmfg.group_id) \
                      FILTER (WHERE lmfg.group_id IS NOT NULL), '{}') \
                      AS \"group_ids!: Vec<Id>\" \
             FROM location_mfa_flow lmf \
             LEFT JOIN location_mfa_flow_group lmfg \
                 ON lmfg.location_id = lmf.location_id \
                 AND lmfg.flow_id = lmf.flow_id \
             WHERE lmf.location_id = $1 \
             GROUP BY lmf.flow_id, lmf.position, lmf.is_default \
             ORDER BY lmf.position",
            location_id
        )
        .fetch_all(&mut *executor)
        .await?;

        if assignments.is_empty() {
            return Ok(None);
        }

        let user_groups: HashSet<Id> = query_scalar!(
            "SELECT group_id FROM group_user WHERE user_id = $1",
            user_id
        )
        .fetch_all(&mut *executor)
        .await?
        .into_iter()
        .flatten()
        .collect();

        let mut default_flow_id: Option<Id> = None;
        for assignment in &assignments {
            if assignment.is_default {
                default_flow_id = Some(assignment.flow_id);
            } else if assignment
                .group_ids
                .iter()
                .any(|group_id| user_groups.contains(group_id))
            {
                let flow = MfaFlow::find_by_id(&mut *executor, assignment.flow_id)
                    .await?
                    .expect("flow referenced by assignment must exist");
                let steps = MfaFlowStep::find_by_flow(&mut *executor, assignment.flow_id).await?;
                return Ok(Some((flow, steps)));
            }
        }

        if let Some(flow_id) = default_flow_id {
            let flow = MfaFlow::find_by_id(&mut *executor, flow_id)
                .await?
                .expect("default flow must exist");
            let steps = MfaFlowStep::find_by_flow(&mut *executor, flow_id).await?;
            return Ok(Some((flow, steps)));
        }

        Ok(None)
    }

    /// Derives the legacy `LocationMfaMode` for a location.
    ///
    /// `mfa_enabled` is the authoritative flag and is checked first: when `false` the location
    /// is MFA-off, which is legacy-representable as `Disabled`. When `true`, the function
    /// inspects the flow configuration and returns the matching legacy mode when it is
    /// backward-compatible (single-flow, single-step, full internal set or OIDC only). Returns
    /// `None` when the location uses multi-flow, multi-step, or subset-of-internal-methods
    /// configurations that legacy clients cannot represent.
    ///
    /// The invariant this guarantees: a location with `mfa_enabled = false` is never advertised
    /// to any client as MFA-required.
    pub async fn derive_legacy_mode<'e, E: PgExecutor<'e>>(
        executor: E,
        location_id: Id,
    ) -> sqlx::Result<Option<LocationMfaMode>> {
        // Fetch mfa_enabled and step methods in one query so the executor is consumed only once.
        struct DeriveRow {
            mfa_enabled: bool,
            methods: Option<Vec<VpnClientMfaMethod>>,
        }

        let rows = query_as!(
            DeriveRow,
            "SELECT wn.mfa_enabled AS \"mfa_enabled!: bool\", \
             mfs.methods AS \"methods?: Vec<VpnClientMfaMethod>\" \
             FROM wireguard_network wn \
             LEFT JOIN location_mfa_flow lmf ON lmf.location_id = wn.id \
             LEFT JOIN mfa_flow_step mfs ON mfs.flow_id = lmf.flow_id \
             WHERE wn.id = $1 \
             ORDER BY lmf.position, mfs.position",
            location_id
        )
        .fetch_all(executor)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        // mfa_enabled is the authoritative flag. When false the location is MFA-off, which is
        // legacy-representable as Disabled rather than omitted.
        if !rows[0].mfa_enabled {
            return Ok(Some(LocationMfaMode::Disabled));
        }

        // Collect step rows that actually have methods (NULL for locations with no flows).
        let steps: Vec<&Vec<VpnClientMfaMethod>> =
            rows.iter().filter_map(|r| r.methods.as_ref()).collect();

        if steps.len() != 1 {
            return Ok(None);
        }

        let methods = steps[0];
        let set: HashSet<VpnClientMfaMethod> = methods.iter().copied().collect();

        let all_internal: HashSet<VpnClientMfaMethod> = [
            VpnClientMfaMethod::Totp,
            VpnClientMfaMethod::Email,
            VpnClientMfaMethod::Biometric,
            VpnClientMfaMethod::MobileApprove,
        ]
        .into();

        if set == all_internal {
            return Ok(Some(LocationMfaMode::Internal));
        }

        if set == HashSet::from([VpnClientMfaMethod::Oidc]) {
            return Ok(Some(LocationMfaMode::External));
        }

        Ok(None)
    }
}

impl MfaFlowStep<NoId> {
    /// Inserts a single step and returns its assigned id.
    pub async fn insert_single(
        conn: &mut PgConnection,
        flow_id: Id,
        position: i32,
        methods: &[VpnClientMfaMethod],
    ) -> sqlx::Result<Id> {
        let id = query_scalar!(
            "INSERT INTO mfa_flow_step (flow_id, position, methods) \
             VALUES ($1, $2, $3::vpn_client_mfa_method[]) RETURNING id",
            flow_id,
            position,
            methods as &[VpnClientMfaMethod],
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(id)
    }

    /// Inserts a batch of steps for a flow, assigning contiguous 0-based positions
    /// from the outer array order.
    pub async fn insert_batch(
        conn: &mut PgConnection,
        flow_id: Id,
        step_methods: &[Vec<VpnClientMfaMethod>],
    ) -> sqlx::Result<Vec<MfaFlowStep<Id>>> {
        let mut steps = Vec::with_capacity(step_methods.len());
        for (i, methods) in step_methods.iter().enumerate() {
            let id = Self::insert_single(&mut *conn, flow_id, i as i32, methods).await?;

            steps.push(MfaFlowStep {
                id,
                flow_id,
                position: i as i32,
                methods: methods.clone(),
            });
        }
        Ok(steps)
    }
}

impl MfaFlowStep<Id> {
    /// Returns all steps for a given flow, ordered by position.
    pub async fn find_by_flow<'e, E: PgExecutor<'e>>(
        executor: E,
        flow_id: Id,
    ) -> sqlx::Result<Vec<MfaFlowStep<Id>>> {
        query_as!(
            MfaFlowStep,
            "SELECT id, flow_id, position, \
             methods AS \"methods: Vec<VpnClientMfaMethod>\" \
             FROM mfa_flow_step \
             WHERE flow_id = $1 \
             ORDER BY position",
            flow_id
        )
        .fetch_all(executor)
        .await
    }

    /// Deletes all steps for a given flow except those whose id is in `keep_ids`.
    pub async fn delete_by_flow_except(
        conn: &mut PgConnection,
        flow_id: Id,
        keep_ids: &[Id],
    ) -> sqlx::Result<()> {
        query!(
            "DELETE FROM mfa_flow_step \
             WHERE flow_id = $1 AND id != ALL($2::bigint[])",
            flow_id,
            keep_ids,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// Offsets the position of the given steps by `offset` to make room for a swap.
    pub async fn offset_positions(
        conn: &mut PgConnection,
        flow_id: Id,
        offset: i32,
        step_ids: &[Id],
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE mfa_flow_step \
             SET position = position + $2 \
             WHERE flow_id = $1 AND id = ANY($3::bigint[])",
            flow_id,
            offset,
            step_ids,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// Updates the position and methods of a single step.
    ///
    /// Scoped by `flow_id` so a step id belonging to another flow can never be written through
    /// this path, even if a caller skips the ownership check in `update_with_steps`.
    pub async fn update_single(
        conn: &mut PgConnection,
        flow_id: Id,
        step_id: Id,
        position: i32,
        methods: &[VpnClientMfaMethod],
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE mfa_flow_step \
             SET position = $1, methods = $2::vpn_client_mfa_method[] \
             WHERE id = $3 AND flow_id = $4",
            position,
            methods as &[VpnClientMfaMethod],
            step_id,
            flow_id,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// Deletes all steps for a given flow.
    pub async fn delete_by_flow(conn: &mut PgConnection, flow_id: Id) -> sqlx::Result<()> {
        query!("DELETE FROM mfa_flow_step WHERE flow_id = $1", flow_id)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
