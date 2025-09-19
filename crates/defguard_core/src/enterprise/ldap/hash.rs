use base64::Engine;
use defguard_common::hex::to_lower_hex;
use md4::Md4;
use rand::{RngCore, rngs::OsRng};
use sha1::{
    Digest, Sha1,
    digest::generic_array::{GenericArray, sequence::Concat},
};

/// Calculate salted SHA1 hash from given password in SSHA password storage scheme.
#[must_use]
pub fn salted_sha1_hash(password: &str) -> String {
    // random bytes
    let mut salt = [0u8; 4];
    OsRng.fill_bytes(&mut salt);

    let mut pass = Vec::from(password);
    pass.extend_from_slice(&salt);

    let checksum = Sha1::digest(pass);
    let checksum = checksum.concat(GenericArray::from(salt));

    format!(
        "{{SSHA}}{}",
        base64::prelude::BASE64_STANDARD.encode(checksum)
    )
}

/// Calculate Windows NT-HASH; used for `sambaNTPassword`.
#[must_use]
pub fn nthash(password: &str) -> String {
    let password_utf16_le: Vec<u8> = password
        .encode_utf16()
        .flat_map(|c| IntoIterator::into_iter(c.to_le_bytes()))
        .collect();
    to_lower_hex(&Md4::digest(password_utf16_le))
}

/// Calculated AD password used for `unicodePwd`.
#[must_use]
pub fn unicode_pwd(password: &str) -> Vec<u8> {
    let quoted = format!("\"{password}\"");
    let utf16_bytes: Vec<u8> = quoted.encode_utf16().flat_map(u16::to_le_bytes).collect();

    utf16_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unicode_pwd() {
        let encoded = unicode_pwd("newPassword");
        let res = base64::prelude::BASE64_STANDARD.encode(encoded);
        assert_eq!(res, "IgBuAGUAdwBQAGEAcwBzAHcAbwByAGQAIgA=");
    }

    #[test]
    fn test_hash() {
        assert_eq!(nthash("password"), "8846f7eaee8fb117ad06bdd830b7586c");
        assert_eq!(
            nthash("Zażółć gęślą jaźń"),
            "d8aaaa749c60362557d56f330f6ae217"
        );
    }
}
