use defguard_common::db::models::{
    Settings,
    settings::{SettingsPatch, update_current_settings},
};
use defguard_core::handlers::Auth;
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{make_test_client, setup_pool};

#[sqlx::test]
async fn test_settings(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;

    let (client, _client_state) = make_test_client(pool).await;
    let auth = Auth::new("admin", "pass123");
    let response = &client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    // get settings
    let response = client.get("/api/v1/settings").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let mut settings: Settings = response.json().await;
    // modify settings
    settings.wireguard_enabled = false;
    settings.challenge_template = "Modified".to_owned();
    let response = client.put("/api/v1/settings").json(&settings).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    // verify modified settings
    let response = client.get("/api/v1/settings").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let new_settings: Settings = response.json().await;
    assert_eq!(new_settings, settings);
    // patch settings
    let patch_json: &str = r#"
    {
        "wireguard_enabled": true
    }"#;
    let settings_patch: SettingsPatch = serde_json::from_str(patch_json).unwrap();
    let response = client
        .patch("/api/v1/settings")
        .json(&settings_patch)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client.get("/api/v1/settings").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let new_settings: Settings = response.json().await;
    assert!(new_settings.wireguard_enabled);
}

#[sqlx::test]
async fn test_patch_settings_clears_optional_fields(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _client_state) = make_test_client(pool.clone()).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // --- smtp_user & smtp_password ---

    // set smtp_user and smtp_password (include the required trio so validation passes)
    let patch: SettingsPatch = serde_json::from_str(
        r#"{
            "smtp_server": "smtp.example.com",
            "smtp_port": 587,
            "smtp_sender": "noreply@example.com",
            "smtp_user": "testuser",
            "smtp_password": "testpass"
        }"#,
    )
    .unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify fields are set
    let response = client.get("/api/v1/settings").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let settings: Settings = response.json().await;
    assert_eq!(
        settings.smtp_user,
        Some("testuser".to_owned()),
        "smtp_user should be set after initial PATCH"
    );
    // smtp_password is redacted in the API response; verify via DB
    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert!(
        from_db.smtp_password.is_some(),
        "smtp_password should be set in DB after initial PATCH"
    );

    // clear smtp_user and smtp_password by sending null
    let patch: SettingsPatch =
        serde_json::from_str(r#"{ "smtp_user": null, "smtp_password": null }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // assert both fields are cleared in the DB
    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert!(
        from_db.smtp_user.is_none(),
        "smtp_user should be cleared to None after PATCH with null"
    );
    assert!(
        from_db.smtp_password.is_none(),
        "smtp_password should be cleared to None after PATCH with null"
    );

    // --- ldap_user_rdn_attr ---

    // set ldap_user_rdn_attr
    let patch: SettingsPatch = serde_json::from_str(r#"{ "ldap_user_rdn_attr": "uid" }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // verify field is set
    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert_eq!(
        from_db.ldap_user_rdn_attr,
        Some("uid".to_owned()),
        "ldap_user_rdn_attr should be set after PATCH"
    );

    // clear ldap_user_rdn_attr by sending null
    let patch: SettingsPatch = serde_json::from_str(r#"{ "ldap_user_rdn_attr": null }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // assert field is cleared in the DB
    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert!(
        from_db.ldap_user_rdn_attr.is_none(),
        "ldap_user_rdn_attr should be cleared to None after PATCH with null"
    );
}

// JSON fragment containing all required LDAP fields except ldap_url (add that at the call site).
const VALID_LDAP_FIELDS_NO_URL: &str = r#"
    "ldap_bind_username": "cn=admin,dc=example,dc=com",
    "ldap_bind_password": "secret",
    "ldap_username_attr": "uid",
    "ldap_user_search_base": "ou=users,dc=example,dc=com",
    "ldap_user_obj_class": "inetOrgPerson",
    "ldap_member_attr": "memberUid",
    "ldap_groupname_attr": "cn",
    "ldap_group_obj_class": "posixGroup",
    "ldap_group_member_attr": "memberUid",
    "ldap_group_search_base": "ou=groups,dc=example,dc=com"
"#;

const VALID_LDAP_URL: &str = r#""ldap_url": "ldap://localhost""#;

#[sqlx::test]
async fn test_ldap_settings_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, _client_state) = make_test_client(pool).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enabling LDAP without any fields configured must fail
    let patch: SettingsPatch = serde_json::from_str(r#"{ "ldap_enabled": true }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "enabling LDAP without configured fields should return 400"
    );

    // enabling LDAP with an invalid URL must fail even when all other fields are present
    let patch: SettingsPatch = serde_json::from_str(&format!(
        r#"{{ {VALID_LDAP_FIELDS_NO_URL}, "ldap_url": "not-a-url", "ldap_enabled": true }}"#
    ))
    .unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "enabling LDAP with an invalid URL should return 400"
    );

    // enabling LDAP with all required fields filled and a valid URL must succeed
    let patch: SettingsPatch = serde_json::from_str(&format!(
        r#"{{ {VALID_LDAP_FIELDS_NO_URL}, {VALID_LDAP_URL}, "ldap_enabled": true }}"#
    ))
    .unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "enabling LDAP with all required fields should return 200"
    );
}

#[sqlx::test]
async fn test_ldap_remote_enrollment_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (client, client_state) = make_test_client(pool.clone()).await;

    let auth = Auth::new("admin", "pass123");
    let response = client.post("/api/v1/auth").json(&auth).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enabling remote enrollment without LDAP configured must fail
    let patch: SettingsPatch =
        serde_json::from_str(r#"{ "ldap_remote_enrollment_enabled": true }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "enabling remote enrollment without LDAP configured should return 400"
    );

    // configure LDAP fields (without SMTP)
    let patch: SettingsPatch = serde_json::from_str(&format!(
        r#"{{ {VALID_LDAP_FIELDS_NO_URL}, {VALID_LDAP_URL} }}"#
    ))
    .unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    // enabling remote enrollment with LDAP configured but no SMTP must still fail
    let patch: SettingsPatch =
        serde_json::from_str(r#"{ "ldap_remote_enrollment_enabled": true }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "enabling remote enrollment without SMTP configured should return 400"
    );

    // configure SMTP via direct DB mutation (same pattern used for test setup in auth tests)
    let mut settings = Settings::get_current_settings();
    settings.smtp_server = Some("smtp.example.com".into());
    settings.smtp_port = Some(587);
    settings.smtp_sender = Some("noreply@example.com".into());
    update_current_settings(&client_state.pool, settings)
        .await
        .unwrap();

    // enabling remote enrollment with both LDAP and SMTP configured must succeed
    let patch: SettingsPatch =
        serde_json::from_str(r#"{ "ldap_remote_enrollment_enabled": true }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "enabling remote enrollment with LDAP and SMTP configured should return 200"
    );
    // verify the flag was actually persisted to the database (not just held in memory)
    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert!(
        from_db.ldap_remote_enrollment_enabled,
        "ldap_remote_enrollment_enabled must be persisted to DB after enabling"
    );

    // enabling send_invite while remote enrollment is disabled must fail
    // (use a fresh settings state: disable enrollment first)
    let patch: SettingsPatch =
        serde_json::from_str(r#"{ "ldap_remote_enrollment_enabled": false }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert!(
        !from_db.ldap_remote_enrollment_enabled,
        "disabling ldap_remote_enrollment_enabled must be persisted to DB"
    );

    let patch: SettingsPatch =
        serde_json::from_str(r#"{ "ldap_remote_enrollment_send_invite": true }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "enabling send_invite without remote enrollment enabled should return 400"
    );

    // re-enable remote enrollment, then enabling send_invite must succeed
    let patch: SettingsPatch =
        serde_json::from_str(r#"{ "ldap_remote_enrollment_enabled": true }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(response.status(), StatusCode::OK);

    let patch: SettingsPatch =
        serde_json::from_str(r#"{ "ldap_remote_enrollment_send_invite": true }"#).unwrap();
    let response = client.patch("/api/v1/settings").json(&patch).send().await;
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "enabling send_invite with remote enrollment enabled should return 200"
    );
    // verify both flags were persisted to the database
    let from_db = Settings::get(&pool).await.unwrap().unwrap();
    assert!(
        from_db.ldap_remote_enrollment_enabled,
        "ldap_remote_enrollment_enabled must still be true in DB"
    );
    assert!(
        from_db.ldap_remote_enrollment_send_invite,
        "ldap_remote_enrollment_send_invite must be persisted to DB after enabling"
    );
}
