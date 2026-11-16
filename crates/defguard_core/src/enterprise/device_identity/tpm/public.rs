use openssl::{
    bn::{BigNum, BigNumContext},
    ec::{EcGroup, EcKey, EcPoint},
    nid::Nid,
    pkey::{PKey, Public},
    rsa::Rsa,
};
use sha2::{Digest, Sha256};

use super::{MAX_PUBLIC_SIZE, Result, TpmError};

const RSA: u16 = 0x0001;
const AES: u16 = 0x0006;
const CFB: u16 = 0x0043;
pub(super) const SHA256: u16 = 0x000b;
const NULL: u16 = 0x0010;
const ECC: u16 = 0x0023;
const ECDSA: u16 = 0x0018;
const P256: u16 = 0x0003;
const FIXED_TPM: u32 = 1 << 1;
const FIXED_PARENT: u32 = 1 << 4;
const SENSITIVE_DATA_ORIGIN: u32 = 1 << 5;
const USER_WITH_AUTH: u32 = 1 << 6;
const ADMIN_WITH_POLICY: u32 = 1 << 7;
const NO_DA: u32 = 1 << 10;
const RESTRICTED: u32 = 1 << 16;
const DECRYPT: u32 = 1 << 17;
const SIGN: u32 = 1 << 18;
const RESIDENT: u32 = FIXED_TPM | FIXED_PARENT | SENSITIVE_DATA_ORIGIN;

pub(super) enum EndorsementKey {
    Rsa(PKey<Public>),
    Ecc { key: PKey<Public>, x: [u8; 32] },
}

pub(super) struct Reader<'a> {
    remaining: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    pub(super) fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        if count > self.remaining.len() {
            return Err(TpmError::InvalidEvidence("truncated TPM structure"));
        }
        let (bytes, remaining) = self.remaining.split_at(count);
        self.remaining = remaining;
        Ok(bytes)
    }

    pub(super) fn u16(&mut self) -> Result<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub(super) fn u32(&mut self) -> Result<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub(super) fn sized(&mut self) -> Result<&'a [u8]> {
        let size = usize::from(self.u16()?);
        self.take(size)
    }

    pub(super) fn finish(self) -> Result<()> {
        if !self.remaining.is_empty() {
            return Err(TpmError::InvalidEvidence("trailing TPM structure data"));
        }
        Ok(())
    }
}

fn header(bytes: &[u8]) -> Result<(Reader<'_>, u16, u32, &[u8])> {
    if bytes.is_empty() || bytes.len() > MAX_PUBLIC_SIZE {
        return Err(TpmError::InvalidPublic("public area exceeds size limits"));
    }
    let mut reader = Reader::new(bytes);
    let algorithm = reader.u16()?;
    if reader.u16()? != SHA256 {
        return Err(TpmError::UnsupportedProfile(
            "expected SHA-256 Name algorithm",
        ));
    }
    let attributes = reader.u32()?;
    let policy = reader.sized()?;
    if !policy.is_empty() && policy.len() != 32 {
        return Err(TpmError::InvalidPublic(
            "invalid SHA-256 authorization policy",
        ));
    }
    Ok((reader, algorithm, attributes, policy))
}

fn attributes_match(attributes: u32, required: u32) -> Result<()> {
    if attributes & required != required || attributes & !(required | NO_DA) != 0 {
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
    let mut padded = [0u8; 32];
    padded[32 - bytes.len()..].copy_from_slice(bytes);
    Ok(padded)
}

fn ecc_key(reader: &mut Reader<'_>) -> Result<(PKey<Public>, [u8; 32])> {
    if reader.u16()? != P256 || reader.u16()? != NULL {
        return Err(TpmError::UnsupportedProfile("expected P-256 with null KDF"));
    }
    let x = coordinate(reader.sized()?)?;
    let y = coordinate(reader.sized()?)?;
    let mut encoded = [0u8; 65];
    encoded[0] = 4;
    encoded[1..33].copy_from_slice(&x);
    encoded[33..].copy_from_slice(&y);
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
    let mut context = BigNumContext::new()?;
    let point = EcPoint::from_bytes(&group, &encoded, &mut context)
        .map_err(|_| TpmError::InvalidPublic("invalid P-256 point"))?;
    let key = EcKey::from_public_key(&group, &point)
        .map_err(|_| TpmError::InvalidPublic("invalid P-256 key"))?;
    key.check_key()
        .map_err(|_| TpmError::InvalidPublic("invalid P-256 key"))?;
    Ok((PKey::from_ec_key(key)?, x))
}

pub(super) fn endorsement_key(bytes: &[u8]) -> Result<EndorsementKey> {
    let (mut reader, algorithm, attributes, policy) = header(bytes)?;
    attributes_match(
        attributes,
        RESIDENT | ADMIN_WITH_POLICY | RESTRICTED | DECRYPT,
    )?;
    if policy.len() != 32 {
        return Err(TpmError::UnsupportedProfile(
            "expected SHA-256 EK authorization policy",
        ));
    }
    if reader.u16()? != AES || reader.u16()? != 128 || reader.u16()? != CFB {
        return Err(TpmError::UnsupportedProfile(
            "expected AES-128-CFB EK symmetric parameters",
        ));
    }
    if reader.u16()? != NULL {
        return Err(TpmError::UnsupportedProfile("expected null EK scheme"));
    }
    let key = match algorithm {
        RSA => {
            if reader.u16()? != 2048 {
                return Err(TpmError::UnsupportedProfile("expected RSA-2048 EK"));
            }
            let exponent = reader.u32()?;
            if exponent != 0 && exponent != 65537 {
                return Err(TpmError::UnsupportedProfile("expected RSA exponent 65537"));
            }
            let modulus = reader.sized()?;
            if modulus.len() != 256 || modulus[0] & 0x80 == 0 || modulus[255] & 1 == 0 {
                return Err(TpmError::InvalidPublic("invalid RSA-2048 modulus"));
            }
            let rsa = Rsa::from_public_components(
                BigNum::from_slice(modulus)?,
                BigNum::from_u32(65537)?,
            )?;
            EndorsementKey::Rsa(PKey::from_rsa(rsa)?)
        }
        ECC => {
            let (key, x) = ecc_key(&mut reader)?;
            EndorsementKey::Ecc { key, x }
        }
        _ => return Err(TpmError::UnsupportedProfile("unsupported EK algorithm")),
    };
    reader.finish()?;
    Ok(key)
}

pub(super) fn signing_key(bytes: &[u8], restricted: bool) -> Result<PKey<Public>> {
    let (mut reader, algorithm, attributes, policy) = header(bytes)?;
    if algorithm != ECC {
        return Err(TpmError::UnsupportedProfile("expected ECC signing key"));
    }
    let required = RESIDENT | USER_WITH_AUTH | SIGN | if restricted { RESTRICTED } else { 0 };
    attributes_match(attributes, required)?;
    if !policy.is_empty() {
        return Err(TpmError::UnsupportedProfile(
            "expected empty signing-key authorization policy",
        ));
    }
    if reader.u16()? != NULL || reader.u16()? != ECDSA || reader.u16()? != SHA256 {
        return Err(TpmError::UnsupportedProfile(
            "expected ECDSA-SHA256 signing scheme with null symmetric algorithm",
        ));
    }
    let (key, _) = ecc_key(&mut reader)?;
    reader.finish()?;
    Ok(key)
}

pub(super) fn name(public: &[u8]) -> [u8; 34] {
    let mut name = [0u8; 34];
    name[..2].copy_from_slice(&SHA256.to_be_bytes());
    name[2..].copy_from_slice(&Sha256::digest(public));
    name
}
