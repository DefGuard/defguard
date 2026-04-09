use std::fmt;

mod generated {
    pub mod defguard {
        pub mod proxy {
            pub mod v2 {
                tonic::include_proto!("defguard.proxy.v2");
            }
        }

        pub mod gateway {
            pub mod v2 {
                tonic::include_proto!("defguard.gateway.v2");
            }
        }

        pub mod worker {
            pub mod v1 {
                tonic::include_proto!("defguard.worker.v1");
            }
        }

        pub mod enterprise {
            pub mod firewall {
                pub mod v2 {
                    tonic::include_proto!("defguard.enterprise.firewall.v2");
                }
            }
        }

        pub mod client_types {
            tonic::include_proto!("defguard.client_types");
        }

        pub mod common {
            pub mod v2 {
                tonic::include_proto!("defguard.common.v2");
            }
        }
    }
}

pub mod proxy {
    pub use crate::generated::defguard::proxy::v2::*;
}

pub mod gateway {
    pub use crate::generated::defguard::gateway::v2::*;
}

pub mod worker {
    pub use crate::generated::defguard::worker::v1::*;
}

pub mod enterprise {
    pub mod firewall {
        pub use crate::generated::defguard::enterprise::firewall::v2::*;
    }
}

pub mod client_types {
    pub use crate::generated::defguard::client_types::*;
}

pub mod common {
    pub use crate::generated::defguard::common::v2::*;
}

use client_types::MfaMethod;
use defguard_common::{
    csv::AsCsv,
    db::{
        Id,
        models::{
            Device, DeviceConfig, User, WireguardNetwork,
            vpn_client_session::VpnClientMfaMethod,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
};
use proxy::CoreError;
use serde::Serialize;
use tonic::Status;

use crate::{
    enterprise::firewall::FirewallConfig,
    gateway::{Configuration, Peer},
};

// Client MFA methods
impl fmt::Display for MfaMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Totp => "TOTP",
            Self::Email => "Email",
            Self::Oidc => "OIDC",
            Self::Biometric => "Biometric",
            Self::MobileApprove => "MobileApprove",
        })
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

impl From<MfaMethod> for VpnClientMfaMethod {
    fn from(val: MfaMethod) -> Self {
        match val {
            MfaMethod::Totp => VpnClientMfaMethod::Totp,
            MfaMethod::Email => VpnClientMfaMethod::Email,
            MfaMethod::Oidc => VpnClientMfaMethod::Oidc,
            MfaMethod::Biometric => VpnClientMfaMethod::Biometric,
            MfaMethod::MobileApprove => VpnClientMfaMethod::MobileApprove,
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

impl From<DeviceConfig> for client_types::DeviceConfig {
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
                <LocationMfaMode as Into<client_types::LocationMfaMode>>::into(
                    config.location_mfa_mode,
                )
                .into(),
            ),
            service_location_mode: Some(
                <ServiceLocationMode as Into<client_types::ServiceLocationMode>>::into(
                    config.service_location_mode,
                )
                .into(),
            ),
        }
    }
}

impl From<Device<Id>> for client_types::Device {
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

impl From<User<Id>> for client_types::AdminInfo {
    fn from(admin: User<Id>) -> Self {
        Self {
            name: format!("{} {}", admin.first_name, admin.last_name),
            phone_number: admin.phone,
            email: admin.email,
        }
    }
}

impl From<LocationMfaMode> for client_types::LocationMfaMode {
    fn from(value: LocationMfaMode) -> Self {
        match value {
            LocationMfaMode::Disabled => client_types::LocationMfaMode::Disabled,
            LocationMfaMode::Internal => client_types::LocationMfaMode::Internal,
            LocationMfaMode::External => client_types::LocationMfaMode::External,
        }
    }
}

impl From<ServiceLocationMode> for client_types::ServiceLocationMode {
    fn from(value: ServiceLocationMode) -> Self {
        match value {
            ServiceLocationMode::Disabled => client_types::ServiceLocationMode::Disabled,
            ServiceLocationMode::PreLogon => client_types::ServiceLocationMode::Prelogon,
            ServiceLocationMode::AlwaysOn => client_types::ServiceLocationMode::Alwayson,
        }
    }
}

impl Configuration {
    pub fn new(
        location: &WireguardNetwork<Id>,
        peers: Vec<Peer>,
        maybe_firewall_config: Option<FirewallConfig>,
    ) -> Self {
        Self {
            name: location.name.clone(),
            port: location.port.cast_unsigned(),
            private_key: location.prvkey.clone(),
            addresses: location.address().iter().map(ToString::to_string).collect(),
            peers,
            firewall_config: maybe_firewall_config,
            mtu: location.mtu.cast_unsigned(),
            fwmark: location.fwmark as u32,
        }
    }
}
