use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::db::setup_pool;

/// Helper: create a flow with two steps and return its (flow, steps).
async fn create_flow(pool: &sqlx::PgPool) -> (MfaFlow<Id>, Vec<MfaFlowStep<Id>>) {
    let mut tx = pool.begin().await.unwrap();
    let (flow, steps) = MfaFlow::create(
        &mut *tx,
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

#[sqlx::test]
async fn test_insert_new_step(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (flow, original_steps) = create_flow(&pool).await;
    assert_eq!(original_steps.len(), 2);

    let mut tx = pool.begin().await.unwrap();
    let (_, updated_steps) = MfaFlow::update_with_steps(
        &mut *tx,
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
        &mut *tx,
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

    // Add a third step
    let mut tx = pool.begin().await.unwrap();
    MfaFlowStep::insert_batch(&mut *tx, flow.id, &[vec![VpnClientMfaMethod::Oidc]])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let all_steps = MfaFlowStep::find_by_flow(&pool, flow.id).await.unwrap();
    assert_eq!(all_steps.len(), 3);

    // Update: keep steps 0 and 2, delete step 1
    let mut tx = pool.begin().await.unwrap();
    let (_, updated_steps) = MfaFlow::update_with_steps(
        &mut *tx,
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
        &mut *tx,
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
    let errors = validate_flow_input("Test", &[vec![], vec![VpnClientMfaMethod::Totp]]);
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
