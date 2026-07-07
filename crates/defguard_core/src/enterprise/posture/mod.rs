use std::fmt;

use thiserror::Error;

mod evaluation;
mod version;
pub mod version_list;

#[cfg(test)]
mod tests;

pub(crate) use evaluation::validate_posture;

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
    ClientVersionTooOld {
        required: String,
        actual: String,
    },
    PrereleaseClientNotAllowed,
    OsVersionTooOld {
        required: i32,
        actual: String,
    },
    KernelVersionTooOld {
        required: i32,
        actual: String,
    },
    DiskEncryptionRequired,
    AntivirusRequired,
    AdDomainRequired,
    SecurityUpdateTooOld {
        required_max_age_days: i32,
        actual_age_days: i32,
    },
    AndroidSecurityPatchTooOld {
        required_max_age_days: i32,
        actual_age_days: i32,
    },
    DeviceIntegrityRequired,
    /// A required check could not be evaluated (InsufficientPermissions or DetectionFailed).
    CheckUnavailable {
        check: &'static str,
    },
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPostureData => write!(f, "posture data is missing"),
            Self::OsNotAllowed => write!(f, "operating system is not allowed"),
            Self::ClientVersionTooOld { required, actual } => {
                write!(
                    f,
                    "client version {actual} is too old (required: {required})"
                )
            }
            Self::PrereleaseClientNotAllowed => {
                write!(f, "pre-release client versions are not allowed")
            }
            Self::OsVersionTooOld { required, actual } => {
                write!(f, "OS version {actual} is too old (required: {required})")
            }
            Self::KernelVersionTooOld { required, actual } => {
                write!(
                    f,
                    "kernel version {actual} is too old (required: {required})"
                )
            }
            Self::DiskEncryptionRequired => write!(f, "disk encryption is required"),
            Self::AntivirusRequired => write!(f, "antivirus is required"),
            Self::AdDomainRequired => write!(f, "Active Directory domain join is required"),
            Self::SecurityUpdateTooOld {
                required_max_age_days,
                actual_age_days,
            } => write!(
                f,
                "last Windows security update is {actual_age_days} days old \
                 (max allowed: {required_max_age_days})"
            ),
            Self::AndroidSecurityPatchTooOld {
                required_max_age_days,
                actual_age_days,
            } => write!(
                f,
                "Android security patch is {actual_age_days} days old \
                 (max allowed: {required_max_age_days})"
            ),
            Self::DeviceIntegrityRequired => write!(f, "device integrity check failed"),
            Self::CheckUnavailable { check } => {
                write!(f, "required check '{check}' could not be evaluated")
            }
        }
    }
}

pub(crate) enum PostureResult {
    Pass,
    Fail(Vec<FailureReason>),
}
