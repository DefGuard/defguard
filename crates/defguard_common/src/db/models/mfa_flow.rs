use std::collections::HashSet;

use chrono::{DateTime, Utc};
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

/// Errors that can occur during MFA flow assignment.
#[derive(Debug, Error)]
pub enum MfaFlowAssignmentError {
    #[error("No default MFA flow designated for this location")]
    NoDefaultDesignated,
    #[error("More than one MFA flow designated as the default for this location")]
    MultipleDefaultsDesignated,
    #[error("The default MFA flow assignment must not be scoped to any groups")]
    DefaultHasGroups,
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
    }

    if step_methods.is_empty() {
        errors.push(MfaFlowValidationField {
            field: "steps".into(),
            code: "min_items".into(),
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
/// intermediate positions never conflict.
pub const POSITION_SWAP_OFFSET: i32 = 10_000;

impl MfaFlow<NoId> {
    /// Creates a new flow with its steps in a single transaction.
    /// `step_methods` is one `Vec` per step; positions are assigned 0-based
    /// from the outer array order.
    pub async fn create(
        conn: &mut PgConnection,
        title: String,
        step_methods: Vec<Vec<VpnClientMfaMethod>>,
    ) -> sqlx::Result<(MfaFlow<Id>, Vec<MfaFlowStep<Id>>)> {
        let now = Utc::now();
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
        query!(
            "UPDATE mfa_flow SET title = $1, updated_at = now() WHERE id = $2",
            title,
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
    ) -> sqlx::Result<(MfaFlow<Id>, Vec<MfaFlowStep<Id>>)> {
        let incoming_ids: Vec<Id> = step_updates.iter().filter_map(|(id, _)| *id).collect();

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
                MfaFlowStep::update_single(&mut *conn, step_id, position, &methods).await?;
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
    pub async fn check_deletable<'e, E: PgExecutor<'e> + Copy>(
        executor: E,
        flow_id: Id,
    ) -> Result<(), MfaFlowDeleteError> {
        // Flow is the only assignment for any location?
        let orphaned: Vec<String> = query_scalar!(
            "SELECT wn.name \
             FROM location_mfa_flow lmf \
             JOIN wireguard_network wn ON wn.id = lmf.location_id \
             WHERE lmf.flow_id = $1 \
             AND (SELECT COUNT(*) FROM location_mfa_flow \
                  WHERE location_id = lmf.location_id) = 1",
            flow_id
        )
        .fetch_all(executor)
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
        .fetch_all(executor)
        .await?;

        if !defaults.is_empty() {
            return Err(MfaFlowDeleteError::FlowIsDefault(defaults));
        }

        Ok(())
    }

    /// Resolves the first matching MFA flow for a user at a location.
    pub async fn resolve_for_user<'e>(
        executor: impl PgExecutor<'e> + Copy,
        location_id: Id,
        user_id: Id,
    ) -> sqlx::Result<Option<(MfaFlow<Id>, Vec<MfaFlowStep<Id>>)>> {
        use std::collections::HashSet;

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
        .fetch_all(executor)
        .await?;

        if assignments.is_empty() {
            return Ok(None);
        }

        let user_groups: HashSet<Id> = query_scalar!(
            "SELECT group_id FROM group_user WHERE user_id = $1",
            user_id
        )
        .fetch_all(executor)
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
                let flow = MfaFlow::find_by_id(executor, assignment.flow_id)
                    .await?
                    .expect("flow referenced by assignment must exist");
                let steps = MfaFlowStep::find_by_flow(executor, assignment.flow_id).await?;
                return Ok(Some((flow, steps)));
            }
        }

        if let Some(flow_id) = default_flow_id {
            let flow = MfaFlow::find_by_id(executor, flow_id)
                .await?
                .expect("default flow must exist");
            let steps = MfaFlowStep::find_by_flow(executor, flow_id).await?;
            return Ok(Some((flow, steps)));
        }

        Ok(None)
    }

    /// Derives the legacy `LocationMfaMode` for a location if the current
    /// flow configuration is backward-compatible. Returns `None` when the
    /// location uses multi-flow, multi-step, or subset-of-internal-methods
    /// configurations that legacy clients cannot represent.
    pub async fn derive_legacy_mode<'e, E: PgExecutor<'e>>(
        executor: E,
        location_id: Id,
    ) -> sqlx::Result<Option<LocationMfaMode>> {
        struct StepRow {
            methods: Vec<VpnClientMfaMethod>,
        }

        let steps = query_as!(
            StepRow,
            "SELECT mfs.methods AS \"methods: Vec<VpnClientMfaMethod>\" \
             FROM location_mfa_flow lmf \
             JOIN mfa_flow_step mfs ON mfs.flow_id = lmf.flow_id \
             WHERE lmf.location_id = $1 \
             ORDER BY lmf.position, mfs.position",
            location_id
        )
        .fetch_all(executor)
        .await?;

        if steps.len() != 1 {
            return Ok(None);
        }

        let methods = &steps[0].methods;
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

/// Internal row type for the resolve_for_user query.
struct ResolveAssignmentRow {
    flow_id: Id,
    is_default: bool,
    group_ids: Vec<Id>,
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
            let id = query_scalar!(
                "INSERT INTO mfa_flow_step (flow_id, position, methods) \
                 VALUES ($1, $2, $3::vpn_client_mfa_method[]) RETURNING id",
                flow_id,
                i as i32,
                methods as &[VpnClientMfaMethod],
            )
            .fetch_one(&mut *conn)
            .await?;

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
    pub async fn update_single(
        conn: &mut PgConnection,
        step_id: Id,
        position: i32,
        methods: &[VpnClientMfaMethod],
    ) -> sqlx::Result<()> {
        query!(
            "UPDATE mfa_flow_step \
             SET position = $1, methods = $2::vpn_client_mfa_method[] \
             WHERE id = $3",
            position,
            methods as &[VpnClientMfaMethod],
            step_id,
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
