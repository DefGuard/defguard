use chrono::{TimeDelta, Utc};
use defguard_common::db::{
    models::{
        WireguardNetwork,
        wireguard::{LocationMfaMode, ServiceLocationMode},
    },
    setup_pool,
};
use defguard_proto::enterprise::posture::{
    BoolCheck, DevicePostureCheckRequest, DevicePostureData, Int32Check, StringCheck,
    UnavailableReason, bool_check, int32_check, string_check,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::{
    enterprise::{
        db::models::device_posture::{
            DevicePosture, DevicePostureLocation, DevicePostureOsRule, OsType,
        },
        license::{License, LicenseTier, SupportType, set_cached_license},
        limits::{Counts, set_counts},
        posture::evaluation::validate_posture,
    },
    grpc::proto::enterprise::license::LicenseLimits,
};

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
        "test".to_owned(),
        true,
        Some(Utc::now() + TimeDelta::days(1)),
        Some(limits),
        None,
        LicenseTier::Enterprise,
        SupportType::Basic,
        vec![],
    );
    set_cached_license(Some(license));
    set_counts(Counts::new(1, 1, 1, 1));
}

async fn create_location(pool: &sqlx::PgPool) -> i64 {
    WireguardNetwork::new(
        "test-location".to_owned(),
        51820,
        "endpoint".to_owned(),
        None,
        Vec::<ipnetwork::IpNetwork>::new(),
        true,
        false,
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
        result: Some(string_check::Result::Value(s.to_owned())),
    }
}

fn int32_check_value(v: i32) -> Int32Check {
    Int32Check {
        result: Some(int32_check::Result::Value(v)),
    }
}

fn linux_posture_data(os_version: &str, disk_encryption: bool) -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "linux".to_owned(),
        os_version: Some(string_check_value(os_version)),
        disk_encryption: Some(bool_check_value(disk_encryption)),
        ..Default::default()
    }
}

fn linux_posture_data_with_kernel(kernel_version: &str) -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "linux".to_owned(),
        linux_kernel_version: Some(string_check_value(kernel_version)),
        ..Default::default()
    }
}

fn windows_posture_data() -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "windows".to_owned(),
        os_version: Some(string_check_value("11.0")),
        disk_encryption: Some(bool_check_value(true)),
        antivirus_present: Some(bool_check_value(true)),
        windows_ad_domain_joined: Some(bool_check_value(true)),
        windows_security_update_age_days: Some(int32_check_value(0)),
        ..Default::default()
    }
}

fn make_request(location_id: i64, data: Option<DevicePostureData>) -> DevicePostureCheckRequest {
    DevicePostureCheckRequest {
        location_id,
        pubkey: "testpubkey".to_owned(),
        device_posture_data: data,
    }
}

/// Creates a Linux posture policy with no OS version requirement (Linux has no version list).
async fn save_linux_policy(
    pool: &sqlx::PgPool,
    location_id: i64,
    disk_encryption_required: Option<bool>,
    min_client_version: Option<&str>,
    allow_prerelease_client: bool,
) {
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "test-policy".to_owned(),
        description: None,
        min_client_version: min_client_version.map(str::to_owned),
        allow_prerelease_client,
    }
    .save(pool)
    .await
    .unwrap();

    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Linux,
        min_os_version: None,
        disk_encryption_required,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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

/// Creates a Windows posture policy requiring a minimum OS major version.
/// Windows has a known version list `[10, 11]`, making it suitable for OS version tests.
async fn save_windows_os_version_policy(
    pool: &sqlx::PgPool,
    location_id: i64,
    min_os_version: i32,
    disk_encryption_required: Option<bool>,
) {
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "windows-os-version-policy".to_owned(),
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
        min_os_version: Some(min_os_version),
        disk_encryption_required,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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

async fn save_windows_policy(
    pool: &sqlx::PgPool,
    location_id: i64,
    antivirus_required: Option<bool>,
    ad_domain_joined_required: Option<bool>,
    windows_security_update_max_age: Option<i32>,
) {
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "windows-policy".to_owned(),
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
        windows_security_update_max_age,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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

async fn save_android_policy(
    pool: &sqlx::PgPool,
    location_id: i64,
    android_security_patch_level_max_age: Option<i32>,
) {
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "android-policy".to_owned(),
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
        os_type: OsType::Android,
        min_os_version: None,
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age,
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

fn android_posture_data(patch_date: &str) -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "android".to_owned(),
        os_version: Some(string_check_value("14.0")),
        android_security_patch_date: Some(StringCheck {
            result: Some(string_check::Result::Value(patch_date.to_owned())),
        }),
        ..Default::default()
    }
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

/// Both OS version (Windows 10 required, device on 11) and disk encryption pass.
#[sqlx::test]
async fn pass_all_checks_met(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_os_version_policy(&pool, location_id, 10, Some(true)).await;

    let result = validate_posture(
        &pool,
        &make_request(location_id, Some(windows_posture_data())),
    )
    .await
    .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

/// Policy requires Windows 11; device reports exactly "11.0" — boundary must pass.
#[sqlx::test]
async fn pass_boundary_os_version_exact(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_os_version_policy(&pool, location_id, 11, None).await;

    let data = DevicePostureData {
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "windows".to_owned(),
        os_version: Some(string_check_value("11.0")),
        ..Default::default()
    };

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn fail_missing_posture_data(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, None, true).await;

    let result = validate_posture(&pool, &make_request(location_id, None))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::MissingPostureData)
    ));
}

/// Device reports OS version 99 (not in any known list) — must produce UnrecognizedVersion.
#[sqlx::test]
async fn fail_unrecognized_os_version(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    // Windows policy requiring min_os_version 11 — device claims version 99 (unknown).
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "win-unrecognized".to_owned(),
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
        min_os_version: Some(11),
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "windows".to_owned(),
        os_version: Some(string_check_value("99.0")),
        ..Default::default()
    };

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(
        matches!(
            result,
            super::PostureResult::Fail(ref reasons) if reasons.len() == 1
                && matches!(reasons[0], super::FailureReason::UnrecognizedVersion { check: "os_version", .. })
        ),
        "expected UnrecognizedVersion for Windows OS version 99"
    );
}

/// Device on a known-but-old OS version still produces OsVersionTooOld (regression guard).
#[sqlx::test]
async fn fail_os_version_too_old_regression(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    // Windows policy requiring 11 - device reports 10 (known, but too old).
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "win-too-old".to_owned(),
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
        min_os_version: Some(11),
        disk_encryption_required: None,
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "windows".to_owned(),
        os_version: Some(string_check_value("10.0")),
        ..Default::default()
    };

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(
        matches!(
            result,
            super::PostureResult::Fail(ref reasons) if reasons.len() == 1
                && matches!(reasons[0], super::FailureReason::OsVersionTooOld { required: 11, .. })
        ),
        "expected OsVersionTooOld for Windows 10 against required 11"
    );
}

/// Device reports kernel version 99 (not in LINUX_KERNEL_VERSIONS) - must produce UnrecognizedVersion.
#[sqlx::test]
async fn fail_unrecognized_kernel_version(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "kernel-unrecognized".to_owned(),
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
        windows_security_update_max_age: None,
        min_kernel_version: Some(6),
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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

    let data = linux_posture_data_with_kernel("99.0.0");

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(
        matches!(
            result,
            super::PostureResult::Fail(ref reasons) if reasons.len() == 1
                && matches!(reasons[0], super::FailureReason::UnrecognizedVersion { check: "linux_kernel_version", .. })
        ),
        "expected UnrecognizedVersion for kernel version 99"
    );
}

/// Client reports version 1.7.0 (major.minor "1.7" not in CLIENT_VERSIONS) — UnrecognizedVersion.
#[sqlx::test]
async fn fail_unrecognized_client_version(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, Some("1.6"), true).await;

    let mut data = linux_posture_data("6.1.0", true);
    data.defguard_client_version = "1.7.0".to_owned();

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(
        matches!(
            result,
            super::PostureResult::Fail(ref reasons) if reasons.len() == 1
                && matches!(reasons[0], super::FailureReason::UnrecognizedVersion { check: "client_version", .. })
        ),
        "expected UnrecognizedVersion for client 1.7.0"
    );
}

/// Client on known version 1.6.x that meets the minimum still passes.
#[sqlx::test]
async fn pass_known_client_version_meets_minimum(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, Some("2.1"), true).await;

    let mut data = linux_posture_data("6.1.0", true);
    data.defguard_client_version = "2.1.2".to_owned();

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(
        matches!(result, super::PostureResult::Pass),
        "expected Pass for client 2.1.2 against required 2.1"
    );
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
async fn pass_security_update_within_max_age(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_policy(&pool, location_id, None, None, Some(30)).await;

    let mut data = windows_posture_data();
    data.windows_security_update_age_days = Some(int32_check_value(15));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
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
        name: "kernel-policy".to_owned(),
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
        windows_security_update_max_age: None,
        min_kernel_version: Some(6),
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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
        name: "integrity-policy".to_owned(),
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
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: Some(true),
        android_security_patch_level_max_age: None,
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
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "macos".to_owned(),
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
        name: "windows-only".to_owned(),
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
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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

    save_linux_policy(&pool, location_id, Some(true), None, true).await;

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

/// Policy requires Windows 11; device reports "10.0" (known but too old).
#[sqlx::test]
async fn fail_os_version_too_old(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_os_version_policy(&pool, location_id, 11, None).await;

    let data = DevicePostureData {
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "windows".to_owned(),
        os_version: Some(string_check_value("10.0")),
        ..Default::default()
    };

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::OsVersionTooOld { .. })
    ));
}

/// Policy requires Windows 11 (major only); device reports "11.5" (same major, non-zero minor).
/// Must pass because OS version comparison is major-only.
#[sqlx::test]
async fn pass_os_version_same_major_lower_minor(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_os_version_policy(&pool, location_id, 11, None).await;

    let data = DevicePostureData {
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "windows".to_owned(),
        os_version: Some(string_check_value("11.5")),
        ..Default::default()
    };

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
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

    save_linux_policy(&pool, location_id, None, Some("2.2"), true).await;

    let mut data = linux_posture_data("22.04", true);
    data.defguard_client_version = "2.1.2".to_owned();

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::ClientVersionTooOld { ref required, .. } if required == "2.2")
    ));
}

#[sqlx::test]
async fn pass_accept_prerelease(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, Some("2.1"), true).await;
    let mut data = linux_posture_data("22.04", true);
    data.defguard_client_version = "2.1.0-alpha".to_owned();

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(
        matches!(result, super::PostureResult::Pass),
        "expected Pass for prerelease but got Fail"
    );
}

#[sqlx::test]
async fn fail_prerelease_not_allowed(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_linux_policy(&pool, location_id, None, None, false).await;

    let mut data = linux_posture_data("22.04", true);
    data.defguard_client_version = "1.6.0-beta1".to_owned();

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

    save_linux_policy(&pool, location_id, Some(true), None, true).await;

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

    save_linux_policy(&pool, location_id, Some(true), None, true).await;

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

    save_linux_policy(&pool, location_id, Some(true), None, true).await;

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
        name: "policy-a".to_owned(),
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
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
    }
    .save(&pool)
    .await
    .unwrap();

    // Policy B: requires disk encryption — will fail.
    let policy_b = DevicePosture {
        id: defguard_common::db::NoId,
        name: "policy-b".to_owned(),
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
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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

    save_linux_policy(&pool, location_id, None, None, true).await;

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
async fn fail_security_update_too_old(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_windows_policy(&pool, location_id, None, None, Some(30)).await;

    let mut data = windows_posture_data();
    data.windows_security_update_age_days = Some(int32_check_value(90));

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::SecurityUpdateTooOld { .. })
    ));
}

#[sqlx::test]
async fn fail_kernel_version_too_old(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "kernel-policy".to_owned(),
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
        windows_security_update_max_age: None,
        min_kernel_version: Some(6),
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
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
        name: "integrity-policy".to_owned(),
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
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: Some(true),
        android_security_patch_level_max_age: None,
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
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "macos".to_owned(),
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

    save_linux_policy(&pool, location_id, Some(true), None, true).await;

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

#[sqlx::test]
async fn pass_android_security_patch_within_max_age(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_android_policy(&pool, location_id, Some(30)).await;

    // Patch date 15 days ago — within the 30-day limit.
    let patch_date = (Utc::now() - TimeDelta::days(15))
        .format("%Y-%m-%d")
        .to_string();
    let data = android_posture_data(&patch_date);

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(result, super::PostureResult::Pass));
}

#[sqlx::test]
async fn fail_android_security_patch_too_old(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_android_policy(&pool, location_id, Some(30)).await;

    // Patch date 90 days ago — exceeds the 30-day limit.
    let patch_date = (Utc::now() - TimeDelta::days(90))
        .format("%Y-%m-%d")
        .to_string();
    let data = android_posture_data(&patch_date);

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::AndroidSecurityPatchTooOld { .. })
    ));
}

#[sqlx::test]
async fn fail_android_security_patch_unparseable(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    set_enterprise_license();
    let location_id = create_location(&pool).await;

    save_android_policy(&pool, location_id, Some(30)).await;

    let data = android_posture_data("not-a-date");

    let result = validate_posture(&pool, &make_request(location_id, Some(data)))
        .await
        .unwrap();

    assert!(matches!(
        result,
        super::PostureResult::Fail(ref reasons) if reasons.len() == 1
            && matches!(reasons[0], super::FailureReason::CheckUnavailable { .. })
    ));
}
