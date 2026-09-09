use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::db::{
    models::{
        group::Group,
        user::User,
        wireguard::{LocationMfaMode, WireguardNetwork},
    },
    setup_pool,
};

/// Helper: create a flow with two steps and return its (flow, steps).
async fn create_flow(pool: &sqlx::PgPool) -> (MfaFlow<Id>, Vec<MfaFlowStep<Id>>) {
    let mut tx = pool.begin().await.unwrap();
    let (flow, steps) = MfaFlow::create(
        &mut tx,
        "Test Flow".into(),
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (flow, steps)
}

/// Assign two group-scoped entries and one default in a single full-replace call, used by the
/// resolution-ordering tests. Each `(flow_id, group_id)` pair fixes one entry's scope.
async fn assign_three(
    pool: &sqlx::PgPool,
    location_id: Id,
    first: (Id, Id),
    second: (Id, Id),
    default: Id,
) {
    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        location_id,
        &[
            LocationMfaFlowAssignment {
                flow_id: first.0,
                is_default: false,
                group_ids: vec![first.1],
            },
            LocationMfaFlowAssignment {
                flow_id: second.0,
                is_default: false,
                group_ids: vec![second.1],
            },
            LocationMfaFlowAssignment {
                flow_id: default,
                is_default: true,
                group_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

#[sqlx::test]
async fn test_insert_new_step(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, original_steps) = create_flow(&pool).await;
    assert_eq!(original_steps.len(), 2);

    let mut tx = pool.begin().await.unwrap();
    let (_, updated_steps) = MfaFlow::update_with_steps(
        &mut tx,
        flow.id,
        "Test Flow".into(),
        vec![
            (Some(original_steps[0].id), vec![VpnClientMfaMethod::Totp]),
            (Some(original_steps[1].id), vec![VpnClientMfaMethod::Email]),
            (None, vec![VpnClientMfaMethod::Oidc]),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(updated_steps.len(), 3);
    assert_eq!(updated_steps[0].methods, vec![VpnClientMfaMethod::Totp]);
    assert_eq!(updated_steps[1].methods, vec![VpnClientMfaMethod::Email]);
    assert_eq!(updated_steps[2].methods, vec![VpnClientMfaMethod::Oidc]);
    assert_eq!(updated_steps[0].position, 0);
    assert_eq!(updated_steps[1].position, 1);
    assert_eq!(updated_steps[2].position, 2);
}

#[sqlx::test]
async fn test_update_kept_step(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, original_steps) = create_flow(&pool).await;
    assert_eq!(original_steps.len(), 2);

    let mut tx = pool.begin().await.unwrap();
    let (_, updated_steps) = MfaFlow::update_with_steps(
        &mut tx,
        flow.id,
        "Renamed Flow".into(),
        vec![
            (Some(original_steps[0].id), vec![VpnClientMfaMethod::Totp]),
            (
                Some(original_steps[1].id),
                vec![
                    VpnClientMfaMethod::Biometric,
                    VpnClientMfaMethod::MobileApprove,
                ],
            ),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(updated_steps.len(), 2);
    assert_eq!(updated_steps[0].methods, vec![VpnClientMfaMethod::Totp]);
    assert_eq!(
        updated_steps[1].methods,
        vec![
            VpnClientMfaMethod::Biometric,
            VpnClientMfaMethod::MobileApprove
        ]
    );

    let flow = MfaFlow::find_by_id(&pool, flow.id).await.unwrap().unwrap();
    assert_eq!(flow.title, "Renamed Flow");
}

#[sqlx::test]
async fn test_delete_removed_step(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, original_steps) = create_flow(&pool).await;
    assert_eq!(original_steps.len(), 2);

    // Add a third step at position 2 (the flow already has steps at positions 0 and 1, so this
    // must not collide with the `UNIQUE (flow_id, position)` constraint).
    let mut tx = pool.begin().await.unwrap();
    MfaFlowStep::insert_single(&mut tx, flow.id, 2, &[VpnClientMfaMethod::Oidc])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let all_steps = MfaFlowStep::find_by_flow(&pool, flow.id).await.unwrap();
    assert_eq!(all_steps.len(), 3);

    // Update: keep steps 0 and 2, delete step 1
    let mut tx = pool.begin().await.unwrap();
    let (_, updated_steps) = MfaFlow::update_with_steps(
        &mut tx,
        flow.id,
        "Test Flow".into(),
        vec![
            (Some(all_steps[0].id), all_steps[0].methods.clone()),
            (Some(all_steps[2].id), all_steps[2].methods.clone()),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(updated_steps.len(), 2);
    assert_eq!(updated_steps[0].id, all_steps[0].id);
    assert_eq!(updated_steps[1].id, all_steps[2].id);
    assert_eq!(updated_steps[0].position, 0);
    assert_eq!(updated_steps[1].position, 1);

    let db_steps = MfaFlowStep::find_by_flow(&pool, flow.id).await.unwrap();
    assert_eq!(db_steps.len(), 2);
    let db_ids: Vec<Id> = db_steps.iter().map(|s| s.id).collect();
    assert!(!db_ids.contains(&all_steps[1].id));
}

#[sqlx::test]
async fn test_position_swap(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, original_steps) = create_flow(&pool).await;
    assert_eq!(original_steps.len(), 2);
    let step0_id = original_steps[0].id;
    let step0_methods = original_steps[0].methods.clone();
    let step1_id = original_steps[1].id;
    let step1_methods = original_steps[1].methods.clone();

    let mut tx = pool.begin().await.unwrap();
    let (_, updated_steps) = MfaFlow::update_with_steps(
        &mut tx,
        flow.id,
        "Test Flow".into(),
        vec![
            (Some(step1_id), step1_methods.clone()),
            (Some(step0_id), step0_methods.clone()),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(updated_steps.len(), 2);
    assert_eq!(updated_steps[0].id, step1_id);
    assert_eq!(updated_steps[0].methods, step1_methods);
    assert_eq!(updated_steps[0].position, 0);
    assert_eq!(updated_steps[1].id, step0_id);
    assert_eq!(updated_steps[1].methods, step0_methods);
    assert_eq!(updated_steps[1].position, 1);
}

/// A three-step reorder still satisfies `UNIQUE (flow_id, position)`: the offset-into-disjoint-
/// range reconciliation must never leave two steps sharing a position inside the transaction.
#[sqlx::test]
async fn test_position_reorder_three_steps(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (flow, steps) = MfaFlow::create(
        &mut tx,
        "Reorder".into(),
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
            vec![VpnClientMfaMethod::Biometric],
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(steps.len(), 3);

    // Reverse the order.
    let mut tx = pool.begin().await.unwrap();
    let (_, updated) = MfaFlow::update_with_steps(
        &mut tx,
        flow.id,
        "Reorder".into(),
        vec![
            (Some(steps[2].id), steps[2].methods.clone()),
            (Some(steps[1].id), steps[1].methods.clone()),
            (Some(steps[0].id), steps[0].methods.clone()),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(updated[0].id, steps[2].id);
    assert_eq!(updated[0].position, 0);
    assert_eq!(updated[1].id, steps[1].id);
    assert_eq!(updated[1].position, 1);
    assert_eq!(updated[2].id, steps[0].id);
    assert_eq!(updated[2].position, 2);
}

#[sqlx::test]
async fn test_assign_to_location(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (flow1, _) = create_flow(&pool).await;
    let (flow2, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Second Flow".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let network = WireguardNetwork::default()
        .try_set_address("10.0.0.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    let group = Group::new("assign-group").save(&pool).await.unwrap();

    // Assign two flows to the location: the non-default flow is group-scoped, the default is not.
    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[
            LocationMfaFlowAssignment {
                flow_id: flow1.id,
                is_default: false,
                group_ids: vec![group.id],
            },
            LocationMfaFlowAssignment {
                flow_id: flow2.id,
                is_default: true,
                group_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let items = MfaFlow::for_location(&pool, network.id).await.unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, flow1.id);
    assert_eq!(items[0].position, 0);
    assert!(!items[0].is_default);
    assert_eq!(items[0].group_names.len(), 1);
    assert_eq!(items[1].id, flow2.id);
    assert_eq!(items[1].position, 1);
    assert!(items[1].is_default);
    assert_eq!(items[0].step_count, 2);
    assert_eq!(items[1].step_count, 1);
}

#[sqlx::test]
async fn test_assign_to_location_full_replace(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (flow1, _) = create_flow(&pool).await;
    let (flow2, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Second Flow".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let network = WireguardNetwork::default()
        .try_set_address("10.0.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    // First assignment: flow1 only
    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow1.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Second assignment replaces: flow2 only
    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow2.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let items = MfaFlow::for_location(&pool, network.id).await.unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, flow2.id);
}

#[sqlx::test]
async fn test_assign_no_default_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow1, _) = create_flow(&pool).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.2.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let result = MfaFlow::assign_to_location(
        &mut pool.acquire().await.unwrap(),
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow1.id,
            is_default: false,
            group_ids: Vec::new(),
        }],
    )
    .await;
    assert!(matches!(
        result,
        Err(MfaFlowAssignmentError::NoDefaultDesignated)
    ));
}

/// Two assignments both flagged default is a distinct failure from none being flagged, so it must
/// not be reported as `no_default_designated`.
#[sqlx::test]
async fn test_assign_multiple_defaults_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow1, _) = create_flow(&pool).await;
    let (flow2, _) = create_flow(&pool).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.7.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let result = MfaFlow::assign_to_location(
        &mut pool.acquire().await.unwrap(),
        network.id,
        &[
            LocationMfaFlowAssignment {
                flow_id: flow1.id,
                is_default: true,
                group_ids: Vec::new(),
            },
            LocationMfaFlowAssignment {
                flow_id: flow2.id,
                is_default: true,
                group_ids: Vec::new(),
            },
        ],
    )
    .await;
    assert!(matches!(
        result,
        Err(MfaFlowAssignmentError::MultipleDefaultsDesignated)
    ));

    // The rejected save must not have partially applied.
    let assignments = MfaFlow::for_location(&pool, network.id).await.unwrap();
    assert!(assignments.is_empty());
}

#[sqlx::test]
async fn test_assign_default_with_groups_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow1, _) = create_flow(&pool).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.3.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let result = MfaFlow::assign_to_location(
        &mut pool.acquire().await.unwrap(),
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow1.id,
            is_default: true,
            group_ids: vec![flow1.id], // default must have empty groups
        }],
    )
    .await;
    assert!(matches!(
        result,
        Err(MfaFlowAssignmentError::DefaultHasGroups)
    ));
}

/// An MFA-disabled location can have its assignment list cleared: there is nothing to enforce, so
/// an empty list is a valid (re)configuration rather than a missing default.
#[sqlx::test]
async fn test_assign_clear_disabled_location(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, _) = create_flow(&pool).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.9.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    // network.mfa_enabled is false (default).

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    MfaFlow::assign_to_location(&mut tx, network.id, &[])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let items = MfaFlow::for_location(&pool, network.id).await.unwrap();
    assert!(items.is_empty(), "clearing must remove all assignments");
}

/// Clearing an MFA-enabled location's assignment list is still refused: such a location must keep
/// something to enforce.
#[sqlx::test]
async fn test_assign_clear_enabled_location_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut network = WireguardNetwork::default()
        .try_set_address("10.0.10.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let result =
        MfaFlow::assign_to_location(&mut pool.acquire().await.unwrap(), network.id, &[]).await;
    assert!(matches!(
        result,
        Err(MfaFlowAssignmentError::NoDefaultDesignated)
    ));
}

/// A non-default assignment scoped to no groups can never match any user, so it is refused rather
/// than saved as an assignment that never fires. This is the mirror of `DefaultHasGroups`.
#[sqlx::test]
async fn test_assign_non_default_without_groups_rejected(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (flow1, _) = create_flow(&pool).await;
    let (flow2, _) = create_flow(&pool).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.11.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let result = MfaFlow::assign_to_location(
        &mut pool.acquire().await.unwrap(),
        network.id,
        &[
            LocationMfaFlowAssignment {
                flow_id: flow1.id,
                is_default: false,
                group_ids: Vec::new(), // inert: empty group set
            },
            LocationMfaFlowAssignment {
                flow_id: flow2.id,
                is_default: true,
                group_ids: Vec::new(),
            },
        ],
    )
    .await;
    assert!(matches!(
        result,
        Err(MfaFlowAssignmentError::NonDefaultWithoutGroups(id)) if id == flow1.id
    ));

    // The rejected save must not have partially applied.
    let assignments = MfaFlow::for_location(&pool, network.id).await.unwrap();
    assert!(assignments.is_empty());
}

/// `has_default_assignment` reflects the presence of a designated default, which is what the
/// `mfa_enabled` precondition keys on.
#[sqlx::test]
async fn test_has_default_assignment(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, _) = create_flow(&pool).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.12.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    assert!(
        !MfaFlow::has_default_assignment(&pool, network.id)
            .await
            .unwrap()
    );

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert!(
        MfaFlow::has_default_assignment(&pool, network.id)
            .await
            .unwrap()
    );

    // Clearing (an MFA-disabled location) removes the default.
    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(&mut tx, network.id, &[])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert!(
        !MfaFlow::has_default_assignment(&pool, network.id)
            .await
            .unwrap()
    );
}

#[sqlx::test]
async fn test_check_deletable_location_requires_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow1, _) = create_flow(&pool).await;

    let mut network = WireguardNetwork::default()
        .try_set_address("10.0.4.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    // location_requires_flow is scoped to MFA-enabled locations.
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow1.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let result = MfaFlow::check_deletable(&mut pool.acquire().await.unwrap(), flow1.id).await;
    assert!(matches!(
        result,
        Err(MfaFlowDeleteError::LocationRequiresFlow(_))
    ));
}

/// When MFA is disabled at a location, deleting the location's only assigned,
/// non-default flow is allowed because `LocationRequiresFlow` is scoped to
/// MFA-enabled locations.
#[sqlx::test]
async fn test_check_deletable_allows_disabled_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (flow1, _) = create_flow(&pool).await;
    let (flow2, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Default".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let network = WireguardNetwork::default()
        .try_set_address("10.0.4.2/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    // network.mfa_enabled is false (default).
    let group = Group::new("disabled-group").save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[
            LocationMfaFlowAssignment {
                flow_id: flow1.id,
                is_default: false,
                group_ids: vec![group.id],
            },
            LocationMfaFlowAssignment {
                flow_id: flow2.id,
                is_default: true,
                group_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // flow1 is the only non-default assignment for an MFA-disabled location.
    let result = MfaFlow::check_deletable(&mut pool.acquire().await.unwrap(), flow1.id).await;
    assert!(
        result.is_ok(),
        "deletion should be allowed from an MFA-disabled location: {result:?}"
    );
}

#[sqlx::test]
async fn test_check_deletable_flow_is_default(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow1, _) = create_flow(&pool).await;
    let (flow2, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Second".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let network = WireguardNetwork::default()
        .try_set_address("10.0.5.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    let group = Group::new("default-group").save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[
            LocationMfaFlowAssignment {
                flow_id: flow1.id,
                is_default: true,
                group_ids: Vec::new(),
            },
            LocationMfaFlowAssignment {
                flow_id: flow2.id,
                is_default: false,
                group_ids: vec![group.id],
            },
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // flow2 is not the only assignment and not default → OK
    assert!(
        MfaFlow::check_deletable(&mut pool.acquire().await.unwrap(), flow2.id)
            .await
            .is_ok()
    );
    // flow1 is the default → refused
    let result = MfaFlow::check_deletable(&mut pool.acquire().await.unwrap(), flow1.id).await;
    assert!(matches!(result, Err(MfaFlowDeleteError::FlowIsDefault(_))));
}

/// A delete that passes its checks must not race an assignment that makes the same flow a
/// location's sole default. The two paths contend on the flow row (`FOR UPDATE` against
/// `FOR SHARE`), so the assignment cannot commit inside the delete's check-to-delete window.
#[sqlx::test]
async fn test_delete_and_assign_do_not_race(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, _) = create_flow(&pool).await;

    let network = WireguardNetwork::default()
        .try_set_address("10.0.8.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    // Transaction A: the flow is assigned nowhere, so the delete is currently allowed.
    let mut tx_delete = pool.begin().await.unwrap();
    MfaFlow::check_deletable(&mut tx_delete, flow.id)
        .await
        .expect("flow is unassigned, so deletion is permitted");

    // Transaction B tries to make it a location's default while A still holds the lock. It must
    // block rather than commit, so a short timeout has to elapse.
    let assign_pool = pool.clone();
    let flow_id = flow.id;
    let location_id = network.id;
    let mut assign = tokio::spawn(async move {
        let mut tx = assign_pool.begin().await.unwrap();
        let result = MfaFlow::assign_to_location(
            &mut tx,
            location_id,
            &[LocationMfaFlowAssignment {
                flow_id,
                is_default: true,
                group_ids: Vec::new(),
            }],
        )
        .await;
        if result.is_ok() {
            tx.commit().await.unwrap();
        }
        result
    });

    let blocked = tokio::time::timeout(std::time::Duration::from_millis(500), &mut assign).await;
    assert!(
        blocked.is_err(),
        "the assignment must block until the delete transaction finishes"
    );

    // A completes the delete it was cleared for.
    query!("DELETE FROM mfa_flow WHERE id = $1", flow.id)
        .execute(&mut *tx_delete)
        .await
        .unwrap();
    tx_delete.commit().await.unwrap();

    // B unblocks, sees the flow is gone, and refuses instead of violating the foreign key.
    let result = assign.await.unwrap();
    assert!(matches!(
        result,
        Err(MfaFlowAssignmentError::UnknownFlow(id)) if id == flow_id
    ));

    // The location was left with no assignments rather than a dangling default.
    let items = MfaFlow::for_location(&pool, network.id).await.unwrap();
    assert!(items.is_empty());
}

#[sqlx::test]
async fn test_resolve_group_match(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (flow1, _) = create_flow(&pool).await;
    let (flow2, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Default".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let user = User::new("resolver", None, "Ln", "Fn", "r@t.com", None)
        .save(&pool)
        .await
        .unwrap();
    let group = Group::new("resolver-group").save(&pool).await.unwrap();
    sqlx::query!(
        "INSERT INTO group_user (group_id, user_id) VALUES ($1, $2)",
        group.id,
        user.id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let network = WireguardNetwork::default()
        .try_set_address("10.0.6.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[
            LocationMfaFlowAssignment {
                flow_id: flow1.id,
                is_default: false,
                group_ids: vec![group.id],
            },
            LocationMfaFlowAssignment {
                flow_id: flow2.id,
                is_default: true,
                group_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut conn = pool.acquire().await.unwrap();
    let result = MfaFlow::resolve_for_user(&mut conn, network.id, user.id)
        .await
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().0.id, flow1.id);
}

#[sqlx::test]
async fn test_resolve_fallback_to_default(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (flow1, _) = create_flow(&pool).await;
    let (flow2, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Default".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let user = User::new("fallback", None, "Ln", "Fn", "f@t.com", None)
        .save(&pool)
        .await
        .unwrap();
    let group = Group::new("fb-group").save(&pool).await.unwrap();

    let network = WireguardNetwork::default()
        .try_set_address("10.0.7.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[
            LocationMfaFlowAssignment {
                flow_id: flow1.id,
                is_default: false,
                group_ids: vec![group.id],
            },
            LocationMfaFlowAssignment {
                flow_id: flow2.id,
                is_default: true,
                group_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut conn = pool.acquire().await.unwrap();
    let result = MfaFlow::resolve_for_user(&mut conn, network.id, user.id)
        .await
        .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().0.id, flow2.id);
}

/// Ordered first-match: when a user matches two group-scoped assignments, the one at the lower
/// `position` wins. The test runs resolution in both orderings and asserts the resolved flow
/// flips, so a single ordering cannot pass by accident.
#[sqlx::test]
async fn test_resolve_order_decides_between_matches(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (flow_first, _) = create_flow(&pool).await; // TOTP -> Email
    let (flow_second, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Second".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };
    let (flow_default, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Default".into(),
            vec![vec![VpnClientMfaMethod::Biometric]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let user = User::new("order-resolver", None, "Ln", "Fn", "o@t.com", None)
        .save(&pool)
        .await
        .unwrap();
    let group_a = Group::new("order-a").save(&pool).await.unwrap();
    let group_b = Group::new("order-b").save(&pool).await.unwrap();
    for gid in [group_a.id, group_b.id] {
        sqlx::query!(
            "INSERT INTO group_user (group_id, user_id) VALUES ($1, $2)",
            gid,
            user.id,
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    let network = WireguardNetwork::default()
        .try_set_address("10.3.0.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    // flow_first (scoped to group_a) at position 0, flow_second (group_b) at position 1.
    assign_three(
        &pool,
        network.id,
        (flow_first.id, group_a.id),
        (flow_second.id, group_b.id),
        flow_default.id,
    )
    .await;
    let mut conn = pool.acquire().await.unwrap();
    let resolved = MfaFlow::resolve_for_user(&mut conn, network.id, user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.0.id, flow_first.id, "lower position must win");

    // Swap the positions: the resolved flow must flip to the new lower-position entry.
    assign_three(
        &pool,
        network.id,
        (flow_second.id, group_b.id),
        (flow_first.id, group_a.id),
        flow_default.id,
    )
    .await;
    let mut conn = pool.acquire().await.unwrap();
    let resolved = MfaFlow::resolve_for_user(&mut conn, network.id, user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.0.id, flow_second.id, "swapped position must win");
}

/// A user in two groups that each match a different assignment resolves to the same flow on every
/// call: ordered first-match is deterministic.
#[sqlx::test]
async fn test_resolve_user_in_two_groups_deterministic(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let (flow_a, _) = create_flow(&pool).await;
    let (flow_b, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Second".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };
    let (flow_default, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Default".into(),
            vec![vec![VpnClientMfaMethod::Biometric]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let user = User::new("two-group-resolver", None, "Ln", "Fn", "t@t.com", None)
        .save(&pool)
        .await
        .unwrap();
    let group_a = Group::new("two-group-a").save(&pool).await.unwrap();
    let group_b = Group::new("two-group-b").save(&pool).await.unwrap();
    for gid in [group_a.id, group_b.id] {
        sqlx::query!(
            "INSERT INTO group_user (group_id, user_id) VALUES ($1, $2)",
            gid,
            user.id,
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    let network = WireguardNetwork::default()
        .try_set_address("10.3.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    assign_three(
        &pool,
        network.id,
        (flow_a.id, group_a.id),
        (flow_b.id, group_b.id),
        flow_default.id,
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();
    for _ in 0..5 {
        let resolved = MfaFlow::resolve_for_user(&mut conn, network.id, user.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resolved.0.id, flow_a.id, "resolution must be stable");
    }
}

/// `position`, not `mfa_flow.id` or creation order, decides. `flow_low_id` is created first (so it
/// has the lower id) but is assigned the higher position; `flow_high_id` is created second yet
/// wins because it holds the lower position. A stray `ORDER BY mf.id` would resolve to
/// `flow_low_id`, so this test catches it.
#[sqlx::test]
async fn test_resolve_position_not_id_order(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    // Created first: lower id, but assigned the higher position.
    let (flow_low_id, _) = create_flow(&pool).await;
    // Created second: higher id, assigned the lower position.
    let (flow_high_id, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Higher Id".into(),
            vec![vec![VpnClientMfaMethod::Oidc]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };
    let (flow_default, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(
            &mut tx,
            "Default".into(),
            vec![vec![VpnClientMfaMethod::Biometric]],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    assert!(
        flow_low_id.id < flow_high_id.id,
        "precondition: ids follow creation order"
    );

    let user = User::new("position-resolver", None, "Ln", "Fn", "p@t.com", None)
        .save(&pool)
        .await
        .unwrap();
    let group_a = Group::new("position-a").save(&pool).await.unwrap();
    let group_b = Group::new("position-b").save(&pool).await.unwrap();
    for gid in [group_a.id, group_b.id] {
        sqlx::query!(
            "INSERT INTO group_user (group_id, user_id) VALUES ($1, $2)",
            gid,
            user.id,
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    let network = WireguardNetwork::default()
        .try_set_address("10.3.2.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    // Higher-id flow at position 0, lower-id flow at position 1.
    assign_three(
        &pool,
        network.id,
        (flow_high_id.id, group_a.id),
        (flow_low_id.id, group_b.id),
        flow_default.id,
    )
    .await;

    let mut conn = pool.acquire().await.unwrap();
    let resolved = MfaFlow::resolve_for_user(&mut conn, network.id, user.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resolved.0.id, flow_high_id.id, "position must outrank id");
}

#[sqlx::test]
async fn test_derive_legacy_internal(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(
        &mut tx,
        "Internal".into(),
        vec![vec![
            VpnClientMfaMethod::Totp,
            VpnClientMfaMethod::Email,
            VpnClientMfaMethod::Biometric,
            VpnClientMfaMethod::MobileApprove,
        ]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.0.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mode = MfaFlow::derive_legacy_mode(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(mode, Some(LocationMfaMode::Internal));
}

#[sqlx::test]
async fn test_derive_legacy_external(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(
        &mut tx,
        "External".into(),
        vec![vec![VpnClientMfaMethod::Oidc]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mode = MfaFlow::derive_legacy_mode(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(mode, Some(LocationMfaMode::External));
}

#[sqlx::test]
async fn test_derive_legacy_multi_step_omitted(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, _) = create_flow(&pool).await; // 2 steps

    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.2.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mode = MfaFlow::derive_legacy_mode(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(mode, None);
}

#[sqlx::test]
async fn test_derive_legacy_internal_subset_omitted(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(
        &mut tx,
        "Subset".into(),
        vec![vec![VpnClientMfaMethod::Totp]], // only TOTP, not all four
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.3.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mode = MfaFlow::derive_legacy_mode(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(mode, None);
}

/// A location with two flows assigned is not legacy-representable: the legacy mode collapses to a
/// single flow, so `derive_legacy_mode` must omit it even though each flow is individually
/// single-step and full-internal. This is the fourth incompatible shape the ADR lists.
#[sqlx::test]
async fn test_derive_legacy_multi_flow_omitted(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let full_internal = vec![vec![
        VpnClientMfaMethod::Totp,
        VpnClientMfaMethod::Email,
        VpnClientMfaMethod::Biometric,
        VpnClientMfaMethod::MobileApprove,
    ]];

    let (flow_a, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(&mut tx, "Flow A".into(), full_internal.clone())
            .await
            .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };
    let (flow_b, _) = {
        let mut tx = pool.begin().await.unwrap();
        let (f, s) = MfaFlow::create(&mut tx, "Flow B".into(), full_internal)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        (f, s)
    };

    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.4.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let group = Group::new("multi-flow-group").save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[
            LocationMfaFlowAssignment {
                flow_id: flow_a.id,
                is_default: false,
                group_ids: vec![group.id],
            },
            LocationMfaFlowAssignment {
                flow_id: flow_b.id,
                is_default: true,
                group_ids: Vec::new(),
            },
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mode = MfaFlow::derive_legacy_mode(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(mode, None);
}

/// A location with mfa_enabled = false returns Disabled even when it has
/// flow assignments, because the stored flag is authoritative and the
/// location must never be advertised as MFA-required.
#[sqlx::test]
async fn test_derive_legacy_disabled_with_assignments(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(
        &mut tx,
        "Internal Recipe".into(),
        vec![vec![
            VpnClientMfaMethod::Totp,
            VpnClientMfaMethod::Email,
            VpnClientMfaMethod::Biometric,
            VpnClientMfaMethod::MobileApprove,
        ]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // network.mfa_enabled is false (default), but the flow is assigned.
    let network = WireguardNetwork::default()
        .try_set_address("10.2.0.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mode = MfaFlow::derive_legacy_mode(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(
        mode,
        Some(LocationMfaMode::Disabled),
        "mfa_enabled=false must derive Disabled to never advertise MFA-required"
    );
}

#[sqlx::test]
async fn test_validation_empty_title(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _pool = pool;
    let errors = validate_flow_input("  ", &[vec![VpnClientMfaMethod::Totp]]);
    assert!(
        errors
            .iter()
            .any(|e| e.field == "title" && e.code == "required")
    );
}

#[sqlx::test]
async fn test_validation_zero_steps(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _pool = pool;
    let errors = validate_flow_input("Test", &[]);
    assert!(
        errors
            .iter()
            .any(|e| e.field == "steps" && e.code == "min_items")
    );
}

#[sqlx::test]
async fn test_validation_zero_method_step(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _pool = pool;
    let errors = validate_flow_input("Test", &[Vec::new(), vec![VpnClientMfaMethod::Totp]]);
    assert!(
        errors
            .iter()
            .any(|e| e.field == "steps[0].methods" && e.code == "min_items")
    );
    // The valid step should not produce errors
    assert!(
        !errors
            .iter()
            .any(|e| e.field == "steps[1].methods" && e.code == "min_items")
    );
}

#[sqlx::test]
async fn test_validation_duplicate_method(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _pool = pool;
    let errors = validate_flow_input(
        "Test",
        &[vec![
            VpnClientMfaMethod::Totp,
            VpnClientMfaMethod::Email,
            VpnClientMfaMethod::Totp,
        ]],
    );
    assert!(
        errors
            .iter()
            .any(|e| e.field == "steps[0].methods" && e.code == "duplicate")
    );
}

#[sqlx::test]
async fn test_validation_title_too_long(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _pool = pool;
    let long_title = "x".repeat(MAX_MFA_FLOW_TITLE_LEN + 1);
    let errors = validate_flow_input(&long_title, &[vec![VpnClientMfaMethod::Totp]]);
    assert!(
        errors
            .iter()
            .any(|e| e.field == "title" && e.code == "max_length"),
        "expected max_length error for overly long title, got: {errors:?}"
    );
}

#[sqlx::test]
async fn test_validation_title_at_max_is_ok(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _pool = pool;
    let max_title = "x".repeat(MAX_MFA_FLOW_TITLE_LEN);
    let errors = validate_flow_input(&max_title, &[vec![VpnClientMfaMethod::Totp]]);
    assert!(
        !errors
            .iter()
            .any(|e| e.field == "title" && e.code == "max_length"),
        "title at max length should pass, got: {errors:?}"
    );
}

#[sqlx::test]
async fn test_validation_too_many_steps(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let _pool = pool;
    let many_steps: Vec<Vec<VpnClientMfaMethod>> = (0..=MAX_MFA_FLOW_STEPS)
        .map(|_| vec![VpnClientMfaMethod::Totp])
        .collect();
    let errors = validate_flow_input("Test", &many_steps);
    assert!(
        errors
            .iter()
            .any(|e| e.field == "steps" && e.code == "max_items"),
        "expected max_items error for too many steps, got: {errors:?}"
    );
}

/// `all_with_oidc_in_flows` must select locations whose assigned flow steps
/// include OIDC, and exclude locations that use internal-only flows.
#[sqlx::test]
async fn test_all_with_oidc_in_flows_flow_shape_predicate(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    // Create an OIDC-only flow.
    let mut tx = pool.begin().await.unwrap();
    let (oidc_flow, _) = MfaFlow::create(
        &mut tx,
        "OIDC Only".into(),
        vec![vec![VpnClientMfaMethod::Oidc]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Create an internal-only flow.
    let mut tx = pool.begin().await.unwrap();
    let (internal_flow, _) = MfaFlow::create(
        &mut tx,
        "Internal Only".into(),
        vec![vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Email]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Location A: OIDC flow, MFA enabled.
    let mut network_oidc = WireguardNetwork::default()
        .try_set_address("10.20.0.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network_oidc.mfa_enabled = true;
    network_oidc.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network_oidc.id,
        &[LocationMfaFlowAssignment {
            flow_id: oidc_flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Location B: internal flow, MFA enabled.
    let mut network_internal = WireguardNetwork::default()
        .try_set_address("10.20.1.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network_internal.mfa_enabled = true;
    network_internal.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network_internal.id,
        &[LocationMfaFlowAssignment {
            flow_id: internal_flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let external = WireguardNetwork::all_with_oidc_in_flows(&pool)
        .await
        .unwrap();
    let external_ids: Vec<_> = external.iter().map(|l| l.id).collect();
    assert!(
        external_ids.contains(&network_oidc.id),
        "OIDC-flow location {oidc_id} should be in results, got: {external_ids:?}",
        oidc_id = network_oidc.id
    );
    assert!(
        !external_ids.contains(&network_internal.id),
        "internal-only location {internal_id} must not be in results",
        internal_id = network_internal.id
    );
}

/// An MFA-disabled location whose flows reference OIDC must still be returned.
///
/// This pins a deliberate decision. Adding `WHERE wn.mfa_enabled = true` to the predicate would
/// look like a consistency fix against `instance_has_internal_mfa`, but it would silently drop this
/// case from the provider-deletion warning. Switching MFA off preserves the assignment list, and
/// re-enabling checks only that a default assignment exists, never that its methods are
/// satisfiable, so such a location is one call away from refusing every user at connect time.
#[sqlx::test]
async fn test_oidc_predicate_includes_mfa_disabled_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (oidc_flow, _) = MfaFlow::create(
        &mut tx,
        "OIDC Only".into(),
        vec![vec![VpnClientMfaMethod::Oidc]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // MFA is off, but the OIDC assignment survives, exactly as it does after an admin toggles MFA
    // off on a previously OIDC-protected location.
    let network = WireguardNetwork::default()
        .try_set_address("10.21.0.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    assert!(
        !network.mfa_enabled,
        "precondition: this location must have MFA disabled"
    );

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: oidc_flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let affected = WireguardNetwork::all_with_oidc_in_flows(&pool)
        .await
        .unwrap();
    let affected_ids: Vec<_> = affected.iter().map(|l| l.id).collect();
    assert!(
        affected_ids.contains(&network.id),
        "MFA-disabled location {id} with an OIDC flow must still be reported, got: {affected_ids:?}",
        id = network.id
    );
}

/// `all_with_oidc_in_flows` must return an empty set when no location's flows
/// contain OIDC, even when MFA is enabled on some locations.
#[sqlx::test]
async fn test_all_with_oidc_in_flows_empty_when_no_oidc(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(
        &mut tx,
        "Internal".into(),
        vec![vec![VpnClientMfaMethod::Totp]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.20.2.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let external = WireguardNetwork::all_with_oidc_in_flows(&pool)
        .await
        .unwrap();
    assert!(
        external.is_empty(),
        "should return empty when no location has OIDC in its flows, got {} locations",
        external.len()
    );
}

/// Query checking for internal MFA should return false when every MFA location
/// uses only OIDC flows.
#[sqlx::test]
async fn test_internal_mfa_query_false_for_oidc_only(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (oidc_flow, _) = MfaFlow::create(
        &mut tx,
        "OIDC Only".into(),
        vec![vec![VpnClientMfaMethod::Oidc]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.30.0.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: oidc_flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // The same flow-shape query the enrollment server uses: any MFA-enabled
    // location whose flows include an internal method.
    let has_internal = sqlx::query_scalar!(
        "SELECT EXISTS( \
            SELECT 1 FROM wireguard_network wn \
            JOIN location_mfa_flow lmf ON lmf.location_id = wn.id \
            JOIN mfa_flow_step mfs ON mfs.flow_id = lmf.flow_id \
            WHERE wn.mfa_enabled = true \
            AND mfs.methods && ARRAY['totp','email','biometric','mobileapprove']::vpn_client_mfa_method[] \
        ) \"exists!\""
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        !has_internal,
        "OIDC-only location must not trigger the internal MFA check"
    );
}

#[sqlx::test]
async fn test_derive_legacy_internal_with_fido2(_: PgPoolOptions, options: PgConnectOptions) {
    // FIDO2 alongside the internal set stays Internal: a client that cannot see
    // FIDO2 can still pass the step with any of the other factors, so adding it
    // must not lock legacy clients out of the location.
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(
        &mut tx,
        "Internal + FIDO2".into(),
        vec![vec![
            VpnClientMfaMethod::Totp,
            VpnClientMfaMethod::Email,
            VpnClientMfaMethod::Biometric,
            VpnClientMfaMethod::MobileApprove,
            VpnClientMfaMethod::Fido2,
        ]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.9.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mode = MfaFlow::derive_legacy_mode(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(mode, Some(LocationMfaMode::Internal));
}

#[sqlx::test]
async fn test_derive_legacy_fido2_only_omitted(_: PgPoolOptions, options: PgConnectOptions) {
    // A FIDO2-only step has no legacy equivalent: a client that cannot drive a
    // security key has no way to pass it, so fail closed rather than advertise
    // a mode it cannot satisfy.
    let pool = setup_pool(options).await;

    let mut tx = pool.begin().await.unwrap();
    let (flow, _) = MfaFlow::create(
        &mut tx,
        "FIDO2 only".into(),
        vec![vec![VpnClientMfaMethod::Fido2]],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut network = WireguardNetwork::default()
        .try_set_address("10.1.10.1/24")
        .unwrap()
        .save(&pool)
        .await
        .unwrap();
    network.mfa_enabled = true;
    network.save(&pool).await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    MfaFlow::assign_to_location(
        &mut tx,
        network.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mode = MfaFlow::derive_legacy_mode(&pool, network.id)
        .await
        .unwrap();
    assert_eq!(mode, None);
}
