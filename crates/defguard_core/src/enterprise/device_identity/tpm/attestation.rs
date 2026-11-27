use defguard_proto::device_identity::{SignatureAlgorithm, TpmAttestationResponse};
use openssl::{ecdsa::EcdsaSig, hash::MessageDigest, memcmp, sign::Verifier};
use sha2::{Digest, Sha256};
use tpm2_protocol::{
    TpmUnmarshal,
    data::{TpmAlgId, TpmsAttest, TpmuAttest},
};

use super::{
    MAX_ATTEST_SIZE, MAX_SIGNATURE_SIZE, Result, TpmError, VerifiedIdentity, check_certificates,
    public::{endorsement_key, name, signing_key},
};
use crate::enterprise::{
    db::models::device_identity::DeviceAttestationChallenge, device_identity::qualifying_data,
};

fn check_name(bytes: &[u8]) -> Result<()> {
    if bytes.len() != 34 || !bytes.starts_with(&TpmAlgId::Sha256.value().to_be_bytes()) {
        return Err(TpmError::InvalidEvidence("invalid SHA-256 TPM Name"));
    }
    Ok(())
}

/// Checks that the Certify payload binds the challenge digest to the expected identity key's TPM Name.
/// The caller must verify the signature over the original payload bytes separately.
fn check_certify(attest: &[u8], expected_data: &[u8; 32], expected_name: &[u8; 34]) -> Result<()> {
    // The decoder validates the TPM header and field encodings. Require the whole
    // signed payload to be a single Certify structure.
    let (attest, rest) = TpmsAttest::unmarshal(attest)
        .map_err(|_| TpmError::InvalidEvidence("malformed TPM attestation"))?;
    if !rest.is_empty() {
        return Err(TpmError::InvalidEvidence("trailing TPM attestation data"));
    }
    let TpmuAttest::Certify(certify) = attest.attested else {
        return Err(TpmError::InvalidEvidence(
            "expected TPM Certify attestation",
        ));
    };

    // The caller authenticates the payload with the AK bound to the activation challenge.
    // The qualified signer Name is checked here only for format.
    check_name(attest.qualified_signer.as_ref())?;

    // The TPM copies the caller's qualifying data into `extraData`. It must match the
    // digest Core computes from the original challenge data.
    if attest.extra_data.as_ref() != expected_data {
        return Err(TpmError::InvalidEvidence(
            "Certify qualifying data mismatch",
        ));
    }

    // The Name binds the supplied public area. The Qualified Name also depends on the
    // parent chain and is checked only for format here.
    if certify.name.as_ref() != expected_name {
        return Err(TpmError::InvalidEvidence(
            "certified identity key Name mismatch",
        ));
    }
    check_name(certify.qualified_name.as_ref())
}

/// Verifies activation and key certification against Core's saved challenge state and exact data.
/// EK certificate trust requires separate validation.
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
    // Core retained only the digest. Matching it in constant time checks that the
    // response contains the secret wrapped for the original EK and AK.
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

    // Require a supported profile for each key role. Only the restricted AK
    // verifies Certify. The identity key is the one being certified.
    endorsement_key(&evidence.ek_public)?;
    let ak = signing_key(&evidence.ak_public, true)?;
    let identity = signing_key(&evidence.identity_public, false)?;

    // Profile checks alone would also accept a different EK or AK of the same type.
    // Compare their original public areas against the values bound to Start.
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
    // Round-trip the ECDSA pair through DER to require a canonical encoding with no
    // trailing bytes before handing the signature to the verifier.
    let parsed = EcdsaSig::from_der(&signature.signature)
        .map_err(|_| TpmError::InvalidEvidence("invalid DER ECDSA signature"))?;
    if parsed.to_der()? != signature.signature {
        return Err(TpmError::InvalidEvidence(
            "noncanonical or trailing DER signature data",
        ));
    }
    // The AK signed the original TPM attestation bytes. Verify those exact bytes
    // before interpreting the Certify fields.
    let mut verifier = Verifier::new(MessageDigest::sha256(), &ak)?;
    verifier.update(&evidence.attest)?;
    if !verifier
        .verify(&signature.signature)
        .map_err(|_| TpmError::InvalidEvidence("invalid Certify signature"))?
    {
        return Err(TpmError::InvalidEvidence("invalid Certify signature"));
    }

    // A valid AK signature still needs to cover the expected enrollment context
    // and the Name of the identity key Core is about to register.
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
