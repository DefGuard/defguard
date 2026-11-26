use defguard_proto::device_identity::{SignatureAlgorithm, TpmAttestationResponse};
use openssl::{ecdsa::EcdsaSig, hash::MessageDigest, memcmp, sign::Verifier};
use sha2::{Digest, Sha256};

use super::{
    MAX_ATTEST_SIZE, MAX_SIGNATURE_SIZE, Result, TpmError, VerifiedIdentity, check_certificates,
    public::{Reader, SHA256, endorsement_key, name, signing_key},
};
use crate::enterprise::{
    db::models::device_identity::DeviceAttestationChallenge, device_identity::qualifying_data,
};

const TPM_GENERATED_VALUE: u32 = 0xff54_4347;
const TPM_ST_ATTEST_CERTIFY: u16 = 0x8017;

fn check_name(bytes: &[u8]) -> Result<()> {
    if bytes.len() != 34 || !bytes.starts_with(&SHA256.to_be_bytes()) {
        return Err(TpmError::InvalidEvidence("invalid SHA-256 TPM Name"));
    }
    Ok(())
}

fn check_certify(attest: &[u8], expected_data: &[u8; 32], expected_name: &[u8; 34]) -> Result<()> {
    let mut reader = Reader::new(attest);
    if reader.u32()? != TPM_GENERATED_VALUE || reader.u16()? != TPM_ST_ATTEST_CERTIFY {
        return Err(TpmError::InvalidEvidence(
            "expected TPM-generated Certify attestation",
        ));
    }
    check_name(reader.sized()?)?;
    if reader.sized()? != expected_data {
        return Err(TpmError::InvalidEvidence(
            "Certify qualifying data mismatch",
        ));
    }
    let clock = reader.take(17)?;
    if clock[16] > 1 {
        return Err(TpmError::InvalidEvidence("invalid TPM clock safe flag"));
    }
    reader.take(8)?;
    if reader.sized()? != expected_name {
        return Err(TpmError::InvalidEvidence(
            "certified identity key Name mismatch",
        ));
    }
    check_name(reader.sized()?)?;
    reader.finish()
}

/// Verifies activation and key certification against Core's saved challenge state and exact data.
/// Returns the identity SPKI and untrusted EK provenance, without validating certificate chains.
pub fn verify_attestation(
    expected: &DeviceAttestationChallenge,
    data: &[u8],
    response: &TpmAttestationResponse,
) -> Result<VerifiedIdentity> {
    let DeviceAttestationChallenge::Tpm {
        ak_name,
        ek_fingerprint,
        activation_secret_hash,
    } = expected
    else {
        return Err(TpmError::InvalidEvidence("expected TPM challenge state"));
    };
    if response.activation_secret.len() != 32
        || !memcmp::eq(
            &Sha256::digest(&response.activation_secret),
            activation_secret_hash,
        )
    {
        return Err(TpmError::InvalidEvidence(
            "credential activation secret mismatch",
        ));
    }
    let evidence = response
        .evidence
        .as_ref()
        .ok_or(TpmError::InvalidEvidence("missing TPM evidence"))?;
    check_certificates(&evidence.ek_certificate_chain_der)?;
    if evidence.attest.is_empty() || evidence.attest.len() > MAX_ATTEST_SIZE {
        return Err(TpmError::InvalidEvidence(
            "attestation empty or exceeds size limits",
        ));
    }
    endorsement_key(&evidence.ek_public)?;
    let ak = signing_key(&evidence.ak_public, true)?;
    let identity = signing_key(&evidence.identity_public, false)?;
    if Sha256::digest(&evidence.ek_public).as_slice() != ek_fingerprint
        || name(&evidence.ak_public).as_slice() != ak_name
    {
        return Err(TpmError::InvalidEvidence("EK or AK changed since Start"));
    }
    let signature = evidence
        .signature
        .as_ref()
        .ok_or(TpmError::InvalidEvidence("missing Certify signature"))?;
    if signature.algorithm != i32::from(SignatureAlgorithm::EcdsaSha256)
        || signature.signature.is_empty()
        || signature.signature.len() > MAX_SIGNATURE_SIZE
    {
        return Err(TpmError::InvalidEvidence(
            "expected DER ECDSA-SHA256 signature",
        ));
    }
    let parsed = EcdsaSig::from_der(&signature.signature)
        .map_err(|_| TpmError::InvalidEvidence("invalid DER ECDSA signature"))?;
    if parsed.to_der()? != signature.signature {
        return Err(TpmError::InvalidEvidence(
            "noncanonical or trailing DER signature data",
        ));
    }
    let mut verifier = Verifier::new(MessageDigest::sha256(), &ak)?;
    verifier.update(&evidence.attest)?;
    if !verifier
        .verify(&signature.signature)
        .map_err(|_| TpmError::InvalidEvidence("invalid Certify signature"))?
    {
        return Err(TpmError::InvalidEvidence("invalid Certify signature"));
    }
    check_certify(
        &evidence.attest,
        &qualifying_data(data),
        &name(&evidence.identity_public),
    )?;
    Ok(VerifiedIdentity {
        public_key_der: identity.public_key_to_der()?,
        ek_public: evidence.ek_public.clone(),
        ek_certificate_chain_der: evidence.ek_certificate_chain_der.clone(),
    })
}
