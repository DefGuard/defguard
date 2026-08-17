pub mod gateway_conversions;

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
            pub mod posture {
                pub mod v2 {
                    tonic::include_proto!("defguard.enterprise.posture.v2");
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
    pub mod posture {
        pub use crate::generated::defguard::enterprise::posture::v2::*;
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
    db::{
        Id,
        models::{
            Device, User, WireguardNetwork,
            vpn_client_session::VpnClientMfaMethod,
            wireguard::{LocationMfaMode, ServiceLocationMode},
        },
    },
    gateway_types::{FirewallConfig, WireguardPeer},
};
use proxy::CoreError;
use serde::Serialize;
use tonic::Status;

use crate::gateway::Configuration;

// Client MFA methods
impl fmt::Display for MfaMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Totp => "TOTP",
            Self::Email => "Email",
            Self::Oidc => "OIDC",
            Self::Biometric => "Biometric",
            Self::MobileApprove => "MobileApprove",
            Self::Fido2 => "FIDO2",
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
            Self::Fido2 => serializer.serialize_unit_variant("MfaMethod", 5, "Fido2"),
        }
    }
}

impl From<MfaMethod> for VpnClientMfaMethod {
    fn from(val: MfaMethod) -> Self {
        match val {
            MfaMethod::Totp => Self::Totp,
            MfaMethod::Email => Self::Email,
            MfaMethod::Oidc => Self::Oidc,
            MfaMethod::Biometric => Self::Biometric,
            MfaMethod::MobileApprove => Self::MobileApprove,
            MfaMethod::Fido2 => Self::Fido2,
        }
    }
}

impl From<VpnClientMfaMethod> for MfaMethod {
    fn from(val: VpnClientMfaMethod) -> Self {
        match val {
            VpnClientMfaMethod::Totp => Self::Totp,
            VpnClientMfaMethod::Email => Self::Email,
            VpnClientMfaMethod::Oidc => Self::Oidc,
            VpnClientMfaMethod::Biometric => Self::Biometric,
            VpnClientMfaMethod::MobileApprove => Self::MobileApprove,
            VpnClientMfaMethod::Fido2 => Self::Fido2,
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
            LocationMfaMode::Disabled => Self::Disabled,
            LocationMfaMode::Internal => Self::Internal,
            LocationMfaMode::External => Self::External,
        }
    }
}

impl From<ServiceLocationMode> for client_types::ServiceLocationMode {
    fn from(value: ServiceLocationMode) -> Self {
        match value {
            ServiceLocationMode::Disabled => Self::Disabled,
            ServiceLocationMode::PreLogon => Self::Prelogon,
            ServiceLocationMode::AlwaysOn => Self::Alwayson,
        }
    }
}

impl Configuration {
    pub fn new(
        location: &WireguardNetwork<Id>,
        peers: Vec<WireguardPeer>,
        maybe_firewall_config: Option<FirewallConfig>,
    ) -> Self {
        Self {
            name: location.name.clone(),
            port: location.port.cast_unsigned(),
            private_key: location.prvkey.clone(),
            addresses: location.address().iter().map(ToString::to_string).collect(),
            peers: peers.into_iter().map(Into::into).collect(),
            firewall_config: maybe_firewall_config.map(Into::into),
            mtu: location.mtu.cast_unsigned(),
            fwmark: location.fwmark as u32,
        }
    }
}
