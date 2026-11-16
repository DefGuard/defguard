use defguard_proto::device_identity::{SignatureAlgorithm, TpmAttestationIdentity};
use p256::{
    ecdsa::{
        Signature, VerifyingKey,
        signature::{Signer, Verifier},
    },
    pkcs8::{DecodePublicKey, EncodePublicKey},
};
use sha2_10::{Digest, Sha256};

use super::{
    create_challenge,
    test_support::{Fixture, activate, name},
    verify_attestation,
};
use crate::enterprise::db::models::device_identity::DeviceAttestationChallenge;

const DATA: &[u8] = b"defguard attestation context\0\xff";

#[test]
fn test_challenge_rejects_empty_public_areas() {
    assert!(create_challenge(&TpmAttestationIdentity::default()).is_err());
}

#[test]
fn test_ecc_and_rsa_credentials_activate_and_certify_the_identity_key() {
    for rsa in [false, true] {
        let fixture = Fixture::new(rsa);
        let prepared = create_challenge(&fixture.metadata).unwrap();
        let secret = activate(
            &fixture,
            &prepared.challenge,
            &name(&fixture.metadata.ak_public),
        );
        let DeviceAttestationChallenge::Tpm {
            ak_name,
            ek_fingerprint,
            activation_secret_hash,
        } = &prepared.verification
        else {
            panic!("expected TPM state")
        };
        assert_eq!(ak_name, &name(&fixture.metadata.ak_public));
        assert_eq!(
            ek_fingerprint.as_slice(),
            Sha256::digest(&fixture.metadata.ek_public).as_slice()
        );
        assert_eq!(
            activation_secret_hash.as_slice(),
            Sha256::digest(&secret).as_slice()
        );
        assert_eq!(secret.len(), 32);
        let response = fixture.response(secret, DATA);
        let verified = verify_attestation(&prepared.verification, DATA, &response).unwrap();
        assert_eq!(verified.ek_public, fixture.metadata.ek_public);
        assert!(verified.ek_certificate_chain_der.is_empty());
        assert_eq!(
            verified.public_key_fingerprint().as_slice(),
            Sha256::digest(&verified.public_key_der).as_slice()
        );
        let public = VerifyingKey::from_public_key_der(&verified.public_key_der).unwrap();
        assert_eq!(public, *fixture.identity.verifying_key());
        assert_eq!(
            verified.public_key_der,
            fixture
                .identity
                .verifying_key()
                .to_public_key_der()
                .unwrap()
                .as_bytes()
        );
        let signature: Signature = fixture.identity.sign(b"possession challenge");
        public.verify(b"possession challenge", &signature).unwrap();
    }
}

#[test]
fn test_each_challenge_uses_a_new_activation_secret_and_wrapping() {
    let fixture = Fixture::new(false);
    let first = create_challenge(&fixture.metadata).unwrap();
    let second = create_challenge(&fixture.metadata).unwrap();
    let first_secret = activate(
        &fixture,
        &first.challenge,
        &name(&fixture.metadata.ak_public),
    );
    let second_secret = activate(
        &fixture,
        &second.challenge,
        &name(&fixture.metadata.ak_public),
    );
    assert_ne!(first_secret, second_secret);
    assert_ne!(
        first.challenge.encrypted_secret,
        second.challenge.encrypted_secret
    );
    assert_ne!(
        first.challenge.credential_blob,
        second.challenge.credential_blob
    );
}

#[test]
fn test_certificate_trust_is_not_required_but_provenance_is_retained() {
    let mut fixture = Fixture::new(false);
    fixture.metadata.ek_certificate_chain_der = vec![vec![0xff, 0, 1]];
    let prepared = create_challenge(&fixture.metadata).unwrap();
    let secret = activate(
        &fixture,
        &prepared.challenge,
        &name(&fixture.metadata.ak_public),
    );
    let response = fixture.response(secret, DATA);
    let verified = verify_attestation(&prepared.verification, DATA, &response).unwrap();
    assert_eq!(
        verified.ek_certificate_chain_der,
        fixture.metadata.ek_certificate_chain_der
    );
}

#[test]
fn test_verification_rejects_wrong_secret_context_keys_and_signature() {
    let fixture = Fixture::new(false);
    let prepared = create_challenge(&fixture.metadata).unwrap();
    let secret = activate(
        &fixture,
        &prepared.challenge,
        &name(&fixture.metadata.ak_public),
    );
    let response = fixture.response(secret, DATA);
    assert!(
        verify_attestation(
            &prepared.verification,
            b"another enrollment context",
            &response
        )
        .is_err()
    );
    let mut bad = response.clone();
    bad.activation_secret[0] ^= 1;
    assert!(verify_attestation(&prepared.verification, DATA, &bad).is_err());
    let mut wrong_signer = response.clone();
    let evidence = wrong_signer.evidence.as_mut().unwrap();
    let signature: Signature = fixture.identity.sign(&evidence.attest);
    evidence.signature.as_mut().unwrap().signature = signature.to_der().as_bytes().to_vec();
    assert!(verify_attestation(&prepared.verification, DATA, &wrong_signer).is_err());
    let mut bad = response.clone();
    bad.evidence = None;
    assert!(verify_attestation(&prepared.verification, DATA, &bad).is_err());
    for mutate in [
        "signature",
        "signature_algorithm",
        "signature_trailing",
        "missing_signature",
        "identity",
    ] {
        let mut bad = response.clone();
        let evidence = bad.evidence.as_mut().unwrap();
        match mutate {
            "signature" => evidence.signature.as_mut().unwrap().signature[10] ^= 1,
            "signature_algorithm" => {
                evidence.signature.as_mut().unwrap().algorithm =
                    SignatureAlgorithm::RsaSsaSha256.into()
            }
            "signature_trailing" => evidence.signature.as_mut().unwrap().signature.push(0),
            "missing_signature" => evidence.signature = None,
            "identity" => {
                evidence.identity_public[22..].copy_from_slice(&fixture.metadata.ak_public[22..])
            }
            _ => unreachable!(),
        }
        assert!(
            verify_attestation(&prepared.verification, DATA, &bad).is_err(),
            "{mutate}"
        );
    }
    for field in ["ek", "ak", "secret"] {
        let mut expected = prepared.verification.clone();
        let DeviceAttestationChallenge::Tpm {
            ak_name,
            ek_fingerprint,
            activation_secret_hash,
        } = &mut expected
        else {
            unreachable!()
        };
        match field {
            "ek" => ek_fingerprint[0] ^= 1,
            "ak" => ak_name[5] ^= 1,
            "secret" => activation_secret_hash[0] ^= 1,
            _ => unreachable!(),
        }
        assert!(
            verify_attestation(&expected, DATA, &response).is_err(),
            "{field}"
        );
    }
    let wrong_backend = DeviceAttestationChallenge::AppleAppAttest {
        key_id: "apple-key".into(),
    };
    assert!(verify_attestation(&wrong_backend, DATA, &response).is_err());
}

#[test]
fn test_valid_ak_signatures_do_not_bypass_certify_structure_checks() {
    let fixture = Fixture::new(false);
    let prepared = create_challenge(&fixture.metadata).unwrap();
    let secret = activate(
        &fixture,
        &prepared.challenge,
        &name(&fixture.metadata.ak_public),
    );
    let response = fixture.response(secret, DATA);
    for (field, offset, value) in [
        ("magic", 0, 0),
        ("quote_instead_of_certify", 5, 0x18),
        ("qualified_signer_algorithm", 9, 0x0c),
        ("extra_data", 44, 0xff),
        ("clock_safe", 92, 2),
        ("certified_name", 105, 0xff),
        ("qualified_name_algorithm", 140, 0x0c),
    ] {
        let mut bad = response.clone();
        bad.evidence.as_mut().unwrap().attest[offset] = value;
        fixture.sign(&mut bad);
        assert!(
            verify_attestation(&prepared.verification, DATA, &bad).is_err(),
            "{field}"
        );
    }
    let mut trailing = response.clone();
    trailing.evidence.as_mut().unwrap().attest.push(0);
    fixture.sign(&mut trailing);
    assert!(verify_attestation(&prepared.verification, DATA, &trailing).is_err());
    for length in 0..response.evidence.as_ref().unwrap().attest.len() {
        let mut bad = response.clone();
        bad.evidence.as_mut().unwrap().attest.truncate(length);
        fixture.sign(&mut bad);
        assert!(
            verify_attestation(&prepared.verification, DATA, &bad).is_err(),
            "prefix length {length}"
        );
    }
}

#[test]
fn test_signing_key_attributes_and_profiles_cannot_be_forged_in_evidence() {
    let fixture = Fixture::new(false);
    let prepared = create_challenge(&fixture.metadata).unwrap();
    let secret = activate(
        &fixture,
        &prepared.challenge,
        &name(&fixture.metadata.ak_public),
    );
    let response = fixture.response(secret, DATA);
    for mask in [
        1u32 << 1,
        1 << 4,
        1 << 5,
        1 << 6,
        1 << 18,
        1 << 16,
        1 << 17,
        1 << 11,
    ] {
        let mut bad = response.clone();
        let evidence = bad.evidence.as_mut().unwrap();
        let attributes =
            u32::from_be_bytes(evidence.identity_public[4..8].try_into().unwrap()) ^ mask;
        evidence.identity_public[4..8].copy_from_slice(&attributes.to_be_bytes());
        assert!(
            verify_attestation(&prepared.verification, DATA, &bad).is_err(),
            "attribute {mask:#x}"
        );
    }
    for field in [
        "ak_attributes",
        "ak_hash",
        "ak_curve",
        "ek_hash",
        "ek_symmetric",
    ] {
        let mut metadata = fixture.metadata.clone();
        match field {
            "ak_attributes" => metadata.ak_public[7] ^= 2,
            "ak_hash" => metadata.ak_public[3] = 0x0c,
            "ak_curve" => metadata.ak_public[17] = 0x04,
            "ek_hash" => metadata.ek_public[3] = 0x0c,
            "ek_symmetric" => metadata.ek_public[45] = 0xff,
            _ => unreachable!(),
        }
        assert!(create_challenge(&metadata).is_err(), "{field}");
    }
}

#[test]
fn test_public_area_decoder_rejects_truncation_trailing_data_and_invalid_points() {
    let fixture = Fixture::new(false);
    for length in 0..fixture.metadata.ek_public.len() {
        let mut metadata = fixture.metadata.clone();
        metadata.ek_public.truncate(length);
        assert!(create_challenge(&metadata).is_err(), "EK prefix {length}");
    }
    for length in 0..fixture.metadata.ak_public.len() {
        let mut metadata = fixture.metadata.clone();
        metadata.ak_public.truncate(length);
        assert!(create_challenge(&metadata).is_err(), "AK prefix {length}");
    }
    let mut trailing = fixture.metadata.clone();
    trailing.ak_public.push(0);
    assert!(create_challenge(&trailing).is_err());
    let mut invalid_point = fixture.metadata.clone();
    invalid_point.ak_public[22..54].fill(0);
    invalid_point.ak_public[56..88].fill(0);
    assert!(create_challenge(&invalid_point).is_err());
    let mut outer_size = fixture.metadata.clone();
    outer_size.ak_public = [vec![0, 90], outer_size.ak_public].concat();
    assert!(create_challenge(&outer_size).is_err());
}

#[test]
fn test_untrusted_input_size_limits_are_enforced() {
    let fixture = Fixture::new(false);
    for metadata in [
        TpmAttestationIdentity {
            ek_public: vec![0; 1025],
            ..fixture.metadata.clone()
        },
        TpmAttestationIdentity {
            ak_public: vec![0; 1025],
            ..fixture.metadata.clone()
        },
        TpmAttestationIdentity {
            ek_certificate_chain_der: vec![vec![]; 9],
            ..fixture.metadata.clone()
        },
        TpmAttestationIdentity {
            ek_certificate_chain_der: vec![vec![0; 32769]],
            ..fixture.metadata.clone()
        },
        TpmAttestationIdentity {
            ek_certificate_chain_der: vec![vec![0; 32768]; 5],
            ..fixture.metadata.clone()
        },
    ] {
        assert!(create_challenge(&metadata).is_err());
    }
    let prepared = create_challenge(&fixture.metadata).unwrap();
    let secret = activate(
        &fixture,
        &prepared.challenge,
        &name(&fixture.metadata.ak_public),
    );
    let response = fixture.response(secret, DATA);
    let mut bad = response.clone();
    bad.evidence.as_mut().unwrap().attest = vec![0; 1025];
    assert!(verify_attestation(&prepared.verification, DATA, &bad).is_err());
    let mut bad = response.clone();
    bad.evidence
        .as_mut()
        .unwrap()
        .signature
        .as_mut()
        .unwrap()
        .signature = vec![0; 81];
    assert!(verify_attestation(&prepared.verification, DATA, &bad).is_err());
    let mut bad = response;
    bad.activation_secret.resize(33, 0);
    assert!(verify_attestation(&prepared.verification, DATA, &bad).is_err());
}

#[test]
fn test_challenge_rejects_nonstandard_rsa_ek_parameters() {
    let fixture = Fixture::new(true);
    for field in ["bits", "exponent", "modulus_bits", "even_modulus"] {
        let mut metadata = fixture.metadata.clone();
        match field {
            "bits" => metadata.ek_public[50..52].copy_from_slice(&1024u16.to_be_bytes()),
            "exponent" => metadata.ek_public[52..56].copy_from_slice(&3u32.to_be_bytes()),
            "modulus_bits" => metadata.ek_public[58] &= 0x7f,
            "even_modulus" => *metadata.ek_public.last_mut().unwrap() &= 0xfe,
            _ => unreachable!(),
        }
        assert!(create_challenge(&metadata).is_err(), "{field}");
    }
    let mut explicit_exponent = fixture.metadata.clone();
    explicit_exponent.ek_public[52..56].copy_from_slice(&65537u32.to_be_bytes());
    let prepared = create_challenge(&explicit_exponent).unwrap();
    let secret = activate(
        &fixture,
        &prepared.challenge,
        &name(&fixture.metadata.ak_public),
    );
    let mut response = fixture.response(secret, DATA);
    response.evidence.as_mut().unwrap().ek_public = explicit_exponent.ek_public;
    assert!(verify_attestation(&prepared.verification, DATA, &response).is_ok());
}
