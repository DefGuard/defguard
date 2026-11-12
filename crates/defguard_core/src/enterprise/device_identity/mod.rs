use defguard_common::db::{Id, models::Device};
use defguard_proto::{
    client_types::attestation_start_request::DeviceIdentity,
    device_identity::{AttestationRequest, AttestationResponse, IdentityState},
};
use tonic::Status;

use crate::db::models::enrollment::Token;

pub fn generate_attestation_challenge(
    _enrollment: &Token,
    _device: &Device<Id>,
    _identity: &DeviceIdentity,
) -> Result<AttestationRequest, Status> {
    Err(Status::unimplemented(
        "Device attestation challenge generation is not implemented",
    ))
}

pub fn verify_attestation(
    _enrollment: &Token,
    _device: &Device<Id>,
    _response: &AttestationResponse,
) -> Result<IdentityState, Status> {
    Err(Status::unimplemented(
        "Device attestation verification is not implemented",
    ))
}
