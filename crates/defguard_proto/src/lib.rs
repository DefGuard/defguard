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

use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceConfig, User,
            vpn_client_session::VpnClientMfaMethod,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
};
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

impl Into<VpnClientMfaMethod> for MfaMethod {
    fn into(self) -> VpnClientMfaMethod {
        match self {
            Self::Totp => VpnClientMfaMethod::Totp,
            Self::Email => VpnClientMfaMethod::Email,
            Self::Oidc => VpnClientMfaMethod::Oidc,
            Self::Biometric => VpnClientMfaMethod::Biometric,
            Self::MobileApprove => VpnClientMfaMethod::MobileApprove,
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

impl From<DeviceConfig> for proxy::DeviceConfig {
    fn from(config: DeviceConfig) -> Self {
        // DEPRECATED(1.5): superseeded by location_mfa_mode
        let mfa_enabled = config.location_mfa_mode == LocationMfaMode::Internal;
        Self {
            network_id: config.network_id,
            network_name: config.network_name,
            config: config.config,
            endpoint: config.endpoint,
            assigned_ip: config.address.as_csv(),
            pubkey: config.pubkey,
            allowed_ips: config.allowed_ips.as_csv(),
            dns: config.dns,
            keepalive_interval: config.keepalive_interval,
            #[allow(deprecated)]
            mfa_enabled,
            location_mfa_mode: Some(
                <LocationMfaMode as Into<proxy::LocationMfaMode>>::into(config.location_mfa_mode)
                    .into(),
            ),
            service_location_mode: Some(
                <ServiceLocationMode as Into<proxy::ServiceLocationMode>>::into(
                    config.service_location_mode,
                )
                .into(),
            ),
        }
    }
}

impl From<Device<Id>> for proxy::Device {
    fn from(device: Device<Id>) -> Self {
        Self {
            id: device.id,
            name: device.name,
            pubkey: device.wireguard_pubkey,
            user_id: device.user_id,
            created_at: device.created.and_utc().timestamp(),
        }
    }
}

impl From<User<Id>> for proxy::AdminInfo {
    fn from(admin: User<Id>) -> Self {
        Self {
            name: format!("{} {}", admin.first_name, admin.last_name),
            phone_number: admin.phone,
            email: admin.email,
        }
    }
}

impl From<LocationMfaMode> for proxy::LocationMfaMode {
    fn from(value: LocationMfaMode) -> Self {
        match value {
            LocationMfaMode::Disabled => proxy::LocationMfaMode::Disabled,
            LocationMfaMode::Internal => proxy::LocationMfaMode::Internal,
            LocationMfaMode::External => proxy::LocationMfaMode::External,
        }
    }
}

impl From<ServiceLocationMode> for proxy::ServiceLocationMode {
    fn from(value: ServiceLocationMode) -> Self {
        match value {
            ServiceLocationMode::Disabled => proxy::ServiceLocationMode::Disabled,
            ServiceLocationMode::PreLogon => proxy::ServiceLocationMode::Prelogon,
            ServiceLocationMode::AlwaysOn => proxy::ServiceLocationMode::Alwayson,
        }
    }
}
