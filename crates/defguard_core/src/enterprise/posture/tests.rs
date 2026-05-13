use chrono::{TimeDelta, Utc};
use defguard_common::db::{
    models::wireguard::{LocationMfaMode, ServiceLocationMode},
    setup_pool,
};
use defguard_proto::enterprise::posture::{
    BoolCheck, DevicePostureCheckRequest, DevicePostureData, StringCheck, UnavailableReason,
    bool_check, string_check, string_check::Result as StringResult,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::{
    enterprise::{
        db::models::device_posture::{
            DevicePosture, DevicePostureLocation, DevicePostureOsRule, OsType,
        },
        license::{License, LicenseTier, SupportType, set_cached_license},
        limits::{Counts, set_counts},
        posture::validate_posture,
    },
    grpc::proto::enterprise::license::LicenseLimits,
};
use defguard_common::db::models::WireguardNetwork;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn set_enterprise_license() {
    let limits = LicenseLimits {
        users: 100,
        devices: 100,
        locations: 100,
        network_devices: Some(100),
    };
    let license = License::new(
        "test".to_string(),
        true,
        Some(Utc::now() + TimeDelta::days(1)),
        Some(limits),
        None,
        LicenseTier::Enterprise,
        SupportType::Basic,
    );
    set_cached_license(Some(license));
    set_counts(Counts::new(1, 1, 1, 1));
}

async fn create_location(pool: &sqlx::PgPool) -> i64 {
    WireguardNetwork::new(
        "test-location".to_string(),
        51820,
        "endpoint".to_string(),
        None,
        Vec::<ipnetwork::IpNetwork>::new(),
        true,
        false,
        false,
        LocationMfaMode::Disabled,
        ServiceLocationMode::Disabled,
    )
    .save(pool)
    .await
    .unwrap()
    .id
}

fn bool_check_value(v: bool) -> BoolCheck {
    BoolCheck {
        result: Some(bool_check::Result::Value(v)),
    }
}

fn bool_check_unavailable(reason: UnavailableReason) -> BoolCheck {
    BoolCheck {
        result: Some(bool_check::Result::Unavailable(reason as i32)),
    }
}

fn string_check_value(s: &str) -> StringCheck {
    StringCheck {
        result: Some(string_check::Result::Value(s.to_string())),
    }
}

fn string_check_unavailable(reason: UnavailableReason) -> StringCheck {
    StringCheck {
        result: Some(StringResult::Unavailable(reason as i32)),
    }
}

fn linux_posture_data(os_version: &str, disk_encryption: bool) -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: "1.6.0".to_string(),
        os_type: "linux".to_string(),
        os_version: Some(string_check_value(os_version)),
        disk_encryption: Some(bool_check_value(disk_encryption)),
        ..Default::default()
    }
}

fn windows_posture_data() -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: "1.6.0".to_string(),
        os_type: "windows".to_string(),
        os_version: Some(string_check_value("11.0")),
        disk_encryption: Some(bool_check_value(true)),
        antivirus_present: Some(bool_check_value(true)),
        windows_ad_domain_joined: Some(bool_check_value(true)),
        windows_security_update_current: Some(bool_check_value(true)),
        ..Default::default()
    }
}

fn make_request(location_id: i64, data: Option<DevicePostureData>) -> DevicePostureCheckRequest {
    DevicePostureCheckRequest {
        location_id,
        pubkey: "testpubkey".to_string(),
        device_posture_data: data,
    }
}

async fn save_linux_policy(
    pool: &sqlx::PgPool,
    location_id: i64,
    min_os_version: Option<&str>,
    disk_encryption_required: Option<bool>,
    min_client_version: Option<&str>,
    allow_prerelease_client: bool,
) {
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "test-policy".to_string(),
        description: None,
        min_client_version: min_client_version.map(str::to_string),
        allow_prerelease_client,
    }
    .save(pool)
    .await
    .unwrap();

    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Linux,
        min_os_version: min_os_version.map(str::to_string),
        disk_encryption_required,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_current: None,
        min_kernel_version: None,
        device_integrity_required: None,
    }
    .save(pool)
    .await
    .unwrap();

    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.unwrap(),
        location_id,
        &[policy.id],
    )
    .await
    .unwrap();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test]
async fn pass_no_posture_assigned(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("22.04", true))),
    )
    .await
    .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn pass_all_checks_met(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, Some("20.04"), Some(true), None, true).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("22.04", true))),
    )
    .await
    .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn pass_boundary_os_version_exact(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, Some("22.04"), None, None, true).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("22.04", true))),
    )
    .await
    .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn fail_missing_posture_data(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, None, None, true).await;

    let result = validate_posture(&pool, &make_request(location_id, None))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::MissingPostureData)
    ));
}

#[sqlx::test]
async fn pass_antivirus_present(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_policy(&pool, location_id, Some(true), None, None).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(windows_posture_data())),
    )
    .await
    .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn pass_ad_domain_joined(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_policy(&pool, location_id, None, Some(true), None).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(windows_posture_data())),
    )
    .await
    .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn pass_security_update_current(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_policy(&pool, location_id, None, None, Some(true)).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(windows_posture_data())),
    )
    .await
    .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn pass_kernel_version_meets_minimum(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "kernel-policy".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: true,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Linux,
        min_os_version: None,
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_current: None,
        min_kernel_version: Some("6.1.0".to_string()),
        device_integrity_required: None,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.unwrap(),
        location_id,
        &[policy.id],
    )
    .await
    .unwrap();

    let mut data = linux_posture_data("22.04", true);
    data.linux_kernel_version = Some(string_check_value("6.8.0"));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn pass_device_integrity_ok(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "integrity-policy".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: true,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Macos,
        min_os_version: None,
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_current: None,
        min_kernel_version: None,
        device_integrity_required: Some(true),
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.unwrap(),
        location_id,
        &[policy.id],
    )
    .await
    .unwrap();

    let data = DevicePostureData {
        defguard_client_version: "1.6.0".to_string(),
        os_type: "macos".to_string(),
        device_integrity: Some(bool_check_value(true)),
        ..Default::default()
    };

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn fail_os_not_in_policy(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    // Policy only has a Windows rule; device reports Linux.
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "windows-only".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: true,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Windows,
        min_os_version: None,
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_current: None,
        min_kernel_version: None,
        device_integrity_required: None,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.unwrap(),
        location_id,
        &[policy.id],
    )
    .await
    .unwrap();

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("22.04", true))),
    )
    .await
    .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::OsNotAllowed)
    ));
}

#[sqlx::test]
async fn fail_disk_encryption_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, Some(true), None, true).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("22.04", false))),
    )
    .await
    .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::DiskEncryptionRequired)
    ));
}

#[sqlx::test]
async fn fail_os_version_too_old(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, Some("22.04"), None, None, true).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("20.04", true))),
    )
    .await
    .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::OsVersionTooOld { .. })
    ));
}

/// Regression guard: policy requires "22.10", device reports "22.04".
/// Under major-only comparison this must PASS because major versions are equal.
#[sqlx::test]
async fn pass_os_version_same_major_lower_minor(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    // Require 22.10 — device has 22.04 (same major, older minor).
    save_linux_policy(&pool, location_id, Some("22.10"), None, None, true).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("22.04", true))),
    )
    .await
    .unwrap();

    assert!(
        matches!(result, super::PostureResult::Pass),
        "expected Pass for same-major OS version but got Fail"
    );
}

#[sqlx::test]
async fn fail_client_version_too_old(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, None, Some("1.5.0"), true).await;

    let mut data = linux_posture_data("22.04", true);
    data.defguard_client_version = "1.4.0".to_string();

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::ClientVersionTooOld { .. })
    ));
}

#[sqlx::test]
async fn fail_prerelease_not_allowed(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, None, None, false).await;

    let mut data = linux_posture_data("22.04", true);
    data.defguard_client_version = "1.6.0-beta1".to_string();

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::PrereleaseClientNotAllowed)
    ));
}

#[sqlx::test]
async fn fail_check_unavailable_detection_failed(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, Some(true), None, true).await;

    let mut data = linux_posture_data("22.04", true);
    data.disk_encryption = Some(bool_check_unavailable(UnavailableReason::DetectionFailed));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::CheckUnavailable { .. })
    ));
}

#[sqlx::test]
async fn fail_check_unavailable_insufficient_permissions(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, Some(true), None, true).await;

    let mut data = linux_posture_data("22.04", true);
    data.disk_encryption = Some(bool_check_unavailable(
        UnavailableReason::InsufficientPermissions,
    ));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::CheckUnavailable { .. })
    ));
}

#[sqlx::test]
async fn pass_check_not_applicable(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, Some(true), None, true).await;

    let mut data = linux_posture_data("22.04", true);
    data.disk_encryption = Some(bool_check_unavailable(UnavailableReason::NotApplicable));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn fail_multi_policy_and_logic(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    // Policy A: passes (no strict requirements).
    let policy_a = DevicePosture {
        id: defguard_common::db::NoId,
        name: "policy-a".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: true,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy_a.id,
        os_type: OsType::Linux,
        min_os_version: None,
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_current: None,
        min_kernel_version: None,
        device_integrity_required: None,
    }
    .save(&pool)
    .await
    .unwrap();

    // Policy B: requires disk encryption — will fail.
    let policy_b = DevicePosture {
        id: defguard_common::db::NoId,
        name: "policy-b".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: true,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy_b.id,
        os_type: OsType::Linux,
        min_os_version: None,
        disk_encryption_required: Some(true),
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_current: None,
        min_kernel_version: None,
        device_integrity_required: None,
    }
    .save(&pool)
    .await
    .unwrap();

    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.unwrap(),
        location_id,
        &[policy_a.id, policy_b.id],
    )
    .await
    .unwrap();

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("22.04", false))),
    )
    .await
    .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons)
            if reasons.iter().any(|r| matches!(r, super::FailureReason::DiskEncryptionRequired))
    ));
}

#[sqlx::test]
async fn fail_enterprise_inactive(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_cached_license(None);
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, None, None, true).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(linux_posture_data("22.04", true))),
    )
    .await;

    assert!(matches!(
        result,
        Err(super::PostureCheckError::NoActiveEnterpriseLicense)
    ));
}

async fn save_windows_policy(
    pool: &sqlx::PgPool,
    location_id: i64,
    antivirus_required: Option<bool>,
    ad_domain_joined_required: Option<bool>,
    windows_security_update_current: Option<bool>,
) {
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "windows-policy".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: true,
    }
    .save(pool)
    .await
    .unwrap();

    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Windows,
        min_os_version: None,
        disk_encryption_required: None,
        antivirus_required,
        ad_domain_joined_required,
        windows_security_update_current,
        min_kernel_version: None,
        device_integrity_required: None,
    }
    .save(pool)
    .await
    .unwrap();

    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.unwrap(),
        location_id,
        &[policy.id],
    )
    .await
    .unwrap();
}

#[sqlx::test]
async fn fail_antivirus_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_policy(&pool, location_id, Some(true), None, None).await;

    let mut data = windows_posture_data();
    data.antivirus_present = Some(bool_check_value(false));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::AntivirusRequired)
    ));
}

#[sqlx::test]
async fn fail_ad_domain_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_policy(&pool, location_id, None, Some(true), None).await;

    let mut data = windows_posture_data();
    data.windows_ad_domain_joined = Some(bool_check_value(false));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::AdDomainRequired)
    ));
}

#[sqlx::test]
async fn fail_security_update_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_policy(&pool, location_id, None, None, Some(true)).await;

    let mut data = windows_posture_data();
    data.windows_security_update_current = Some(bool_check_value(false));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::SecurityUpdateRequired)
    ));
}

#[sqlx::test]
async fn fail_kernel_version_too_old(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "kernel-policy".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: true,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Linux,
        min_os_version: None,
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_current: None,
        min_kernel_version: Some("6.1.0".to_string()),
        device_integrity_required: None,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.unwrap(),
        location_id,
        &[policy.id],
    )
    .await
    .unwrap();

    let mut data = linux_posture_data("22.04", true);
    data.linux_kernel_version = Some(string_check_value("5.15.0"));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.iter().any(|r| matches!(r, super::FailureReason::KernelVersionTooOld { .. }))
    ));
}

#[sqlx::test]
async fn fail_device_integrity_required(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "integrity-policy".to_string(),
        description: None,
        min_client_version: None,
        allow_prerelease_client: true,
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Macos,
        min_os_version: None,
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_current: None,
        min_kernel_version: None,
        device_integrity_required: Some(true),
    }
    .save(&pool)
    .await
    .unwrap();
    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.unwrap(),
        location_id,
        &[policy.id],
    )
    .await
    .unwrap();

    let data = DevicePostureData {
        defguard_client_version: "1.6.0".to_string(),
        os_type: "macos".to_string(),
        device_integrity: Some(bool_check_value(false)),
        ..Default::default()
    };

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::DeviceIntegrityRequired)
    ));
}

#[sqlx::test]
async fn fail_check_unavailable_unspecified(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, Some(true), None, true).await;

    let mut data = linux_posture_data("22.04", true);
    data.disk_encryption = Some(bool_check_unavailable(UnavailableReason::Unspecified));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::CheckUnavailable { .. })
    ));
}
