use defguard_common::db::{Id, models::Device};
use defguard_proto::device_identity::{
    AttestationChallenge, AttestationChallengeSolution, AttestationIdentity, IdentityState,
};
use tonic::Status;

use crate::db::models::enrollment::Token;

pub fn generate_attestation_challenge(
    _enrollment: &Token,
    _device: &Device<Id>,
    _identity: &AttestationIdentity,
) -> Result<AttestationChallenge, Status> {
    Err(Status::unimplemented(
        "Device attestation challenge generation is not implemented",
    ))
}

pub fn verify_attestation(
    _enrollment: &Token,
    _device: &Device<Id>,
    _solution: &AttestationChallengeSolution,
) -> Result<IdentityState, Status> {
    Err(Status::unimplemented(
        "Device attestation verification is not implemented",
    ))
}
