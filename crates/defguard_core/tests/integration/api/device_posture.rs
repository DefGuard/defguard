use defguard_common::db::setup_pool;
use defguard_core::{
    enterprise::{
        db::models::device_posture::{DevicePosture, OsType},
        handlers::device_posture::{
            ApiDevicePosture, ApiOsRule, CLIENT_VERSIONS, EditDevicePosture, valid_os_versions,
        },
        license::{get_cached_license, set_cached_license},
    },
    events::ApiEventType,
};
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::{
    PaginatedApiResponse,
    common::{ClientState, authenticate_admin, make_test_client, set_enterprise_license},
};

fn make_edit(name: &str) -> EditDevicePosture {
    EditDevicePosture {
        name: name.to_string(),
        description: Some(format!("{name} description")),
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![],
    }
}

use crate::api::common::client::TestClient;

/// Set up a test client with enterprise license and admin session ready.
/// All device posture tests that don't test license gating should use this.
async fn setup(options: PgConnectOptions) -> (TestClient, ClientState) {
    let pool = setup_pool(options).await;
    let (mut client, state) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    client.drain_all_events();
    set_enterprise_license();
    (client, state)
}

#[sqlx::test]
async fn test_device_posture_enterprise_license_required(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    client.drain_all_events();

    let edit = make_edit("test");
    let saved = get_cached_license().clone();

    // no license → 403
    set_cached_license(None);
    let response = client.get("/api/v1/device-posture").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    client.assert_event_queue_is_empty();

    // business-only license (default from make_test_client) → 403
    set_cached_license(saved.clone()); // restore Business tier
    let response = client.get("/api/v1/device-posture").send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client
        .post("/api/v1/device-posture")
        .json(&edit)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    client.assert_event_queue_is_empty();

    // enterprise license → accessible
    set_enterprise_license();
    let response = client.get("/api/v1/device-posture").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client
        .post("/api/v1/device-posture")
        .json(&edit)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    set_cached_license(saved);
}

#[sqlx::test]
async fn test_device_posture_crud(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    // list - initially empty
    let response = client.get("/api/v1/device-posture").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert!(page.data.is_empty());

    // create
    let edit = EditDevicePosture {
        name: "My Policy".to_string(),
        description: Some("desc".to_string()),
        min_client_version: Some(CLIENT_VERSIONS[0].to_string()),
        allow_prerelease_client: true,
        os_rules: vec![],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&edit)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: ApiDevicePosture = response.json().await;
    assert_eq!(created.name, "My Policy");
    assert_eq!(created.description.as_deref(), Some("desc"));
    assert_eq!(
        created.min_client_version.as_deref(),
        Some(CLIENT_VERSIONS[0])
    );
    assert!(created.allow_prerelease_client);
    assert!(created.locations.is_empty());
    let id = created.id;

    client.verify_api_events(&[ApiEventType::DevicePostureCreated {
        device_posture: DevicePosture {
            id,
            name: "My Policy".to_string(),
            description: Some("desc".to_string()),
            min_client_version: Some(CLIENT_VERSIONS[0].to_string()),
            allow_prerelease_client: true,
        },
    }]);

    // list - one result
    let response = client.get("/api/v1/device-posture").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, id);

    // get
    let response = client
        .get(format!("/api/v1/device-posture/{id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let fetched: ApiDevicePosture = response.json().await;
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.name, "My Policy");

    // update
    let update = EditDevicePosture {
        name: "Updated Policy".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![],
    };
    let response = client
        .put(format!("/api/v1/device-posture/{id}"))
        .json(&update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated: ApiDevicePosture = response.json().await;
    assert_eq!(updated.name, "Updated Policy");
    assert!(updated.description.is_none());
    assert!(updated.min_client_version.is_none());
    assert!(!updated.allow_prerelease_client);

    client.verify_api_events(&[ApiEventType::DevicePostureUpdated {
        before: DevicePosture {
            id,
            name: "My Policy".to_string(),
            description: Some("desc".to_string()),
            min_client_version: Some(CLIENT_VERSIONS[0].to_string()),
            allow_prerelease_client: true,
        },
        after: DevicePosture {
            id,
            name: "Updated Policy".to_string(),
            description: None,
            min_client_version: None,
            allow_prerelease_client: false,
        },
    }]);

    // delete
    let response = client
        .delete(format!("/api/v1/device-posture/{id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    client.verify_api_events(&[ApiEventType::DevicePostureDeleted {
        device_posture: DevicePosture {
            id,
            name: "Updated Policy".to_string(),
            description: None,
            min_client_version: None,
            allow_prerelease_client: false,
        },
    }]);

    // get after delete → 404
    let response = client
        .get(format!("/api/v1/device-posture/{id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_device_posture_duplicate(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    // create original
    let edit = EditDevicePosture {
        name: "Original".to_string(),
        description: Some("original desc".to_string()),
        min_client_version: Some(CLIENT_VERSIONS[0].to_string()),
        allow_prerelease_client: false,
        os_rules: vec![],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&edit)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let original: ApiDevicePosture = response.json().await;
    client.drain_all_events();

    // duplicate
    let response = client
        .post(format!("/api/v1/device-posture/{}/duplicate", original.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let copy: ApiDevicePosture = response.json().await;

    assert_ne!(copy.id, original.id);
    assert_eq!(copy.name, "Original (copy)");
    assert_eq!(copy.description, original.description);
    assert_eq!(copy.min_client_version, original.min_client_version);
    assert_eq!(
        copy.allow_prerelease_client,
        original.allow_prerelease_client
    );
    assert!(copy.locations.is_empty());

    client.verify_api_events(&[ApiEventType::DevicePostureDuplicated {
        original: DevicePosture {
            id: original.id,
            name: "Original".to_string(),
            description: Some("original desc".to_string()),
            min_client_version: Some(CLIENT_VERSIONS[0].to_string()),
            allow_prerelease_client: false,
        },
        duplicate: DevicePosture {
            id: copy.id,
            name: "Original (copy)".to_string(),
            description: Some("original desc".to_string()),
            min_client_version: Some(CLIENT_VERSIONS[0].to_string()),
            allow_prerelease_client: false,
        },
    }]);

    // list → 2 entries
    let response = client.get("/api/v1/device-posture").send().await;
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert_eq!(page.data.len(), 2);
}

#[sqlx::test]
async fn test_device_posture_duplicate_not_found(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    let response = client
        .post("/api/v1/device-posture/999/duplicate")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_device_posture_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    // unknown min_client_version → 400
    let bad = EditDevicePosture {
        name: "Bad".to_string(),
        description: None,
        min_client_version: Some("99.99".to_string()),
        allow_prerelease_client: false,
        os_rules: vec![],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&bad)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    client.assert_event_queue_is_empty();

    // create a valid one
    let edit = make_edit("Valid");
    let response = client
        .post("/api/v1/device-posture")
        .json(&edit)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: ApiDevicePosture = response.json().await;
    client.drain_all_events();

    // unknown version on update → 400
    let response = client
        .put(format!("/api/v1/device-posture/{}", created.id))
        .json(&bad)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_device_posture_pagination(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    // create 3 postures
    for i in 1..=3 {
        let response = client
            .post("/api/v1/device-posture")
            .json(&make_edit(&format!("Policy {i}")))
            .send()
            .await;
        assert_eq!(response.status(), StatusCode::CREATED);
    }
    client.drain_all_events();

    // page 1, per_page=2 → 2 results
    let response = client
        .get("/api/v1/device-posture?page=1&per_page=2")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert_eq!(page.data.len(), 2);

    // page 2, per_page=2 → 1 result
    let response = client
        .get("/api/v1/device-posture?page=2&per_page=2")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert_eq!(page.data.len(), 1);
}

#[sqlx::test]
async fn test_device_posture_os_versions(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    let response = client
        .get("/api/v1/device-posture/os-versions")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await;

    for os in ["windows", "macos", "linux", "ios", "android"] {
        let versions = body[os]
            .as_array()
            .unwrap_or_else(|| panic!("{os} key missing"));
        assert!(!versions.is_empty(), "{os} version list is empty");
    }

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_device_posture_client_versions(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    let response = client
        .get("/api/v1/device-posture/client-versions")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let versions: Vec<String> = response.json().await;
    assert!(!versions.is_empty());
    assert_eq!(versions, CLIENT_VERSIONS);

    client.assert_event_queue_is_empty();
}

#[sqlx::test]
async fn test_device_posture_os_rules_create_and_get(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    let windows_version = valid_os_versions(&OsType::Windows)[0];
    let macos_version = valid_os_versions(&OsType::Macos)[0];

    let edit = EditDevicePosture {
        name: "With Rules".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: Some(windows_version.to_string()),
                disk_encryption_required: Some(true),
                antivirus_required: Some(false),
                ad_domain_joined_required: None,
                windows_security_update_current: Some(true),
            },
            ApiOsRule::Macos {
                min_os_version: Some(macos_version.to_string()),
                disk_encryption_required: Some(true),
                device_integrity_required: Some(true),
            },
        ],
    };

    let response = client
        .post("/api/v1/device-posture")
        .json(&edit)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: ApiDevicePosture = response.json().await;
    assert_eq!(created.os_rules.len(), 2);
    client.drain_all_events();

    // GET returns the same rules
    let response = client
        .get(format!("/api/v1/device-posture/{}", created.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let fetched: ApiDevicePosture = response.json().await;
    assert_eq!(fetched.os_rules.len(), 2);
    assert!(
        fetched
            .os_rules
            .iter()
            .any(|r| matches!(r, ApiOsRule::Windows { .. }))
    );
    assert!(
        fetched
            .os_rules
            .iter()
            .any(|r| matches!(r, ApiOsRule::Macos { .. }))
    );

    // list also includes os_rules
    let response = client.get("/api/v1/device-posture").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert_eq!(page.data[0].os_rules.len(), 2);
}

#[sqlx::test]
async fn test_device_posture_os_rules_update_replaces(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    let linux_version = valid_os_versions(&OsType::Linux)[0];

    // create with windows + macos rules
    let create = EditDevicePosture {
        name: "Replace Test".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: None,
                disk_encryption_required: Some(true),
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_current: None,
            },
            ApiOsRule::Macos {
                min_os_version: None,
                disk_encryption_required: None,
                device_integrity_required: None,
            },
        ],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&create)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: ApiDevicePosture = response.json().await;
    assert_eq!(created.os_rules.len(), 2);
    client.drain_all_events();

    // update with only linux rule
    let update = EditDevicePosture {
        name: "Replace Test".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![ApiOsRule::Linux {
            min_os_version: Some(linux_version.to_string()),
            min_kernel_version: None,
            disk_encryption_required: Some(true),
        }],
    };
    let response = client
        .put(format!("/api/v1/device-posture/{}", created.id))
        .json(&update)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated: ApiDevicePosture = response.json().await;
    assert_eq!(updated.os_rules.len(), 1);
    assert!(matches!(updated.os_rules[0], ApiOsRule::Linux { .. }));
    client.drain_all_events();

    // GET confirms windows + macos rules are gone
    let response = client
        .get(format!("/api/v1/device-posture/{}", created.id))
        .send()
        .await;
    let fetched: ApiDevicePosture = response.json().await;
    assert_eq!(fetched.os_rules.len(), 1);
    assert!(matches!(fetched.os_rules[0], ApiOsRule::Linux { .. }));
}

#[sqlx::test]
async fn test_device_posture_os_rules_duplicate_copies(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, _) = setup(options).await;

    let create = EditDevicePosture {
        name: "Original".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: None,
                disk_encryption_required: Some(true),
                antivirus_required: Some(true),
                ad_domain_joined_required: None,
                windows_security_update_current: None,
            },
            ApiOsRule::Android {
                min_os_version: None,
                device_integrity_required: Some(true),
            },
        ],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&create)
        .send()
        .await;
    let original: ApiDevicePosture = response.json().await;
    client.drain_all_events();

    // duplicate
    let response = client
        .post(format!("/api/v1/device-posture/{}/duplicate", original.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let copy: ApiDevicePosture = response.json().await;

    assert_ne!(copy.id, original.id);
    assert_eq!(copy.os_rules.len(), 2);
    assert!(
        copy.os_rules
            .iter()
            .any(|r| matches!(r, ApiOsRule::Windows { .. }))
    );
    assert!(
        copy.os_rules
            .iter()
            .any(|r| matches!(r, ApiOsRule::Android { .. }))
    );
    client.drain_all_events();

    // GET on copy confirms rules are persisted
    let response = client
        .get(format!("/api/v1/device-posture/{}", copy.id))
        .send()
        .await;
    let fetched: ApiDevicePosture = response.json().await;
    assert_eq!(fetched.os_rules.len(), 2);
}

#[sqlx::test]
async fn test_device_posture_os_rules_validation(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    // unknown min_os_version for windows → 400
    let bad_version = EditDevicePosture {
        name: "Bad".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![ApiOsRule::Windows {
            min_os_version: Some("Windows 7".to_string()),
            disk_encryption_required: None,
            antivirus_required: None,
            ad_domain_joined_required: None,
            windows_security_update_current: None,
        }],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&bad_version)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    client.assert_event_queue_is_empty();

    // duplicate os_type → 400
    let duplicate_os = EditDevicePosture {
        name: "Dup".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: None,
                disk_encryption_required: None,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_current: None,
            },
            ApiOsRule::Windows {
                min_os_version: None,
                disk_encryption_required: Some(true),
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_current: None,
            },
        ],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&duplicate_os)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    client.assert_event_queue_is_empty();

    // unknown min_kernel_version for linux → 400
    let bad_kernel = EditDevicePosture {
        name: "Bad Kernel".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![ApiOsRule::Linux {
            min_os_version: None,
            min_kernel_version: Some("4.x".to_string()),
            disk_encryption_required: None,
        }],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&bad_kernel)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    client.assert_event_queue_is_empty();
}
