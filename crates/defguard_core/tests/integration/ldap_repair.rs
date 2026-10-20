use defguard_common::db::{models::User, setup_pool};
use defguard_core::enterprise::ldap::migration::repair;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    query, query_as, query_scalar,
};

async fn add_user(pool: &sqlx::PgPool, username: &str, ldap: bool, rdn: &str, path: &str) -> i64 {
    let mut user = User::new(
        username,
        Some("pass123"),
        "Doe",
        "Jane",
        &format!("{username}@example.org"),
        None,
    );
    user.from_ldap = ldap;
    user.ldap_rdn = Some(rdn.into());
    user.ldap_user_path = Some(path.into());
    user.save(pool).await.unwrap().id
}

#[sqlx::test]
async fn test_repair_migrates_both_legacy_dn_formats_and_is_idempotent(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let first = add_user(
        &pool,
        "repair_first",
        true,
        r"Doe\",
        " John,ou=users,dc=example,dc=org",
    )
    .await;
    let second = add_user(
        &pool,
        "repair_second",
        true,
        r"Smith\2C Jane",
        r"ou=People\,West,dc=example,dc=org",
    )
    .await;
    let plain = add_user(
        &pool,
        "repair_plain",
        true,
        "Plain",
        "ou=users,dc=example,dc=org",
    )
    .await;
    let native = add_user(
        &pool,
        "repair_native",
        false,
        r"Native\2C User",
        "ou=users,dc=example,dc=org",
    )
    .await;
    let count = repair(&pool).await.unwrap();
    assert_eq!((count.found, count.migrated), (2, 2));
    let completed: bool =
        query_scalar("SELECT ldap_migration_completed FROM wizard WHERE is_singleton")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(completed);
    for (id, expected_rdn, expected_path) in [
        (first, "Doe, John", "ou=users,dc=example,dc=org"),
        (second, "Smith, Jane", r"ou=People\,West,dc=example,dc=org"),
    ] {
        let user = User::find_by_username(
            &pool,
            if id == first {
                "repair_first"
            } else {
                "repair_second"
            },
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(user.id, id);
        assert_eq!(user.ldap_rdn.as_deref(), Some(expected_rdn));
        assert_eq!(user.ldap_user_path.as_deref(), Some(expected_path));
        assert_eq!(
            (&user.first_name[..], &user.last_name[..], &user.email[..]),
            (
                "Jane",
                "Doe",
                if id == first {
                    "repair_first@example.org"
                } else {
                    "repair_second@example.org"
                }
            )
        );
    }
    let plain_user = User::find_by_username(&pool, "repair_plain")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(plain_user.id, plain);
    assert_eq!(plain_user.ldap_rdn.as_deref(), Some("Plain"));
    assert_eq!(
        plain_user.ldap_user_path.as_deref(),
        Some("ou=users,dc=example,dc=org")
    );
    let native_user = User::find_by_username(&pool, "repair_native")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(native_user.id, native);
    assert_eq!(native_user.ldap_rdn.as_deref(), Some(r"Native\2C User"));
    assert_eq!(
        native_user.ldap_user_path.as_deref(),
        Some("ou=users,dc=example,dc=org")
    );
    add_user(
        &pool,
        "repair_after_completion",
        true,
        r"Late\2C User",
        "ou=users,dc=example,dc=org",
    )
    .await;
    let repeated = repair(&pool).await.unwrap();
    assert_eq!((repeated.found, repeated.migrated), (0, 0));
    let late_user = User::find_by_username(&pool, "repair_after_completion")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(late_user.ldap_rdn.as_deref(), Some(r"Late\2C User"));
}

#[sqlx::test]
async fn test_repair_rolls_back_updates_when_a_later_candidate_fails(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    add_user(
        &pool,
        "rollback_first",
        true,
        r"Doe\2C John",
        "ou=users,dc=example,dc=org",
    )
    .await;
    add_user(
        &pool,
        "rollback_second",
        true,
        r"Smith\2C Jane",
        "ou=users,dc=example,dc=org",
    )
    .await;
    query("CREATE SEQUENCE repair_update_sequence")
        .execute(&pool)
        .await
        .unwrap();
    query("CREATE FUNCTION repair_update_guard() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF nextval('repair_update_sequence') = 2 THEN RAISE EXCEPTION 'expected repair failure'; END IF; RETURN NEW; END; $$").execute(&pool).await.unwrap();
    query("CREATE TRIGGER repair_update_trigger BEFORE UPDATE OF ldap_rdn, ldap_user_path ON \"user\" FOR EACH ROW EXECUTE FUNCTION repair_update_guard()").execute(&pool).await.unwrap();
    let error = match repair(&pool).await {
        Ok(_) => panic!("repair should fail on its second update"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("expected repair failure"));
    let values: Vec<(String, String)> = query_as("SELECT ldap_rdn, ldap_user_path FROM \"user\" WHERE username IN ('rollback_first', 'rollback_second') ORDER BY username").fetch_all(&pool).await.unwrap();
    assert_eq!(
        values,
        vec![
            (r"Doe\2C John".into(), "ou=users,dc=example,dc=org".into()),
            (r"Smith\2C Jane".into(), "ou=users,dc=example,dc=org".into())
        ]
    );
    let sequence_value: i64 = query_scalar("SELECT last_value FROM repair_update_sequence")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sequence_value, 2);
}
