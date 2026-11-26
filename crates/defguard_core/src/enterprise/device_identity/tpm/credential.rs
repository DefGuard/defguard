use defguard_proto::device_identity::{CredentialChallenge, TpmAttestationIdentity};
use openssl::{
    bn::{BigNum, BigNumContext},
    derive::Deriver,
    ec::{EcGroup, EcKey},
    encrypt::Encrypter,
    hash::MessageDigest,
    nid::Nid,
    pkey::PKey,
    rand::rand_bytes,
    rsa::Padding,
    sign::Signer,
    symm::{Cipher, Crypter, Mode},
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::{
    PreparedAttestation, Result, TpmError, check_certificates,
    public::{EndorsementKey, endorsement_key, name, signing_key},
};
use crate::enterprise::db::models::device_identity::DeviceAttestationChallenge;

fn hmac(key: &[u8], parts: &[&[u8]]) -> Result<Zeroizing<Vec<u8>>> {
    let key = PKey::hmac(key)?;
    let mut signer = Signer::new(MessageDigest::sha256(), &key)?;
    for part in parts {
        signer.update(part)?;
    }
    Ok(Zeroizing::new(signer.sign_to_vec()?))
}

/// Derives at most 256 bits because this implementation emits a single HMAC-SHA-256 block.
fn kdfa(seed: &[u8], label: &[u8], context: &[u8], bits: u32) -> Result<Zeroizing<Vec<u8>>> {
    // The counter starts at 1, the label is NUL-terminated, and the HMAC input includes
    // the requested output length in bits. The second TPM KDFa context is empty here.
    let mut key = hmac(
        seed,
        &[
            &1u32.to_be_bytes(),
            label,
            &[0],
            context,
            &bits.to_be_bytes(),
        ],
    )?;
    key.truncate((bits / 8) as usize);
    Ok(key)
}

fn share_secret(ek: &EndorsementKey) -> Result<(Zeroizing<Vec<u8>>, Vec<u8>)> {
    match ek {
        EndorsementKey::Rsa(key) => {
            // For RSA, the TPM recovers a random seed by decrypting it with the private EK.
            // The TPM includes the terminal NUL in the OAEP label.
            let mut seed = Zeroizing::new(vec![0u8; 32]);
            rand_bytes(&mut seed)?;
            let mut encrypt = Encrypter::new(key)?;
            encrypt.set_rsa_padding(Padding::PKCS1_OAEP)?;
            encrypt.set_rsa_oaep_md(MessageDigest::sha256())?;
            encrypt.set_rsa_mgf1_md(MessageDigest::sha256())?;
            encrypt.set_rsa_oaep_label(b"IDENTITY\0")?;
            let mut encrypted = vec![0; encrypt.encrypt_len(&seed)?];
            let length = encrypt.encrypt(&seed, &mut encrypted)?;
            encrypted.truncate(length);
            Ok((seed, encrypted))
        }
        EndorsementKey::Ecc { key, x } => {
            // For ECC, the TPM derives the same ECDH shared secret from the ephemeral public
            // point and its private EK. There is no encrypted seed to decrypt.
            let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
            let ephemeral = EcKey::generate(&group)?;
            let mut context = BigNumContext::new()?;
            let mut ephemeral_x = BigNum::new()?;
            let mut ephemeral_y = BigNum::new()?;
            ephemeral.public_key().affine_coordinates_gfp(
                &group,
                &mut ephemeral_x,
                &mut ephemeral_y,
                &mut context,
            )?;
            let ephemeral_x = ephemeral_x.to_vec_padded(32)?;
            let ephemeral_y = ephemeral_y.to_vec_padded(32)?;
            let ephemeral = PKey::from_ec_key(ephemeral)?;
            let mut derive = Deriver::new(&ephemeral)?;
            derive.set_peer(key)?;
            let shared = Zeroizing::new(derive.derive_to_vec()?);

            // KDFe derives the seed from the shared secret and the two public X coordinates.
            // Their order is fixed: ephemeral key first, then EK.
            let mut digest = Sha256::new();
            digest.update(1u32.to_be_bytes());
            digest.update(&shared);
            digest.update(b"IDENTITY\0");
            digest.update(&ephemeral_x);
            digest.update(x);
            let seed = Zeroizing::new(digest.finalize().to_vec());

            // The ECC encrypted-secret field carries the ephemeral point as two TPM2B coordinates.
            let mut encrypted = Vec::with_capacity(68);
            append_sized(&mut encrypted, &ephemeral_x)?;
            append_sized(&mut encrypted, &ephemeral_y)?;
            Ok((seed, encrypted))
        }
    }
}

fn append_sized(output: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
    let length = u16::try_from(bytes.len())
        .map_err(|_| TpmError::InvalidEvidence("TPM2B length overflow"))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

/// Wraps a fresh activation secret for the supplied EK and AK, without requiring a TPM.
/// The certificate chain is checked only for size. Its trust requires separate validation.
pub fn create_challenge(identity: &TpmAttestationIdentity) -> Result<PreparedAttestation> {
    check_certificates(&identity.ek_certificate_chain_der)?;
    let ek = endorsement_key(&identity.ek_public)?;
    signing_key(&identity.ak_public, true)?;
    let ak_name = name(&identity.ak_public);

    // The activation secret is distinct from the EK-derived wrapping seed. Core only
    // retains its digest to check the value recovered by the client at Finish.
    let mut secret = Zeroizing::new([0u8; 32]);
    rand_bytes(&mut secret[..])?;
    let activation_secret_hash = Sha256::digest(&secret[..]).into();
    let (seed, encrypted_secret) = share_secret(&ek)?;

    // The TPM derives separate encryption and HMAC keys. The AK Name is the STORAGE
    // context, while INTEGRITY uses an empty context and binds the Name in the MAC below.
    let storage = kdfa(&seed, b"STORAGE", &ak_name, 128)?;
    let integrity = kdfa(&seed, b"INTEGRITY", &[], 256)?;
    // The credential format encrypts the secret together with its TPM2B length prefix.
    // The TPM expects AES-CFB with a zero IV and no padding for this wrapping key.
    let mut plaintext = Zeroizing::new(Vec::with_capacity(34));
    append_sized(&mut plaintext, &secret[..])?;
    let mut cipher = Crypter::new(
        Cipher::aes_128_cfb128(),
        Mode::Encrypt,
        &storage,
        Some(&[0; 16]),
    )?;
    cipher.pad(false);
    let mut ciphertext = vec![0; plaintext.len() + 16];
    let written = cipher.update(&plaintext, &mut ciphertext)?;
    let written = written + cipher.finalize(&mut ciphertext[written..])?;
    ciphertext.truncate(written);
    // Authenticating the ciphertext and AK Name binds this credential to that AK.
    // The blob contains a length-prefixed MAC followed by the ciphertext as-is.
    let mac = hmac(&integrity, &[&ciphertext, &ak_name])?;
    let mut credential_blob = Vec::with_capacity(68);
    append_sized(&mut credential_blob, &mac)?;
    credential_blob.extend_from_slice(&ciphertext);
    Ok(PreparedAttestation {
        challenge: CredentialChallenge {
            credential_blob,
            encrypted_secret,
        },
        verification: DeviceAttestationChallenge::Tpm {
            ak_name: ak_name.to_vec(),
            ek_fingerprint: Sha256::digest(&identity.ek_public).into(),
            activation_secret_hash,
        },
    })
}
