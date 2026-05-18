use defguard_common::db::Id;
use defguard_proto::enterprise::posture::{
    DevicePostureData, UnavailableReason, bool_check::Result as BoolResult,
    string_check::Result as StringResult,
};
use sqlx::PgPool;

use super::{
    FailureReason, PostureCheckError, PostureResult,
    version::{
        major_version_in_list, major_version_meets_minimum, minor_version_in_list,
        version_meets_minimum,
    },
    version_list::{
        ANDROID_OS_VERSIONS, CLIENT_VERSIONS, IOS_OS_VERSIONS, LINUX_KERNEL_VERSIONS,
        MACOS_OS_VERSIONS, WINDOWS_OS_VERSIONS,
    },
};
use crate::enterprise::{
    db::models::device_posture::{
        DevicePosture, DevicePostureLocation, DevicePostureOsRule, OsType,
    },
    is_enterprise_license_active,
};

/// Resolves a `BoolCheck` signal:
/// - `Value(true)`  → Ok(true)
/// - `Value(false)` → Ok(false)
/// - `NotApplicable` → Ok(true)  (not applicable means the check is irrelevant for this OS)
/// - `InsufficientPermissions` / `DetectionFailed` / absent → Err(check_name)
fn resolve_bool_check(
    signal: Option<&defguard_proto::enterprise::posture::BoolCheck>,
    check_name: &'static str,
) -> Result<bool, &'static str> {
    match signal.and_then(|c| c.result.as_ref()) {
        Some(BoolResult::Value(v)) => Ok(*v),
        Some(BoolResult::Unavailable(code)) => match UnavailableReason::try_from(*code) {
            Ok(UnavailableReason::NotApplicable) => Ok(true),
            _ => Err(check_name),
        },
        None => Err(check_name),
    }
}

/// Resolves a `StringCheck` signal.
/// Returns `None` when the value is `NotApplicable` (skip the check silently).
/// Returns `Err(check_name)` for unresolvable unavailability or absent field.
fn resolve_string_check(
    signal: Option<&defguard_proto::enterprise::posture::StringCheck>,
    check_name: &'static str,
) -> Result<Option<String>, &'static str> {
    match signal.and_then(|c| c.result.as_ref()) {
        Some(StringResult::Value(v)) => Ok(Some(v.clone())),
        Some(StringResult::Unavailable(code)) => match UnavailableReason::try_from(*code) {
            Ok(UnavailableReason::NotApplicable) => Ok(None),
            _ => Err(check_name),
        },
        None => Err(check_name),
    }
}

fn parse_os_type(s: &str) -> Option<OsType> {
    match s.to_lowercase().as_str() {
        "windows" => Some(OsType::Windows),
        "macos" | "darwin" => Some(OsType::Macos),
        "linux" => Some(OsType::Linux),
        "ios" => Some(OsType::Ios),
        "android" => Some(OsType::Android),
        _ => None,
    }
}

/// Evaluates all per-OS DB fields from `rule` against the signals in `data`,
/// appending any [`FailureReason`]s to `failures`.
///
/// OS and kernel version comparisons use major-only semantics: a device running
/// the same major release as the policy minimum always passes regardless of
/// minor or patch differences. Client version comparisons use full semver.
///
/// For OS and kernel versions, the device-reported major must also appear in the
/// known version list for its OS type. An unrecognized version produces
/// [`FailureReason::UnrecognizedVersion`] regardless of the minimum comparison.
fn evaluate_os_rule(
    rule: &DevicePostureOsRule<defguard_common::db::Id>,
    data: &DevicePostureData,
    failures: &mut Vec<FailureReason>,
) {
    let os_version_list: &[i32] = match rule.os_type {
        OsType::Windows => WINDOWS_OS_VERSIONS,
        OsType::Macos => MACOS_OS_VERSIONS,
        OsType::Ios => IOS_OS_VERSIONS,
        OsType::Android => ANDROID_OS_VERSIONS,
        OsType::Linux => &[], // Linux uses kernel version list, not OS version list
    };

    // min_os_version
    if let Some(required) = rule.min_os_version {
        match resolve_string_check(data.os_version.as_ref(), "os_version") {
            Ok(Some(actual)) => match major_version_in_list(&actual, os_version_list) {
                None => failures.push(FailureReason::CheckUnavailable {
                    check: "os_version (unparseable)",
                }),
                Some(false) => failures.push(FailureReason::UnrecognizedVersion {
                    check: "os_version",
                    actual,
                }),
                Some(true) => match major_version_meets_minimum(required, &actual) {
                    Some(true) => {}
                    Some(false) => {
                        failures.push(FailureReason::OsVersionTooOld { required, actual })
                    }
                    None => failures.push(FailureReason::CheckUnavailable {
                        check: "os_version (unparseable)",
                    }),
                },
            },
            Ok(None) => {} // NotApplicable — skip
            Err(name) => failures.push(FailureReason::CheckUnavailable { check: name }),
        }
    }

    // disk_encryption_required
    if rule.disk_encryption_required == Some(true) {
        match resolve_bool_check(data.disk_encryption.as_ref(), "disk_encryption") {
            Ok(true) => {}
            Ok(false) => failures.push(FailureReason::DiskEncryptionRequired),
            Err(name) => failures.push(FailureReason::CheckUnavailable { check: name }),
        }
    }

    // antivirus_required
    if rule.antivirus_required == Some(true) {
        match resolve_bool_check(data.antivirus_present.as_ref(), "antivirus_present") {
            Ok(true) => {}
            Ok(false) => failures.push(FailureReason::AntivirusRequired),
            Err(name) => failures.push(FailureReason::CheckUnavailable { check: name }),
        }
    }

    // ad_domain_joined_required (Windows only)
    if rule.ad_domain_joined_required == Some(true) {
        match resolve_bool_check(
            data.windows_ad_domain_joined.as_ref(),
            "windows_ad_domain_joined",
        ) {
            Ok(true) => {}
            Ok(false) => failures.push(FailureReason::AdDomainRequired),
            Err(name) => failures.push(FailureReason::CheckUnavailable { check: name }),
        }
    }

    // windows_security_update_current
    if rule.windows_security_update_current == Some(true) {
        match resolve_bool_check(
            data.windows_security_update_current.as_ref(),
            "windows_security_update_current",
        ) {
            Ok(true) => {}
            Ok(false) => failures.push(FailureReason::SecurityUpdateRequired),
            Err(name) => failures.push(FailureReason::CheckUnavailable { check: name }),
        }
    }

    // min_kernel_version (Linux only)
    if let Some(required) = rule.min_kernel_version {
        match resolve_string_check(data.linux_kernel_version.as_ref(), "linux_kernel_version") {
            Ok(Some(actual)) => match major_version_in_list(&actual, LINUX_KERNEL_VERSIONS) {
                None => failures.push(FailureReason::CheckUnavailable {
                    check: "linux_kernel_version (unparseable)",
                }),
                Some(false) => failures.push(FailureReason::UnrecognizedVersion {
                    check: "linux_kernel_version",
                    actual,
                }),
                Some(true) => match major_version_meets_minimum(required, &actual) {
                    Some(true) => {}
                    Some(false) => {
                        failures.push(FailureReason::KernelVersionTooOld { required, actual })
                    }
                    None => failures.push(FailureReason::CheckUnavailable {
                        check: "linux_kernel_version (unparseable)",
                    }),
                },
            },
            Ok(None) => {}
            Err(name) => failures.push(FailureReason::CheckUnavailable { check: name }),
        }
    }

    // device_integrity_required (macOS, Android)
    if rule.device_integrity_required == Some(true) {
        match resolve_bool_check(data.device_integrity.as_ref(), "device_integrity") {
            Ok(true) => {}
            Ok(false) => failures.push(FailureReason::DeviceIntegrityRequired),
            Err(name) => failures.push(FailureReason::CheckUnavailable { check: name }),
        }
    }
}

/// Evaluates posture signals against all policies assigned to the location.
///
/// Returns [`PostureResult::Pass`] when no postures are assigned or all pass.
/// Returns [`PostureResult::Fail`] with accumulated [`FailureReason`]s otherwise.
pub async fn validate_posture(
    pool: &PgPool,
    location_id: Id,
    pubkey: &str,
    posture_data: &Option<DevicePostureData>,
) -> Result<PostureResult, PostureCheckError> {
    debug!(
        "Performing posture check for device {}: {:?}",
        pubkey, posture_data
    );

    // If location has no assigned postures - pass immediately (no license required).
    let posture_ids = DevicePostureLocation::find_by_location(pool, location_id).await?;
    if posture_ids.is_empty() {
        debug!(
            "No posture policies assigned to location {} — passing device {}",
            location_id, pubkey
        );
        return Ok(PostureResult::Pass);
    }

    // Policies exist - enforce the enterprise license.
    if !is_enterprise_license_active() {
        warn!(
            "No active enterprise license - posture check aborted for device {}",
            pubkey
        );
        return Err(PostureCheckError::NoActiveEnterpriseLicense);
    }

    let data = match posture_data.as_ref() {
        Some(d) => d,
        None => {
            info!(
                "Missing posture data - posture check failed for device {}",
                pubkey
            );
            return Ok(PostureResult::Fail(vec![FailureReason::MissingPostureData]));
        }
    };

    let os_type = parse_os_type(&data.os_type);
    let mut all_failures: Vec<FailureReason> = Vec::new();

    for posture_id in posture_ids {
        let Some(policy) = DevicePosture::find_by_id(pool, posture_id).await? else {
            warn!("Posture policy {posture_id} not found — skipping");
            continue;
        };

        // Policy-level: client version checks.
        if let Some(ref required) = policy.min_client_version {
            let actual = &data.defguard_client_version;
            if actual.is_empty() {
                all_failures.push(FailureReason::CheckUnavailable {
                    check: "defguard_client_version",
                });
            } else {
                match minor_version_in_list(actual, CLIENT_VERSIONS) {
                    None => all_failures.push(FailureReason::CheckUnavailable {
                        check: "defguard_client_version (unparseable)",
                    }),
                    Some(false) => all_failures.push(FailureReason::UnrecognizedVersion {
                        check: "client_version",
                        actual: actual.clone(),
                    }),
                    Some(true) => match version_meets_minimum(required, actual) {
                        Some(true) => {}
                        Some(false) => all_failures.push(FailureReason::ClientVersionTooOld {
                            required: required.clone(),
                            actual: actual.clone(),
                        }),
                        None => all_failures.push(FailureReason::CheckUnavailable {
                            check: "defguard_client_version (unparseable)",
                        }),
                    },
                }
            }
        }

        if !policy.allow_prerelease_client {
            // If min_client_version is set and version is empty, CheckUnavailable was
            // already pushed above — avoid a duplicate entry.
            if data.defguard_client_version.is_empty() {
                if policy.min_client_version.is_none() {
                    all_failures.push(FailureReason::CheckUnavailable {
                        check: "defguard_client_version",
                    });
                }
            } else if data.defguard_client_version.contains('-') {
                all_failures.push(FailureReason::PrereleaseClientNotAllowed);
            }
        }

        // OS-level checks.
        let os_rules = DevicePostureOsRule::find_by_posture(pool, posture_id).await?;
        let matching_rule = os_type
            .as_ref()
            .and_then(|ot| os_rules.iter().find(|r| r.os_type == *ot));

        match matching_rule {
            None => all_failures.push(FailureReason::OsNotAllowed),
            Some(rule) => evaluate_os_rule(rule, data, &mut all_failures),
        }
    }

    if all_failures.is_empty() {
        info!("Posture check passed for device {}", pubkey);
        Ok(PostureResult::Pass)
    } else {
        info!(
            "Posture check failed for device {}, reasons: {:?}",
            pubkey, all_failures
        );
        Ok(PostureResult::Fail(all_failures))
    }
}
