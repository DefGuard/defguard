use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::{TimeDelta, Utc};
use defguard_certs::parse_pem_certificate;
use jsonwebtoken::{
    Algorithm, EncodingKey, Header, encode,
    errors::{Error, ErrorKind},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub(crate) const CLIENT_ASSERTION_TYPE: &str =
    "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";
// Microsoft Entra ID documents PS256 with an `x5t#S256` thumbprint for certificate credentials.
const ALGORITHM: Algorithm = Algorithm::PS256;
const VALIDITY: TimeDelta = TimeDelta::minutes(5);

#[derive(Serialize)]
struct Claims<'a> {
    aud: &'a str,
    iss: &'a str,
    sub: &'a str,
    jti: String,
    nbf: i64,
    iat: i64,
    exp: i64,
}

/// Sign a `private_key_jwt` client assertion. `audience` is the token endpoint, and the key
/// must be RSA.
pub(crate) fn client_assertion(
    client_id: &str,
    cert_pem: &str,
    key_pem: &str,
    audience: &str,
) -> Result<String, Error> {
    let cert_der = parse_pem_certificate(cert_pem).map_err(|_| ErrorKind::InvalidKeyFormat)?;
    let key = EncodingKey::from_rsa_pem(key_pem.as_bytes())?;
    let mut header = Header::new(ALGORITHM);
    header.x5t_s256 = Some(BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(&cert_der)));
    let now = Utc::now();
    let claims = Claims {
        aud: audience,
        iss: client_id,
        sub: client_id,
        jti: Uuid::new_v4().to_string(),
        nbf: now.timestamp(),
        iat: now.timestamp(),
        exp: (now + VALIDITY).timestamp(),
    };
    encode(&header, &claims, &key)
}

#[cfg(test)]
pub(crate) mod tests {
    use defguard_certs::{PemLabel, der_to_pem, generate_key_pair};
    use jsonwebtoken::{DecodingKey, Validation, decode, decode_header};
    use rcgen::{CertificateParams, KeyPair};
    use rsa::{
        RsaPrivateKey,
        pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding},
    };

    use super::*;

    /// (certificate, PKCS#8 private key, public key), all PEM.
    pub(crate) fn rsa_cert() -> (String, String, String) {
        let private_key = RsaPrivateKey::new(&mut rand::thread_rng(), 2048).unwrap();
        let key_pem = private_key
            .to_pkcs8_pem(LineEnding::LF)
            .unwrap()
            .to_string();
        let public_pem = private_key
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        let key_pair = KeyPair::from_pem(&key_pem).unwrap();
        let cert = CertificateParams::new(vec!["defguard-test".to_owned()])
            .unwrap()
            .self_signed(&key_pair)
            .unwrap();
        (cert.pem(), key_pem, public_pem)
    }

    #[test]
    fn test_client_assertion() {
        let (cert_pem, key_pem, public_pem) = rsa_cert();
        let audience = "https://login.example.com/tenant/oauth2/v2.0/token";

        let token = client_assertion("client-id", &cert_pem, &key_pem, audience).unwrap();

        let header = decode_header(&token).unwrap();
        assert_eq!(header.alg, Algorithm::PS256);
        let cert_der = parse_pem_certificate(&cert_pem).unwrap();
        assert_eq!(
            header.x5t_s256.unwrap(),
            BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(&cert_der))
        );

        // Decoding verifies the signature, `exp`, `nbf`, `aud`, and `iss`.
        let mut validation = Validation::new(Algorithm::PS256);
        validation.set_audience(&[audience]);
        validation.set_issuer(&["client-id"]);
        validation.validate_nbf = true;
        let claims = decode::<serde_json::Value>(
            &token,
            &DecodingKey::from_rsa_pem(public_pem.as_bytes()).unwrap(),
            &validation,
        )
        .unwrap()
        .claims;
        assert_eq!(claims["sub"], "client-id");
        assert!(claims["jti"].as_str().is_some_and(|jti| !jti.is_empty()));
        assert_eq!(
            claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap(),
            VALIDITY.num_seconds()
        );

        let ec_key_pem = der_to_pem(
            generate_key_pair().unwrap().serialize_der().as_slice(),
            PemLabel::PrivateKey,
        )
        .unwrap();
        assert!(client_assertion("client-id", &cert_pem, &ec_key_pem, audience).is_err());
        assert!(client_assertion("client-id", "not a certificate", &key_pem, audience).is_err());
    }
}
