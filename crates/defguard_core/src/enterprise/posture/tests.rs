use chrono::{TimeDelta, Utc};
use defguard_common::db::{
    models::wireguard::{LocationMfaMode, ServiceLocationMode},
    setup_pool,
};
use defguard_proto::enterprise::posture::{
    BoolCheck, DevicePostureCheckRequest, DevicePostureData, StringCheck, UnavailableReason,
    bool_check, string_check,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use crate::enterprise::{
    db::models::device_posture::{DevicePosture, DevicePostureLocation, DevicePostureOsRule, OsType},
    license::{License, LicenseTier, SupportType, set_cached_license},
    limits::{Counts, set_counts},
    posture::validate_posture,
};
use crate::grpc::proto::enterprise::license::LicenseLimits;
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

fn linux_posture_data(os_version: &str, disk_encryption: bool) -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: "1.6.0".to_string(),
        os_type: "linux".to_string(),
        os_version: Some(string_check_value(os_version)),
        disk_encryption: Some(bool_check_value(disk_encryption)),
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

    DevicePostureLocation::set_for_location(&mut pool.acquire().await.unwrap(), location_id, &[policy.id])
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
    data.disk_encryption =
        Some(bool_check_unavailable(UnavailableReason::InsufficientPermissions));

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
