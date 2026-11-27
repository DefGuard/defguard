//! Parsing and profile checks for TPM public areas used in enrollment attestation.
//!
//! The original TPM encoding determines the key Name. Parsed OpenSSL keys are used
//! for cryptographic operations.

use openssl::{
    bn::BigNum,
    ec::{EcGroup, EcKey},
    nid::Nid,
    pkey::{PKey, Public},
    rsa::Rsa,
};
use sha2::{Digest, Sha256};

use tpm2_protocol::{
    TpmUnmarshal,
    data::{
        TpmAlgId, TpmEccCurve, TpmaObject, TpmsEccPoint, TpmtPublic, TpmtSymDefObject,
        TpmuAsymScheme, TpmuPublicId, TpmuPublicParms, TpmuSymKeyBits, TpmuSymMode,
    },
};

use super::{MAX_PUBLIC_SIZE, Result, TpmError};

const RESIDENT: TpmaObject = TpmaObject::from_bits_truncate(
    TpmaObject::FIXED_TPM.bits()
        | TpmaObject::FIXED_PARENT.bits()
        | TpmaObject::SENSITIVE_DATA_ORIGIN.bits(),
);

pub(super) enum EndorsementKey {
    Rsa(PKey<Public>),
    Ecc { key: PKey<Public>, x: [u8; 32] },
}

fn parse_public(bytes: &[u8]) -> Result<TpmtPublic> {
    if bytes.is_empty() || bytes.len() > MAX_PUBLIC_SIZE {
        return Err(TpmError::InvalidPublic("public area exceeds size limits"));
    }
    // Only a decoded public area can reach the role-specific profile checks.
    let (public, rest) = TpmtPublic::unmarshal(bytes)
        .map_err(|_| TpmError::InvalidEvidence("malformed TPM public area"))?;
    if !rest.is_empty() {
        return Err(TpmError::InvalidEvidence("trailing TPM public area data"));
    }
    if public.name_alg != TpmAlgId::Sha256 {
        return Err(TpmError::UnsupportedProfile(
            "expected SHA-256 Name algorithm",
        ));
    }
    let policy = public.auth_policy.as_ref();
    if !policy.is_empty() && policy.len() != 32 {
        return Err(TpmError::InvalidPublic(
            "invalid SHA-256 authorization policy",
        ));
    }
    Ok(public)
}

fn attributes_match(attributes: TpmaObject, required: TpmaObject) -> Result<()> {
    // NO_DA is optional. Check the full bit mask so reserved bits are also rejected.
    let allowed = required | TpmaObject::NO_DA;
    if !attributes.contains(required) || attributes.bits() & !allowed.bits() != 0 {
        return Err(TpmError::UnsupportedProfile(
            "invalid key object attributes",
        ));
    }
    Ok(())
}

fn coordinate(bytes: &[u8]) -> Result<[u8; 32]> {
    if bytes.is_empty() || bytes.len() > 32 {
        return Err(TpmError::InvalidPublic("invalid P-256 coordinate size"));
    }
    // TPM coordinates may omit leading zeroes. The ECC credential KDF needs X
    // in its fixed-width P-256 representation.
    let mut padded = [0u8; 32];
    padded[32 - bytes.len()..].copy_from_slice(bytes);
    Ok(padded)
}

fn ecc_key(point: &TpmsEccPoint) -> Result<(PKey<Public>, [u8; 32])> {
    let x = coordinate(point.x.as_ref())?;
    let y = coordinate(point.y.as_ref())?;
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
    let x_coordinate = BigNum::from_slice(&x)?;
    let y_coordinate = BigNum::from_slice(&y)?;
    let key = EcKey::from_public_key_affine_coordinates(&group, &x_coordinate, &y_coordinate)
        .map_err(|_| TpmError::InvalidPublic("invalid P-256 point"))?;
    key.check_key()
        .map_err(|_| TpmError::InvalidPublic("invalid P-256 key"))?;
    Ok((PKey::from_ec_key(key)?, x))
}

/// Checks the credential protection parameters shared by RSA and ECC endorsement keys.
///
/// Core wraps the credential with AES-128-CFB and selects the seed protection scheme
/// based on the EK type. The public area must specify `TPM_ALG_NULL` for its scheme.
fn check_ek_parameters(symmetric: TpmtSymDefObject, scheme: TpmAlgId) -> Result<()> {
    if !matches!(
        symmetric,
        TpmtSymDefObject {
            algorithm: TpmAlgId::Aes,
            key_bits: TpmuSymKeyBits::Aes(bits),
            mode: TpmuSymMode::Aes(TpmAlgId::Cfb),
        } if bits.value() == 128
    ) {
        return Err(TpmError::UnsupportedProfile(
            "expected AES-128-CFB EK symmetric parameters",
        ));
    }
    if scheme != TpmAlgId::Null {
        return Err(TpmError::UnsupportedProfile("expected null EK scheme"));
    }
    Ok(())
}

/// Checks the ECC profile shared by endorsement and signing keys.
///
/// Core accepts NIST P-256 with `TPM_ALG_NULL` in the public area's KDF field.
/// This field is separate from the KDFe step Core uses for ECC credential activation.
fn check_ecc_parameters(curve: TpmEccCurve, kdf: TpmAlgId) -> Result<()> {
    if curve != TpmEccCurve::NistP256 || kdf != TpmAlgId::Null {
        return Err(TpmError::UnsupportedProfile("expected P-256 with null KDF"));
    }
    Ok(())
}

pub(super) fn endorsement_key(bytes: &[u8]) -> Result<EndorsementKey> {
    let public = parse_public(bytes)?;
    attributes_match(
        public.object_attributes,
        RESIDENT | TpmaObject::ADMIN_WITH_POLICY | TpmaObject::RESTRICTED | TpmaObject::DECRYPT,
    )?;
    if public.auth_policy.as_ref().len() != 32 {
        return Err(TpmError::UnsupportedProfile(
            "expected SHA-256 EK authorization policy",
        ));
    }
    match (public.parameters, public.unique) {
        (TpmuPublicParms::Rsa(params), TpmuPublicId::Rsa(modulus)) => {
            check_ek_parameters(params.symmetric, params.scheme.scheme)?;
            if params.key_bits.value() != 2048 {
                return Err(TpmError::UnsupportedProfile("expected RSA-2048 EK"));
            }
            // TPM encodes the default RSA exponent of 65537 as zero.
            if !matches!(params.exponent.value(), 0 | 65537) {
                return Err(TpmError::UnsupportedProfile("expected RSA exponent 65537"));
            }
            let modulus = modulus.as_ref();
            if modulus.len() != 256 || modulus[0] & 0x80 == 0 || modulus[255] & 1 == 0 {
                return Err(TpmError::InvalidPublic("invalid RSA-2048 modulus"));
            }
            let rsa = Rsa::from_public_components(
                BigNum::from_slice(modulus)?,
                BigNum::from_u32(65537)?,
            )?;
            Ok(EndorsementKey::Rsa(PKey::from_rsa(rsa)?))
        }
        (TpmuPublicParms::Ecc(params), TpmuPublicId::Ecc(point)) => {
            check_ek_parameters(params.symmetric, params.scheme.scheme)?;
            check_ecc_parameters(params.curve_id, params.kdf.scheme)?;
            let (key, x) = ecc_key(&point)?;
            Ok(EndorsementKey::Ecc { key, x })
        }
        _ => Err(TpmError::UnsupportedProfile("unsupported EK algorithm")),
    }
}

/// Requires a restricted signing key for an AK and an unrestricted one for a device identity.
pub(super) fn signing_key(bytes: &[u8], restricted: bool) -> Result<PKey<Public>> {
    let public = parse_public(bytes)?;
    let (TpmuPublicParms::Ecc(params), TpmuPublicId::Ecc(point)) =
        (public.parameters, public.unique)
    else {
        return Err(TpmError::UnsupportedProfile("expected ECC signing key"));
    };
    let mut required = RESIDENT | TpmaObject::USER_WITH_AUTH | TpmaObject::SIGN_ENCRYPT;
    if restricted {
        required |= TpmaObject::RESTRICTED;
    }
    attributes_match(public.object_attributes, required)?;
    if !public.auth_policy.as_ref().is_empty() {
        return Err(TpmError::UnsupportedProfile(
            "expected empty signing-key authorization policy",
        ));
    }
    if params.symmetric.algorithm != TpmAlgId::Null
        || params.scheme.scheme != TpmAlgId::Ecdsa
        || !matches!(params.scheme.details, TpmuAsymScheme::Hash(details) if details.hash_alg == TpmAlgId::Sha256)
    {
        return Err(TpmError::UnsupportedProfile(
            "expected ECDSA-SHA256 signing scheme with null symmetric algorithm",
        ));
    }
    check_ecc_parameters(params.curve_id, params.kdf.scheme)?;
    let (key, _) = ecc_key(&point)?;
    Ok(key)
}

/// Takes the marshalled TPMT_PUBLIC bytes with any outer TPM2B size prefix removed.
pub(super) fn name(public: &[u8]) -> [u8; 34] {
    // A digest-based TPM Name prefixes the digest with its algorithm ID. Hashing the
    // entire public area binds the key parameters and attributes as well as the key bits.
    let mut name = [0u8; 34];
    name[..2].copy_from_slice(&TpmAlgId::Sha256.value().to_be_bytes());
    name[2..].copy_from_slice(&Sha256::digest(public));
    name
}
