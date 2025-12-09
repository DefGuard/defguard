use std::fmt;

pub mod proxy {
    tonic::include_proto!("defguard.proxy");
}
pub mod gateway {
    tonic::include_proto!("gateway");
}
pub mod auth {
    tonic::include_proto!("auth");
}
pub mod worker {
    tonic::include_proto!("worker");
}
pub mod enterprise {
    pub mod firewall {
        tonic::include_proto!("enterprise.firewall");
    }
}

use proxy::{CoreError, MfaMethod};
use serde::Serialize;
use tonic::Status;

// Client MFA methods
impl fmt::Display for MfaMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Totp => "TOTP",
                Self::Email => "Email",
                Self::Oidc => "OIDC",
                Self::Biometric => "Biometric",
                Self::MobileApprove => "MobileApprove",
            }
        )
    }
}

impl Serialize for MfaMethod {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            Self::Totp => serializer.serialize_unit_variant("MfaMethod", 0, "Totp"),
            Self::Email => serializer.serialize_unit_variant("MfaMethod", 1, "Email"),
            Self::Oidc => serializer.serialize_unit_variant("MfaMethod", 2, "Oidc"),
            Self::Biometric => serializer.serialize_unit_variant("MfaMethod", 3, "Biometric"),
            Self::MobileApprove => {
                serializer.serialize_unit_variant("MfaMethod", 4, "MobileApprove")
            }
        }
    }
}

impl From<Status> for CoreError {
    fn from(status: Status) -> Self {
        Self {
            status_code: status.code().into(),
            message: status.message().into(),
        }
    }
}
