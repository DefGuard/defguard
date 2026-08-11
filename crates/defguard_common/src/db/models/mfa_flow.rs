use chrono::{DateTime, Utc};
use model_derive::Model;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection, PgExecutor, query, query_as, query_scalar};
use utoipa::ToSchema;

use crate::db::{Id, NoId, models::vpn_client_session::VpnClientMfaMethod};

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
        const OFFSET: i32 = 10_000;

        let now = Utc::now();
        query!(
            "UPDATE mfa_flow SET title = $1, updated_at = $2 WHERE id = $3",
            title,
            now,
            flow_id,
        )
        .execute(&mut *conn)
        .await?;

        let incoming_ids: Vec<Id> = step_updates.iter().filter_map(|(id, _)| *id).collect();

        if incoming_ids.is_empty() {
            query!("DELETE FROM mfa_flow_step WHERE flow_id = $1", flow_id,)
                .execute(&mut *conn)
                .await?;
        } else {
            query!(
                "DELETE FROM mfa_flow_step \
                 WHERE flow_id = $1 AND id != ALL($2::bigint[])",
                flow_id,
                &incoming_ids,
            )
            .execute(&mut *conn)
            .await?;

            query!(
                "UPDATE mfa_flow_step \
                 SET position = position + $2 \
                 WHERE flow_id = $1 AND id = ANY($3::bigint[])",
                flow_id,
                OFFSET,
                &incoming_ids,
            )
            .execute(&mut *conn)
            .await?;
        }

        let mut resulting_steps = Vec::with_capacity(step_updates.len());
        for (index, (maybe_id, methods)) in step_updates.into_iter().enumerate() {
            let position = index as i32;
            let id = if let Some(step_id) = maybe_id {
                query!(
                    "UPDATE mfa_flow_step \
                     SET position = $1, methods = $2::vpn_client_mfa_method[] \
                     WHERE id = $3",
                    position,
                    &methods as &[VpnClientMfaMethod],
                    step_id,
                )
                .execute(&mut *conn)
                .await?;
                step_id
            } else {
                let new_id = query_scalar!(
                    "INSERT INTO mfa_flow_step (flow_id, position, methods) \
                     VALUES ($1, $2, $3::vpn_client_mfa_method[]) RETURNING id",
                    flow_id,
                    position,
                    &methods as &[VpnClientMfaMethod],
                )
                .fetch_one(&mut *conn)
                .await?;
                new_id
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
}

impl MfaFlowStep<NoId> {
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
