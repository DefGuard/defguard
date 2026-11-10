use serde::{Deserialize, Serialize};
use sqlx::Type;

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

#[cfg(test)]
mod tests;
