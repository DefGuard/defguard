use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[sqlx(type_name = "device_identity_backend", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DeviceIdentityBackend {
    Tpm,
    AppleAppAttest,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize, Type)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[sqlx(type_name = "device_attestation_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DeviceAttestationStatus {
    #[default]
    NotAttested,
    Attested,
    Unrecognized,
    AttestedManually,
    AttestationFailed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum DevicePossessionState {
    Pending { challenge_id: String, data: Vec<u8> },
    Verified { key_fingerprint: [u8; 32] },
}

#[cfg(test)]
mod tests;
