use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rsa::traits::PublicKeyParts;
use sha2::{Digest, Sha256};

pub mod auth;
pub mod config;
pub mod csv;
pub mod db;
pub mod device_config_gen;
pub mod gateway_event;
pub mod gateway_types;
pub mod globals;
pub mod hex;
pub mod messages;
pub mod random;
pub mod secret;
pub mod types;
pub mod utils;

pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "+", env!("VERGEN_GIT_SHA"));
pub const CARGO_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Version reported to external systems.
///
/// Release workflows can override this with `DEFGUARD_BUILD_VERSION`.
pub const REPORTED_VERSION: &str = resolve_reported_version(
    option_env!("DEFGUARD_BUILD_VERSION"),
    env!("CARGO_PKG_VERSION"),
);

const fn resolve_reported_version(
    build_version: Option<&'static str>,
    cargo_version: &'static str,
) -> &'static str {
    match build_version {
        Some(version) if !version.is_empty() => version,
        _ => cargo_version,
    }
}

// WireGuard key length in bytes.
pub const KEY_LENGTH: usize = 32;

/// Compute the RFC 7638 JWK thumbprint for an RSA private key.
///
/// Used as the key ID (`kid`) for OpenID Connect signing keys.
pub fn rsa_jwk_thumbprint(key: &rsa::RsaPrivateKey) -> String {
    let n = URL_SAFE_NO_PAD.encode(key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(key.e().to_bytes_be());
    let canonical = format!(r#"{{"e":"{e}","kty":"RSA","n":"{n}"}}"#);
    let digest = Sha256::digest(canonical.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

#[cfg(test)]
mod tests {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use rand::rngs::OsRng;
    use rsa::RsaPrivateKey;
    use sha2::{Digest, Sha256};

    use super::resolve_reported_version;

    #[test]
    fn reported_version_uses_build_override_for_prereleases_and_falls_back_otherwise() {
        assert_eq!(
            resolve_reported_version(Some("2.0.0-beta.1"), "2.0.0"),
            "2.0.0-beta.1"
        );
        assert_eq!(resolve_reported_version(Some(""), "2.0.0"), "2.0.0");
        assert_eq!(resolve_reported_version(None, "2.0.0"), "2.0.0");
    }

    #[test]
    fn test_rfc7638_thumbprint_known_answer() {
        // RFC 7638 section 3.1 known values
        let n_b64u = "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw";
        let e_b64u = "AQAB";
        let expected = "NzbLsXh8uDCcd-6MNwXF4W_7noWXFZAfHkxZsRGC9Xs";

        // Build canonical JSON exactly as the helper does
        let canonical = format!(r#"{{"e":"{e_b64u}","kty":"RSA","n":"{n_b64u}"}}"#);
        let digest = Sha256::digest(canonical.as_bytes());
        let thumbprint = URL_SAFE_NO_PAD.encode(digest);

        assert_eq!(thumbprint, expected);
    }

    #[test]
    fn test_rsa_jwk_thumbprint_is_urlsafe_base64() {
        let mut rng = OsRng;
        let key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate key");
        let thumbprint = super::rsa_jwk_thumbprint(&key);

        // Must be exactly 43 characters (SHA-256 digest base64url-nopad)
        assert_eq!(thumbprint.len(), 43);
        // Must be valid base64url (no padding)
        assert!(
            thumbprint
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        // Round-trip decode must produce exactly 32 bytes
        let decoded = URL_SAFE_NO_PAD
            .decode(&thumbprint)
            .expect("valid base64url");
        assert_eq!(decoded.len(), 32);
    }
}
