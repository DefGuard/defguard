use defguard_common::db::{Id, setup_pool};
use defguard_core::{
    enterprise::{
        db::models::device_posture::{DevicePosture, DevicePostureSnapshot},
        handlers::device_posture::{
            ApiDevicePosture, ApiOsRule, AssignLocationsData, AssignPosturesData,
            DevicePostureVersionMetadata, EditDevicePosture,
        },
        license::{get_cached_license, set_cached_license},
        posture::version_list::{
            ANDROID_OS_VERSIONS, DESKTOP_CLIENT_VERSIONS, IOS_OS_VERSIONS, LINUX_KERNEL_VERSIONS,
            MACOS_OS_VERSIONS, MOBILE_CLIENT_VERSIONS, WINDOWS_OS_VERSIONS,
        },
    },
    events::ApiEventType,
    grpc::GatewayCommand,
};
use reqwest::StatusCode;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::{
    PaginatedApiResponse,
    common::{ClientState, authenticate_admin, make_test_client, set_enterprise_license},
};

fn make_edit(name: &str) -> EditDevicePosture {
    EditDevicePosture {
        name: name.to_owned(),
        description: Some(format!("{name} description")),
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: Vec::new(),
    }
}

use crate::api::common::{client::TestClient, make_network};

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

fn drain_gateway_events(gateway_rx: &mut tokio::sync::broadcast::Receiver<GatewayCommand>) {
    while gateway_rx.try_recv().is_ok() {}
}

fn expect_network_modified_peers(
    gateway_rx: &mut tokio::sync::broadcast::Receiver<GatewayCommand>,
    location_id: Id,
    expected_pubkeys: &[&str],
) {
    let event = gateway_rx.try_recv().unwrap();
    let GatewayCommand::NetworkModified(id, _, peers, _) = event else {
        panic!("expected NetworkModified, got {event:?}");
    };
    assert_eq!(id, location_id);
    let actual_pubkeys = peers
        .iter()
        .map(|peer| peer.pubkey.as_str())
        .collect::<Vec<_>>();
    assert_eq!(actual_pubkeys, expected_pubkeys);
}

async fn create_posture(client: &TestClient, name: &str) -> ApiDevicePosture {
    let response = client
        .post("/api/v1/device-posture")
        .json(&make_edit(name))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    response.json().await
}

async fn create_admin_device(client: &TestClient) -> &'static str {
    const PUBKEY: &str = "LQKsT6/3HWKuJmMulH63R8iK+5sI8FyYEL6WDIi6lQU=";
    let response = client
        .post("/api/v1/device/admin")
        .json(&serde_json::json!({
            "name": "posture-test-device",
            "wireguard_pubkey": PUBKEY,
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    PUBKEY
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

    // no license → GET is accessible, POST is forbidden
    set_cached_license(None);
    let response = client.get("/api/v1/device-posture").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let response = client
        .post("/api/v1/device-posture")
        .json(&edit)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    client.assert_event_queue_is_empty();

    // business-only license (default from make_test_client) → GET is accessible, POST is forbidden
    set_cached_license(saved.clone()); // restore Business tier
    let response = client.get("/api/v1/device-posture").send().await;
    assert_eq!(response.status(), StatusCode::OK);
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
async fn test_device_posture_versions_metadata(_: PgPoolOptions, options: PgConnectOptions) {
    let (client, _) = setup(options).await;

    let response = client.get("/api/v1/device-posture/versions").send().await;
    assert_eq!(response.status(), StatusCode::OK);
    let metadata: DevicePostureVersionMetadata = response.json().await;

    assert_eq!(metadata.os_versions.windows, WINDOWS_OS_VERSIONS.to_vec());
    assert_eq!(metadata.os_versions.macos, MACOS_OS_VERSIONS.to_vec());
    assert_eq!(
        metadata.linux_kernel_versions,
        LINUX_KERNEL_VERSIONS.to_vec()
    );
    assert_eq!(metadata.os_versions.ios, IOS_OS_VERSIONS.to_vec());
    assert_eq!(metadata.os_versions.android, ANDROID_OS_VERSIONS.to_vec());
    assert_eq!(
        metadata.desktop_client_versions,
        DESKTOP_CLIENT_VERSIONS
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        metadata.mobile_client_versions,
        MOBILE_CLIENT_VERSIONS
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
    );
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
        name: "My Policy".to_owned(),
        description: Some("desc".to_owned()),
        min_desktop_client_version: Some(DESKTOP_CLIENT_VERSIONS[0].to_owned()),
        min_mobile_client_version: None,
        allow_prerelease_client: true,
        os_rules: Vec::new(),
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
        created.min_desktop_client_version.as_deref(),
        Some(DESKTOP_CLIENT_VERSIONS[0])
    );
    assert!(created.min_mobile_client_version.is_none());
    assert!(created.allow_prerelease_client);
    assert!(created.locations.is_empty());
    let id = created.id;

    client.verify_api_events(&[ApiEventType::DevicePostureCreated {
        snapshot: DevicePostureSnapshot {
            device_posture: DevicePosture {
                id,
                name: "My Policy".to_owned(),
                description: Some("desc".to_owned()),
                min_desktop_client_version: Some(DESKTOP_CLIENT_VERSIONS[0].to_owned()),
                min_mobile_client_version: None,
                allow_prerelease_client: true,
            },
            os_rules: Vec::new(),
            location_ids: Vec::new(),
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
        name: "Updated Policy".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: Vec::new(),
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
    assert!(updated.min_desktop_client_version.is_none());
    assert!(updated.min_mobile_client_version.is_none());
    assert!(!updated.allow_prerelease_client);

    client.verify_api_events(&[ApiEventType::DevicePostureUpdated {
        before: DevicePostureSnapshot {
            device_posture: DevicePosture {
                id,
                name: "My Policy".to_owned(),
                description: Some("desc".to_owned()),
                min_desktop_client_version: Some(DESKTOP_CLIENT_VERSIONS[0].to_owned()),
                min_mobile_client_version: None,
                allow_prerelease_client: true,
            },
            os_rules: Vec::new(),
            location_ids: Vec::new(),
        },
        after: DevicePostureSnapshot {
            device_posture: DevicePosture {
                id,
                name: "Updated Policy".to_owned(),
                description: None,
                min_desktop_client_version: None,
                min_mobile_client_version: None,
                allow_prerelease_client: false,
            },
            os_rules: Vec::new(),
            location_ids: Vec::new(),
        },
    }]);

    // delete
    let response = client
        .delete(format!("/api/v1/device-posture/{id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    client.verify_api_events(&[ApiEventType::DevicePostureDeleted {
        snapshot: DevicePostureSnapshot {
            device_posture: DevicePosture {
                id,
                name: "Updated Policy".to_owned(),
                description: None,
                min_desktop_client_version: None,
                min_mobile_client_version: None,
                allow_prerelease_client: false,
            },
            os_rules: Vec::new(),
            location_ids: Vec::new(),
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
        name: "Original".to_owned(),
        description: Some("original desc".to_owned()),
        min_desktop_client_version: Some(DESKTOP_CLIENT_VERSIONS[0].to_owned()),
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: Vec::new(),
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
    assert_eq!(copy.name, "Copy of Original");
    assert_eq!(copy.description, original.description);
    assert_eq!(
        copy.min_desktop_client_version,
        original.min_desktop_client_version
    );
    assert_eq!(
        copy.min_mobile_client_version,
        original.min_mobile_client_version
    );
    assert_eq!(
        copy.allow_prerelease_client,
        original.allow_prerelease_client
    );
    assert!(copy.locations.is_empty());

    client.verify_api_events(&[ApiEventType::DevicePostureDuplicated {
        original: DevicePostureSnapshot {
            device_posture: DevicePosture {
                id: original.id,
                name: "Original".to_owned(),
                description: Some("original desc".to_owned()),
                min_desktop_client_version: Some(DESKTOP_CLIENT_VERSIONS[0].to_owned()),
                min_mobile_client_version: None,
                allow_prerelease_client: false,
            },
            os_rules: Vec::new(),
            location_ids: Vec::new(),
        },
        duplicate: DevicePostureSnapshot {
            device_posture: DevicePosture {
                id: copy.id,
                name: "Copy of Original".to_owned(),
                description: Some("original desc".to_owned()),
                min_desktop_client_version: Some(DESKTOP_CLIENT_VERSIONS[0].to_owned()),
                min_mobile_client_version: None,
                allow_prerelease_client: false,
            },
            os_rules: Vec::new(),
            location_ids: Vec::new(),
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

    // unknown min_desktop_client_version -> 400
    let bad = EditDevicePosture {
        name: "Bad".to_owned(),
        description: None,
        min_desktop_client_version: Some("99.99".to_owned()),
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: Vec::new(),
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
async fn test_device_posture_list_filters_os_and_defguard(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, _) = setup(options).await;

    let windows_version = WINDOWS_OS_VERSIONS[0];
    let android_version = ANDROID_OS_VERSIONS[2];

    let filtered = EditDevicePosture {
        name: "Filtered posture".to_owned(),
        description: None,
        min_desktop_client_version: Some(DESKTOP_CLIENT_VERSIONS[0].to_owned()),
        min_mobile_client_version: Some(MOBILE_CLIENT_VERSIONS[0].to_owned()),
        allow_prerelease_client: true,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: Some(windows_version),
                disk_encryption_required: Some(true),
                antivirus_required: Some(true),
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
            },
            ApiOsRule::Android {
                min_os_version: Some(android_version),
                device_integrity_required: Some(true),
                android_security_patch_level_max_age: None,
            },
        ],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&filtered)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);

    let other = EditDevicePosture {
        name: "Other posture".to_owned(),
        description: None,
        min_desktop_client_version: Some(DESKTOP_CLIENT_VERSIONS[0].to_owned()),
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: Some(windows_version),
                disk_encryption_required: Some(true),
                antivirus_required: Some(true),
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
            },
            ApiOsRule::Android {
                min_os_version: Some(android_version),
                device_integrity_required: Some(true),
                android_security_patch_level_max_age: None,
            },
        ],
    };
    let response = client
        .post("/api/v1/device-posture")
        .json(&other)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    client.drain_all_events();

    let response = client
        .get("/api/v1/device-posture?defguard=Pre-release%20allowed")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].name, "Filtered posture");

    let response = client
        .get(
            "/api/v1/device-posture?windows=10&windows=Disk%20encryption&windows=Antivirus&android=15&android=Device%20integrity&defguard_desktop=2.1&defguard=Pre-release%20allowed",
        )
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].name, "Filtered posture");

    let response = client
        .get("/api/v1/device-posture?defguard_mobile=1.7.0")
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let page: PaginatedApiResponse<ApiDevicePosture> = response.json().await;
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].name, "Filtered posture");
}

#[sqlx::test]
async fn test_device_posture_os_rules_create_and_get(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    let windows_version = WINDOWS_OS_VERSIONS[0];
    let macos_version = MACOS_OS_VERSIONS[0];

    let edit = EditDevicePosture {
        name: "With Rules".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: Some(windows_version),
                disk_encryption_required: Some(true),
                antivirus_required: Some(false),
                ad_domain_joined_required: None,
                windows_security_update_max_age: Some(30),
            },
            ApiOsRule::Macos {
                min_os_version: Some(macos_version),
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

    // create with windows + macos rules
    let create = EditDevicePosture {
        name: "Replace Test".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: None,
                disk_encryption_required: Some(true),
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
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
        name: "Replace Test".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![ApiOsRule::Linux {
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
        name: "Original".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: None,
                disk_encryption_required: Some(true),
                antivirus_required: Some(true),
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
            },
            ApiOsRule::Android {
                min_os_version: None,
                device_integrity_required: Some(true),
                android_security_patch_level_max_age: Some(30),
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
        name: "Bad".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![ApiOsRule::Windows {
            min_os_version: Some(7),
            disk_encryption_required: None,
            antivirus_required: None,
            ad_domain_joined_required: None,
            windows_security_update_max_age: None,
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
        name: "Dup".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![
            ApiOsRule::Windows {
                min_os_version: None,
                disk_encryption_required: None,
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
            },
            ApiOsRule::Windows {
                min_os_version: None,
                disk_encryption_required: Some(true),
                antivirus_required: None,
                ad_domain_joined_required: None,
                windows_security_update_max_age: None,
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
        name: "Bad Kernel".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: false,
        os_rules: vec![ApiOsRule::Linux {
            min_kernel_version: Some(4),
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

#[sqlx::test]
async fn test_device_posture_set_locations_for_posture(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, _) = setup(options).await;

    // create two locations and one posture
    let net1: serde_json::Value = make_network(&client, "net1").await.json().await;
    let net2: serde_json::Value = make_network(&client, "net2").await.json().await;
    let loc1 = net1["id"].as_i64().unwrap();
    let loc2 = net2["id"].as_i64().unwrap();

    let response = client
        .post("/api/v1/device-posture")
        .json(&make_edit("Posture"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let posture: ApiDevicePosture = response.json().await;
    client.drain_all_events();

    // assign two locations
    let response = client
        .put(format!("/api/v1/device-posture/{}/locations", posture.id))
        .json(&AssignLocationsData {
            locations: vec![loc1, loc2],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result: Vec<i64> = response.json().await;
    assert_eq!(result.len(), 2);
    assert!(result.contains(&loc1));
    assert!(result.contains(&loc2));

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].0,
        ApiEventType::DevicePostureLocationsAssigned { .. }
    ));

    // GET shows both locations
    let response = client
        .get(format!("/api/v1/device-posture/{}", posture.id))
        .send()
        .await;
    let fetched: ApiDevicePosture = response.json().await;
    assert_eq!(fetched.locations.len(), 2);
    assert!(fetched.locations.contains(&loc1));
    assert!(fetched.locations.contains(&loc2));

    // reassign with only one location — replace semantics
    let response = client
        .put(format!("/api/v1/device-posture/{}/locations", posture.id))
        .json(&AssignLocationsData {
            locations: vec![loc1],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result: Vec<i64> = response.json().await;
    assert_eq!(result, vec![loc1]);
    client.drain_all_events();

    let response = client
        .get(format!("/api/v1/device-posture/{}", posture.id))
        .send()
        .await;
    let fetched: ApiDevicePosture = response.json().await;
    assert_eq!(fetched.locations, vec![loc1]);
}

#[sqlx::test]
async fn test_device_posture_set_postures_for_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, _) = setup(options).await;

    // create one location and two postures
    let net: serde_json::Value = make_network(&client, "net").await.json().await;
    let location_id = net["id"].as_i64().unwrap();

    let p1: ApiDevicePosture = client
        .post("/api/v1/device-posture")
        .json(&make_edit("Posture 1"))
        .send()
        .await
        .json()
        .await;
    let p2: ApiDevicePosture = client
        .post("/api/v1/device-posture")
        .json(&make_edit("Posture 2"))
        .send()
        .await
        .json()
        .await;
    client.drain_all_events();

    // assign both postures to the location
    let response = client
        .put(format!("/api/v1/network/{location_id}/postures"))
        .json(&AssignPosturesData {
            postures: vec![p1.id, p2.id],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result: Vec<i64> = response.json().await;
    assert_eq!(result.len(), 2);
    assert!(result.contains(&p1.id));
    assert!(result.contains(&p2.id));

    let events = client.drain_all_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0].0,
        ApiEventType::LocationPosturesAssigned { .. }
    ));

    // GET on each posture shows the location
    for posture in [&p1, &p2] {
        let response = client
            .get(format!("/api/v1/device-posture/{}", posture.id))
            .send()
            .await;
        let fetched: ApiDevicePosture = response.json().await;
        assert!(fetched.locations.contains(&location_id));
    }

    // GET network shows both posture IDs
    let response = client
        .get(format!("/api/v1/network/{location_id}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let network: serde_json::Value = response.json().await;
    let posture_checks: Vec<i64> =
        serde_json::from_value(network["posture_checks"].clone()).unwrap();
    assert_eq!(posture_checks.len(), 2);

    // reassign with empty list — all postures removed
    let response = client
        .put(format!("/api/v1/network/{location_id}/postures"))
        .json(&AssignPosturesData {
            postures: Vec::new(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result: Vec<i64> = response.json().await;
    assert!(result.is_empty());
    client.drain_all_events();

    // GET on postures now shows no locations
    for posture in [&p1, &p2] {
        let response = client
            .get(format!("/api/v1/device-posture/{}", posture.id))
            .send()
            .await;
        let fetched: ApiDevicePosture = response.json().await;
        assert!(fetched.locations.is_empty());
    }
}

#[sqlx::test]
async fn test_assigning_first_posture_refreshes_gateway_with_no_direct_peers(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, state) = setup(options).await;
    let mut gateway_rx = state.gateway_rx;

    let net: serde_json::Value = make_network(&client, "net").await.json().await;
    let location_id = net["id"].as_i64().unwrap();
    let _device_pubkey = create_admin_device(&client).await;
    let posture = create_posture(&client, "Posture").await;
    drain_gateway_events(&mut gateway_rx);
    client.drain_all_events();

    let response = client
        .put(format!("/api/v1/network/{location_id}/postures"))
        .json(&AssignPosturesData {
            postures: vec![posture.id],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    expect_network_modified_peers(&mut gateway_rx, location_id, &[]);
    let events = client.drain_all_events();
    assert!(matches!(
        events[0].0,
        ApiEventType::LocationPosturesAssigned { .. }
    ));
}

#[sqlx::test]
async fn test_removing_last_posture_refreshes_gateway_with_direct_peers(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, state) = setup(options).await;
    let mut gateway_rx = state.gateway_rx;

    let net: serde_json::Value = make_network(&client, "net").await.json().await;
    let location_id = net["id"].as_i64().unwrap();
    let device_pubkey = create_admin_device(&client).await;
    let posture = create_posture(&client, "Posture").await;
    drain_gateway_events(&mut gateway_rx);
    client.drain_all_events();

    let response = client
        .put(format!("/api/v1/network/{location_id}/postures"))
        .json(&AssignPosturesData {
            postures: vec![posture.id],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    expect_network_modified_peers(&mut gateway_rx, location_id, &[]);
    client.drain_all_events();

    let response = client
        .put(format!("/api/v1/network/{location_id}/postures"))
        .json(&AssignPosturesData {
            postures: Vec::new(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    expect_network_modified_peers(&mut gateway_rx, location_id, &[device_pubkey]);
    let events = client.drain_all_events();
    assert!(matches!(
        events[0].0,
        ApiEventType::LocationPosturesAssigned { .. }
    ));
}

#[sqlx::test]
async fn test_reassigning_posture_locations_refreshes_old_and_new_locations(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, state) = setup(options).await;
    let mut gateway_rx = state.gateway_rx;

    let net1: serde_json::Value = make_network(&client, "net1").await.json().await;
    let net2: serde_json::Value = make_network(&client, "net2").await.json().await;
    let loc1 = net1["id"].as_i64().unwrap();
    let loc2 = net2["id"].as_i64().unwrap();
    let device_pubkey = create_admin_device(&client).await;
    let posture = create_posture(&client, "Posture").await;
    drain_gateway_events(&mut gateway_rx);
    client.drain_all_events();

    let response = client
        .put(format!("/api/v1/device-posture/{}/locations", posture.id))
        .json(&AssignLocationsData {
            locations: vec![loc1],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    expect_network_modified_peers(&mut gateway_rx, loc1, &[]);
    client.drain_all_events();

    let response = client
        .put(format!("/api/v1/device-posture/{}/locations", posture.id))
        .json(&AssignLocationsData {
            locations: vec![loc2],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    expect_network_modified_peers(&mut gateway_rx, loc1, &[device_pubkey]);
    expect_network_modified_peers(&mut gateway_rx, loc2, &[]);
    let events = client.drain_all_events();
    assert!(matches!(
        events[0].0,
        ApiEventType::DevicePostureLocationsAssigned { .. }
    ));
}

#[sqlx::test]
async fn test_deleting_assigned_posture_refreshes_gateway_with_direct_peers(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, state) = setup(options).await;
    let mut gateway_rx = state.gateway_rx;

    let net: serde_json::Value = make_network(&client, "net").await.json().await;
    let location_id = net["id"].as_i64().unwrap();
    let device_pubkey = create_admin_device(&client).await;
    let posture = create_posture(&client, "Posture").await;
    drain_gateway_events(&mut gateway_rx);
    client.drain_all_events();

    let response = client
        .put(format!("/api/v1/network/{location_id}/postures"))
        .json(&AssignPosturesData {
            postures: vec![posture.id],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    expect_network_modified_peers(&mut gateway_rx, location_id, &[]);
    client.drain_all_events();

    let response = client
        .delete(format!("/api/v1/device-posture/{}", posture.id))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    expect_network_modified_peers(&mut gateway_rx, location_id, &[device_pubkey]);
    let events = client.drain_all_events();
    assert!(matches!(
        events[0].0,
        ApiEventType::DevicePostureDeleted { .. }
    ));
}

#[sqlx::test]
async fn test_device_posture_assignment_not_found(_: PgPoolOptions, options: PgConnectOptions) {
    let (mut client, _) = setup(options).await;

    let response = client
        .put("/api/v1/device-posture/999/locations")
        .json(&AssignLocationsData {
            locations: Vec::new(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    client.assert_event_queue_is_empty();

    let response = client
        .put("/api/v1/network/999/postures")
        .json(&AssignPosturesData {
            postures: Vec::new(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    client.assert_event_queue_is_empty();
}

/// Create a service location (prelogon mode) and return its ID.
async fn make_service_location(client: &TestClient, name: &str) -> i64 {
    let response = client
        .post("/api/v1/network")
        .json(&serde_json::json!({
            "name": name,
            "address": "10.2.2.1/24",
            "port": 55555,
            "endpoint": "192.168.4.15",
            "allowed_ips": "10.2.2.0/24",
            "dns": "1.1.1.1",
            "mtu": 1420,
            "fwmark": 0,
            "allowed_groups": ["admin"],
            "allow_all_groups": false,
            "keepalive_interval": 25,
            "peer_disconnect_threshold": 300,
            "acl_enabled": false,
            "acl_default_allow": false,
            "allowed_ips_from_acl": false,
            "location_mfa_mode": "disabled",
            "service_location_mode": "prelogon"
        }))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let net: serde_json::Value = response.json().await;
    net["id"].as_i64().unwrap()
}

#[sqlx::test]
async fn test_set_postures_for_service_location_rejected(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, _) = setup(options).await;

    let service_location_id = make_service_location(&client, "service-net").await;
    let posture: ApiDevicePosture = client
        .post("/api/v1/device-posture")
        .json(&make_edit("Posture"))
        .send()
        .await
        .json()
        .await;
    client.drain_all_events();

    // assigning posture checks to a service location is rejected
    let response = client
        .put(format!("/api/v1/network/{service_location_id}/postures"))
        .json(&AssignPosturesData {
            postures: vec![posture.id],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    client.assert_event_queue_is_empty();

    // nothing was assigned to the posture
    let response = client
        .get(format!("/api/v1/device-posture/{}", posture.id))
        .send()
        .await;
    let fetched: ApiDevicePosture = response.json().await;
    assert!(fetched.locations.is_empty());

    // clearing (empty list) is still allowed on a service location
    let response = client
        .put(format!("/api/v1/network/{service_location_id}/postures"))
        .json(&AssignPosturesData {
            postures: Vec::new(),
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_set_locations_for_posture_rejects_service_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let (mut client, _) = setup(options).await;

    let regular_net: serde_json::Value = make_network(&client, "regular-net").await.json().await;
    let regular_location_id = regular_net["id"].as_i64().unwrap();
    let service_location_id = make_service_location(&client, "service-net").await;

    let posture: ApiDevicePosture = client
        .post("/api/v1/device-posture")
        .json(&make_edit("Posture"))
        .send()
        .await
        .json()
        .await;
    client.drain_all_events();

    // assigning a service location to a posture is rejected
    let response = client
        .put(format!("/api/v1/device-posture/{}/locations", posture.id))
        .json(&AssignLocationsData {
            locations: vec![service_location_id],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    client.assert_event_queue_is_empty();

    // a mix containing a service location is rejected too — nothing is assigned
    let response = client
        .put(format!("/api/v1/device-posture/{}/locations", posture.id))
        .json(&AssignLocationsData {
            locations: vec![regular_location_id, service_location_id],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    client.assert_event_queue_is_empty();

    let response = client
        .get(format!("/api/v1/device-posture/{}", posture.id))
        .send()
        .await;
    let fetched: ApiDevicePosture = response.json().await;
    assert!(fetched.locations.is_empty());

    // assigning only regular locations still works
    let response = client
        .put(format!("/api/v1/device-posture/{}/locations", posture.id))
        .json(&AssignLocationsData {
            locations: vec![regular_location_id],
        })
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let result: Vec<i64> = response.json().await;
    assert_eq!(result, vec![regular_location_id]);
}
