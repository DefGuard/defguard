mod attestation;
mod credential;
mod public;

use defguard_proto::device_identity::CredentialChallenge;
use openssl::error::ErrorStack;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::enterprise::db::models::device_identity::DeviceAttestationChallenge;

pub use attestation::verify_attestation;
pub use credential::create_challenge;

const MAX_PUBLIC_SIZE: usize = 1024;
const MAX_ATTEST_SIZE: usize = 1024;
const MAX_SIGNATURE_SIZE: usize = 80;
const MAX_CERTIFICATES: usize = 8;
const MAX_CERTIFICATE_SIZE: usize = 32 * 1024;
const MAX_CERTIFICATE_CHAIN_SIZE: usize = 128 * 1024;

#[derive(Debug, Error)]
pub enum TpmError {
    #[error("Invalid TPM public area: {0}")]
    InvalidPublic(&'static str),
    #[error("Unsupported TPM profile: {0}")]
    UnsupportedProfile(&'static str),
    #[error("Invalid TPM attestation: {0}")]
    InvalidEvidence(&'static str),
    #[error("TPM cryptography failed: {0}")]
    Crypto(#[from] ErrorStack),
}

type Result<T> = std::result::Result<T, TpmError>;

/// A TPM credential activation challenge prepared by Core.
pub struct PreparedAttestation {
    pub challenge: CredentialChallenge,
    /// Core keeps these expected values server-side to verify the response.
    pub verification: DeviceAttestationChallenge,
}

/// The device identity certified by the attestation key.
pub struct VerifiedIdentity {
    /// Encoded as SubjectPublicKeyInfo DER.
    pub public_key_der: Vec<u8>,
    pub ek_public: Vec<u8>,
    /// The client supplied this chain. Manufacturer trust requires separate validation.
    pub ek_certificate_chain_der: Vec<Vec<u8>>,
}

impl VerifiedIdentity {
    /// Computes the SHA-256 fingerprint of the identity SPKI DER.
    pub fn public_key_fingerprint(&self) -> [u8; 32] {
        Sha256::digest(&self.public_key_der).into()
    }
}

fn check_certificates(certificates: &[Vec<u8>]) -> Result<()> {
    if certificates.len() > MAX_CERTIFICATES
        || certificates
            .iter()
            .any(|certificate| certificate.len() > MAX_CERTIFICATE_SIZE)
        || certificates.iter().map(Vec::len).sum::<usize>() > MAX_CERTIFICATE_CHAIN_SIZE
    {
        return Err(TpmError::InvalidEvidence(
            "EK certificate chain exceeds size limits",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) mod test_support;
