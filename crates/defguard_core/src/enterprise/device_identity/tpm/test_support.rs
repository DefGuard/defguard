use std::sync::LazyLock;

use aes::Aes128;
use cfb_mode::{
    Decryptor,
    cipher::{AsyncStreamCipher, KeyIvInit},
};
use defguard_proto::device_identity::{
    CredentialChallenge, Signature, SignatureAlgorithm, TpmAttestationIdentity,
    TpmAttestationResponse, TpmEvidence,
};
use hmac::{Hmac, Mac};
use p256::{
    PublicKey, SecretKey,
    ecdsa::{Signature as EcdsaSignature, SigningKey, signature::Signer},
    elliptic_curve::sec1::ToEncodedPoint,
};
use rand::rngs::OsRng;
use rsa::{Oaep, RsaPrivateKey, traits::PublicKeyParts};
use sha2_10::{Digest, Sha256};

// The RustCrypto verifier is independent of the production OpenSSL implementation.
// This lets the tests check interoperability without reusing the code under test.
pub(in crate::enterprise::device_identity) enum Ek {
    Ecc(SecretKey),
    Rsa(Box<RsaPrivateKey>),
}

static RSA_KEY: LazyLock<RsaPrivateKey> =
    LazyLock::new(|| RsaPrivateKey::new(&mut OsRng, 2048).unwrap());

pub(in crate::enterprise::device_identity) struct Fixture {
    pub ek: Ek,
    pub ak: SigningKey,
    pub identity: SigningKey,
    pub metadata: TpmAttestationIdentity,
    pub identity_public: Vec<u8>,
}

pub(in crate::enterprise::device_identity) fn sized(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn header(algorithm: u16, attributes: u32, policy: &[u8]) -> Vec<u8> {
    let mut bytes = algorithm.to_be_bytes().to_vec();
    bytes.extend_from_slice(&0x000bu16.to_be_bytes());
    bytes.extend_from_slice(&attributes.to_be_bytes());
    sized(&mut bytes, policy);
    bytes
}

fn ecc_point(output: &mut Vec<u8>, key: &PublicKey) {
    let point = key.to_encoded_point(false);
    sized(output, point.x().unwrap());
    sized(output, point.y().unwrap());
}

pub(in crate::enterprise::device_identity) fn signing_public(
    key: &SigningKey,
    restricted: bool,
) -> Vec<u8> {
    let mut bytes = header(
        0x0023,
        0x0004_0472 | if restricted { 0x0001_0000 } else { 0 },
        &[],
    );
    for value in [0x0010u16, 0x0018, 0x000b, 0x0003, 0x0010] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    let point = key.verifying_key().to_encoded_point(false);
    sized(&mut bytes, point.x().unwrap());
    sized(&mut bytes, point.y().unwrap());
    bytes
}

pub(in crate::enterprise::device_identity) fn name(public: &[u8]) -> Vec<u8> {
    [0x000bu16.to_be_bytes().as_slice(), &Sha256::digest(public)].concat()
}

impl Fixture {
    pub(in crate::enterprise::device_identity) fn new(rsa: bool) -> Self {
        let ek = if rsa {
            Ek::Rsa(Box::new(RSA_KEY.clone()))
        } else {
            Ek::Ecc(SecretKey::from_slice(&[3; 32]).unwrap())
        };
        let mut ek_public = header(if rsa { 0x0001 } else { 0x0023 }, 0x0003_00b2, &[0x42; 32]);
        for value in [0x0006u16, 128, 0x0043, 0x0010] {
            ek_public.extend_from_slice(&value.to_be_bytes());
        }
        match &ek {
            Ek::Rsa(key) => {
                ek_public.extend_from_slice(&2048u16.to_be_bytes());
                ek_public.extend_from_slice(&0u32.to_be_bytes());
                sized(&mut ek_public, &key.n().to_bytes_be());
            }
            Ek::Ecc(key) => {
                ek_public.extend_from_slice(&0x0003u16.to_be_bytes());
                ek_public.extend_from_slice(&0x0010u16.to_be_bytes());
                ecc_point(&mut ek_public, &key.public_key());
            }
        }
        let ak = SigningKey::from_slice(&[1; 32]).unwrap();
        let identity = SigningKey::from_slice(&[2; 32]).unwrap();
        let metadata = TpmAttestationIdentity {
            ek_public,
            ak_public: signing_public(&ak, true),
            ek_certificate_chain_der: vec![],
        };
        let identity_public = signing_public(&identity, false);
        Self {
            ek,
            ak,
            identity,
            metadata,
            identity_public,
        }
    }

    pub(in crate::enterprise::device_identity) fn response(
        &self,
        secret: Vec<u8>,
        data: &[u8],
    ) -> TpmAttestationResponse {
        let mut attest = 0xff54_4347u32.to_be_bytes().to_vec();
        attest.extend_from_slice(&0x8017u16.to_be_bytes());
        sized(&mut attest, &name(&self.metadata.ak_public));
        sized(&mut attest, &Sha256::digest(data));
        attest.extend_from_slice(&[0; 16]);
        attest.push(1);
        attest.extend_from_slice(&1u64.to_be_bytes());
        sized(&mut attest, &name(&self.identity_public));
        sized(&mut attest, &name(&self.identity_public));
        let mut response = TpmAttestationResponse {
            activation_secret: secret,
            evidence: Some(TpmEvidence {
                ek_public: self.metadata.ek_public.clone(),
                ek_certificate_chain_der: self.metadata.ek_certificate_chain_der.clone(),
                ak_public: self.metadata.ak_public.clone(),
                identity_public: self.identity_public.clone(),
                attest,
                signature: None,
            }),
        };
        self.sign(&mut response);
        response
    }

    pub(in crate::enterprise::device_identity) fn sign(
        &self,
        response: &mut TpmAttestationResponse,
    ) {
        let evidence = response.evidence.as_mut().unwrap();
        let signature: EcdsaSignature = self.ak.sign(&evidence.attest);
        evidence.signature = Some(Signature {
            algorithm: SignatureAlgorithm::EcdsaSha256.into(),
            signature: signature.to_der().as_bytes().to_vec(),
        });
    }
}

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn sized(&mut self) -> &'a [u8] {
        assert!(self.0.len() >= 2);
        let size = u16::from_be_bytes([self.0[0], self.0[1]]) as usize;
        let (bytes, rest) = self.0[2..].split_at(size);
        self.0 = rest;
        bytes
    }
}

fn kdfa(seed: &[u8], label: &[u8], context: &[u8], bits: u32) -> Vec<u8> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(seed).unwrap();
    mac.update(&1u32.to_be_bytes());
    mac.update(label);
    mac.update(&[0]);
    mac.update(context);
    mac.update(&bits.to_be_bytes());
    mac.finalize().into_bytes()[..bits as usize / 8].to_vec()
}

pub(in crate::enterprise::device_identity) fn activate(
    fixture: &Fixture,
    challenge: &CredentialChallenge,
    ak_name: &[u8],
) -> Vec<u8> {
    let seed = match &fixture.ek {
        Ek::Rsa(private) => private
            .decrypt(
                Oaep::new_with_label::<Sha256, _>("IDENTITY\0"),
                &challenge.encrypted_secret,
            )
            .unwrap(),
        Ek::Ecc(private) => {
            let mut reader = Reader(&challenge.encrypted_secret);
            let x = reader.sized();
            let y = reader.sized();
            assert!(reader.0.is_empty());
            assert_eq!(x.len(), 32);
            assert_eq!(y.len(), 32);
            let encoded = [vec![4], x.to_vec(), y.to_vec()].concat();
            let ephemeral = PublicKey::from_sec1_bytes(&encoded).unwrap();
            let shared =
                p256::ecdh::diffie_hellman(private.to_nonzero_scalar(), ephemeral.as_affine());
            let public = private.public_key().to_encoded_point(false);
            Sha256::digest(
                [
                    1u32.to_be_bytes().as_slice(),
                    shared.raw_secret_bytes().as_slice(),
                    b"IDENTITY\0",
                    x,
                    public.x().unwrap(),
                ]
                .concat(),
            )
            .to_vec()
        }
    };
    let mut reader = Reader(&challenge.credential_blob);
    let expected_mac = reader.sized();
    let ciphertext = reader.0;
    let integrity = kdfa(&seed, b"INTEGRITY", &[], 256);
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&integrity).unwrap();
    mac.update(ciphertext);
    mac.update(ak_name);
    mac.verify_slice(expected_mac).unwrap();
    let storage = kdfa(&seed, b"STORAGE", ak_name, 128);
    let mut plaintext = ciphertext.to_vec();
    Decryptor::<Aes128>::new(storage.as_slice().into(), &[0; 16].into()).decrypt(&mut plaintext);
    let mut reader = Reader(&plaintext);
    let secret = reader.sized().to_vec();
    assert!(reader.0.is_empty());
    secret
}
