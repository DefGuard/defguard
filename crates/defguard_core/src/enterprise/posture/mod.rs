use std::fmt;

use defguard_proto::enterprise::posture::{
    DevicePostureCheckRequest, DevicePostureData,
    bool_check::Result as BoolResult,
    string_check::Result as StringResult,
    UnavailableReason,
};
use semver::Version;
use sqlx::PgPool;
use thiserror::Error;

use crate::enterprise::{
    db::models::device_posture::{DevicePosture, DevicePostureLocation, DevicePostureOsRule, OsType},
    is_enterprise_license_active,
};

#[derive(Debug, Error)]
pub enum PostureCheckError {
    #[error("No active enterprise license found")]
    NoActiveEnterpriseLicense,
    #[error(transparent)]
    DbError(#[from] sqlx::Error),
}

#[derive(Debug)]
pub enum FailureReason {
    MissingPostureData,
    OsNotAllowed,
    ClientVersionTooOld { required: String, actual: String },
    PrereleaseClientNotAllowed,
    OsVersionTooOld { required: String, actual: String },
    KernelVersionTooOld { required: String, actual: String },
    DiskEncryptionRequired,
    AntivirusRequired,
    AdDomainRequired,
    SecurityUpdateRequired,
    DeviceIntegrityRequired,
    /// A required check could not be evaluated (InsufficientPermissions or DetectionFailed).
    CheckUnavailable { check: &'static str },
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPostureData => write!(f, "posture data is missing"),
            Self::OsNotAllowed => write!(f, "operating system is not allowed"),
            Self::ClientVersionTooOld { required, actual } => {
                write!(f, "client version {actual} is too old (required: {required})")
            }
            Self::PrereleaseClientNotAllowed => {
                write!(f, "pre-release client versions are not allowed")
            }
            Self::OsVersionTooOld { required, actual } => {
                write!(f, "OS version {actual} is too old (required: {required})")
            }
            Self::KernelVersionTooOld { required, actual } => {
                write!(f, "kernel version {actual} is too old (required: {required})")
            }
            Self::DiskEncryptionRequired => write!(f, "disk encryption is required"),
            Self::AntivirusRequired => write!(f, "antivirus is required"),
            Self::AdDomainRequired => write!(f, "Active Directory domain join is required"),
            Self::SecurityUpdateRequired => write!(f, "Windows security updates must be current"),
            Self::DeviceIntegrityRequired => write!(f, "device integrity check failed"),
            Self::CheckUnavailable { check } => {
                write!(f, "required check '{check}' could not be evaluated")
            }
        }
    }
}

pub enum PostureResult {
    Pass,
    Fail(Vec<FailureReason>),
}

/// Parses a version string leniently, accepting `MAJOR`, `MAJOR.MINOR`, or full
/// semver by zero-padding missing components. Strips a leading `v` prefix.
fn parse_version_lenient(s: &str) -> Option<Version> {
    let s = s.strip_prefix('v').unwrap_or(s);
    if let Ok(v) = Version::parse(s) {
        return Some(v);
    }
    let parts: Vec<&str> = s.splitn(3, '.').collect();
    let padded = match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => return None,
    };
    Version::parse(&padded).ok()
}

/// Returns `Some(true)` when `actual >= required`, `Some(false)` when it is older,
/// and `None` when either string cannot be parsed.
fn version_meets_minimum(required: &str, actual: &str) -> Option<bool> {
    let req = parse_version_lenient(required)?;
    let act = parse_version_lenient(actual)?;
    Some(act >= req)
}

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
        Some(BoolResult::Unavailable(code)) => {
            match UnavailableReason::try_from(*code) {
                Ok(UnavailableReason::NotApplicable) => Ok(true),
                _ => Err(check_name),
            }
        }
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
        Some(StringResult::Unavailable(code)) => {
            match UnavailableReason::try_from(*code) {
                Ok(UnavailableReason::NotApplicable) => Ok(None),
                _ => Err(check_name),
            }
        }
        None => Err(check_name),
    }
}

fn evaluate_os_rule(
    rule: &DevicePostureOsRule<defguard_common::db::Id>,
    data: &DevicePostureData,
    failures: &mut Vec<FailureReason>,
) {
    // min_os_version
    if let Some(ref required) = rule.min_os_version {
        match resolve_string_check(data.os_version.as_ref(), "os_version") {
            Ok(Some(actual)) => match version_meets_minimum(required, &actual) {
                Some(true) => {}
                Some(false) => failures.push(FailureReason::OsVersionTooOld {
                    required: required.clone(),
                    actual,
                }),
                None => failures.push(FailureReason::CheckUnavailable {
                    check: "os_version (unparseable)",
                }),
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
    if let Some(ref required) = rule.min_kernel_version {
        match resolve_string_check(data.linux_kernel_version.as_ref(), "linux_kernel_version") {
            Ok(Some(actual)) => match version_meets_minimum(required, &actual) {
                Some(true) => {}
                Some(false) => failures.push(FailureReason::KernelVersionTooOld {
                    required: required.clone(),
                    actual,
                }),
                None => failures.push(FailureReason::CheckUnavailable {
                    check: "linux_kernel_version (unparseable)",
                }),
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

/// Evaluates posture signals against all policies assigned to the location.
///
/// Returns [`PostureResult::Pass`] when no postures are assigned or all pass.
/// Returns [`PostureResult::Fail`] with accumulated [`FailureReason`]s otherwise.
pub async fn validate_posture(
    pool: &PgPool,
    request: &DevicePostureCheckRequest,
) -> Result<PostureResult, PostureCheckError> {
    debug!(
        "Performing posture check for device {}: {:?}",
        request.pubkey, request.device_posture_data
    );

    if !is_enterprise_license_active() {
        warn!(
            "No active enterprise license - posture check aborted for device {}",
            request.pubkey
        );
        return Err(PostureCheckError::NoActiveEnterpriseLicense);
    }

    let data = match request.device_posture_data.as_ref() {
        Some(d) => d,
        None => {
            info!(
                "Missing posture data - posture check failed for device {}",
                request.pubkey
            );
            return Ok(PostureResult::Fail(vec![FailureReason::MissingPostureData]));
        }
    };

    let posture_ids = DevicePostureLocation::find_by_location(pool, request.location_id).await?;

    if posture_ids.is_empty() {
        debug!(
            "No posture policies assigned to location {} — passing device {}",
            request.location_id, request.pubkey
        );
        return Ok(PostureResult::Pass);
    }

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
                match version_meets_minimum(required, actual) {
                    Some(true) => {}
                    Some(false) => all_failures.push(FailureReason::ClientVersionTooOld {
                        required: required.clone(),
                        actual: actual.clone(),
                    }),
                    None => all_failures.push(FailureReason::CheckUnavailable {
                        check: "defguard_client_version (unparseable)",
                    }),
                }
            }
        }

        if !policy.allow_prerelease_client && !data.defguard_client_version.is_empty() {
            if data.defguard_client_version.contains('-') {
                all_failures.push(FailureReason::PrereleaseClientNotAllowed);
            }
        }

        // OS-level checks.
        let os_rules = DevicePostureOsRule::find_by_posture(pool, posture_id).await?;

        let matching_rule = os_type.as_ref().and_then(|ot| {
            os_rules
                .iter()
                .find(|r| std::mem::discriminant(&r.os_type) == std::mem::discriminant(ot))
        });

        match matching_rule {
            None => all_failures.push(FailureReason::OsNotAllowed),
            Some(rule) => evaluate_os_rule(rule, data, &mut all_failures),
        }
    }

    if all_failures.is_empty() {
        info!("Posture check passed for device {}", request.pubkey);
        Ok(PostureResult::Pass)
    } else {
        Ok(PostureResult::Fail(all_failures))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_meets_minimum_comparisons() {
        assert_eq!(version_meets_minimum("1.6.0", "1.6.0"), Some(true));
        assert_eq!(version_meets_minimum("1.6.0", "1.7.0"), Some(true));
        assert_eq!(version_meets_minimum("1.6.0", "1.5.9"), Some(false));
        assert_eq!(version_meets_minimum("11", "11.0.0"), Some(true));
        assert_eq!(version_meets_minimum("14.5", "14.4.1"), Some(false));
        assert_eq!(version_meets_minimum("14.5", "14.5.0"), Some(true));
    }

    #[test]
    fn parse_version_lenient_handles_short_forms() {
        assert!(parse_version_lenient("11").is_some());
        assert!(parse_version_lenient("14.5").is_some());
        assert!(parse_version_lenient("1.6.0").is_some());
        assert!(parse_version_lenient("v1.6.0").is_some());
        assert!(parse_version_lenient("not-a-version").is_none());
    }
}
