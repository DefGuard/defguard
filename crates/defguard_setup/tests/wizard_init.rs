use defguard_common::db::{
    models::{
        User,
        settings::initialize_current_settings,
        wizard::{ActiveWizard, Wizard},
    },
    setup_pool,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[sqlx::test]
async fn test_wizard_init_fresh_db(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    // Fresh DB + no auto-adopt flags: Initial wizard
    let wizard = Wizard::init(&pool, false)
        .await
        .expect("Failed to init wizard");

    assert_eq!(wizard.active_wizard, ActiveWizard::Initial);
    assert!(!wizard.completed);
    assert!(wizard.is_active());

    // requires_auth returns false at the Welcome step (no admin created yet)
    let requires_auth = wizard
        .requires_auth(&pool)
        .await
        .expect("Failed to check requires_auth");
    assert!(
        !requires_auth,
        "Initial wizard at Welcome step should not require auth yet"
    );

    let wizard_from_db = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert_eq!(wizard_from_db.active_wizard, ActiveWizard::Initial);
    assert!(!wizard_from_db.completed);
}

#[sqlx::test]
async fn test_wizard_init_auto_adopt_flags(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    // Fresh DB + auto-adopt flags: AutoAdoption wizard
    let wizard = Wizard::init(&pool, true)
        .await
        .expect("Failed to init wizard");

    assert_eq!(wizard.active_wizard, ActiveWizard::AutoAdoption);
    assert!(!wizard.completed);
    assert!(wizard.is_active());

    // requires_auth returns false at the Welcome step
    let requires_auth = wizard
        .requires_auth(&pool)
        .await
        .expect("Failed to check requires_auth");
    assert!(
        !requires_auth,
        "AutoAdoption wizard at Welcome step should not require auth yet"
    );

    let wizard_from_db = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert_eq!(wizard_from_db.active_wizard, ActiveWizard::AutoAdoption);
    assert!(!wizard_from_db.completed);
}

#[sqlx::test]
async fn test_wizard_init_existing_data(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    User::new(
        "existing_user",
        Some("Passw0rd!"),
        "Existing",
        "User",
        "existing@example.com",
        None,
    )
    .save(&pool)
    .await
    .expect("Failed to save user");

    // Existing data + no auto-adopt flags: Migration wizard
    let wizard = Wizard::init(&pool, false)
        .await
        .expect("Failed to init wizard");

    assert_eq!(wizard.active_wizard, ActiveWizard::Migration);
    assert!(!wizard.completed);
    assert!(wizard.is_active());

    // Migration wizard always requires auth (admin must log in)
    let requires_auth = wizard
        .requires_auth(&pool)
        .await
        .expect("Failed to check requires_auth");
    assert!(requires_auth, "Migration wizard should always require auth");

    let wizard_from_db = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert_eq!(wizard_from_db.active_wizard, ActiveWizard::Migration);
}

#[sqlx::test]
async fn test_wizard_init_idempotent(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("Failed to initialize settings");

    let first = Wizard::init(&pool, false)
        .await
        .expect("Failed to first init");
    assert_eq!(first.active_wizard, ActiveWizard::Initial);

    let second = Wizard::init(&pool, false)
        .await
        .expect("Failed to second init");
    assert_eq!(
        second.active_wizard,
        ActiveWizard::Initial,
        "Second init should resume the already-active wizard"
    );

    let third = Wizard::init(&pool, true)
        .await
        .expect("Failed to third init");
    assert_eq!(
        third.active_wizard,
        ActiveWizard::Initial,
        "Already-active wizard should not be switched by flags"
    );

    // Simulate completion: mark wizard as done
    let mut wizard = Wizard::get(&pool).await.expect("Failed to get wizard");
    wizard.completed = true;
    wizard.active_wizard = ActiveWizard::None;
    wizard.save(&pool).await.expect("Failed to save wizard");

    // Init after completion: completed flag is respected, nothing changes
    let after_completion = Wizard::init(&pool, false)
        .await
        .expect("Failed to init after completion");
    assert!(
        after_completion.completed,
        "Completed wizard should stay completed"
    );
    assert_eq!(
        after_completion.active_wizard,
        ActiveWizard::None,
        "Active wizard should remain None after completion"
    );
    assert!(!after_completion.is_active());

    let wizard_from_db = Wizard::get(&pool).await.expect("Failed to get wizard");
    assert!(wizard_from_db.completed);
    assert_eq!(wizard_from_db.active_wizard, ActiveWizard::None);
}
