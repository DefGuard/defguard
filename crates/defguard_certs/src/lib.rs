use std::str::FromStr;

use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::NaiveDateTime;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CertificateSigningRequestParams, IsCa,
    Issuer, KeyPair, KeyUsagePurpose, SigningKey, string::Ia5String,
};
use rustls_pki_types::{CertificateDer, CertificateSigningRequestDer, pem::PemObject};
use thiserror::Error;
use time::{Duration, OffsetDateTime};
use x509_parser::{
    extensions::{GeneralName, ParsedExtension},
    parse_x509_certificate,
};

pub use rcgen::ExtendedKeyUsagePurpose;

const CA_NAME: &str = "Defguard CA";
const NOT_BEFORE_OFFSET_SECS: Duration = Duration::minutes(5);
const DEFAULT_CERT_VALIDITY_DAYS: i64 = 1825;

#[derive(Debug, Error)]
pub enum CertificateError {
    #[error("Certificate generation error: {0}")]
    RCGenError(#[from] rcgen::Error),
    #[error("Failed to parse: {0}")]
    ParsingError(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error("CSR hostname mismatch: {0}")]
    HostnameMismatch(String),
}

pub struct CertificateAuthority<'a> {
    issuer: Issuer<'a, KeyPair>,
    cert_der: CertificateDer<'a>,
}

impl CertificateAuthority<'_> {
    pub fn from_ca_cert_pem(
        ca_cert_pem: &str,
        ca_key_pair: &str,
    ) -> Result<Self, CertificateError> {
        let key_pair = KeyPair::from_pem(ca_key_pair)?;
        let cert_der = CertificateDer::from_pem_slice(ca_cert_pem.as_bytes())
            .map_err(|e| CertificateError::ParsingError(e.to_string()))?;
        let issuer = Issuer::from_ca_cert_der(&cert_der, key_pair)?;
        Ok(Self { issuer, cert_der })
    }

    pub fn from_cert_der_key_pair(
        ca_cert_der: &[u8],
        ca_key_pair: &[u8],
    ) -> Result<Self, CertificateError> {
        let key_pair = KeyPair::try_from(ca_key_pair)?;
        let cert_der = CertificateDer::from(ca_cert_der.to_vec());
        let issuer = Issuer::from_ca_cert_der(&cert_der, key_pair)?;
        Ok(Self { issuer, cert_der })
    }

    pub fn from_key_cert_params(
        key_pair: KeyPair,
        ca_cert_params: CertificateParams,
    ) -> Result<Self, CertificateError> {
        let cert = ca_cert_params.self_signed(&key_pair)?;
        let issuer = Issuer::new(ca_cert_params, key_pair);
        let cert_der = cert.der().clone();
        Ok(Self { issuer, cert_der })
    }

    pub fn new(
        common_name: &str,
        email: &str,
        valid_for_days: u32,
    ) -> Result<Self, CertificateError> {
        let mut ca_params = CertificateParams::new(vec![CA_NAME.to_string()])?;

        // path length 0 to avoid issuing further CAs
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        ca_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, common_name);

        let email_string = Ia5String::from_str(email)?;
        ca_params
            .subject_alt_names
            .push(rcgen::SanType::Rfc822Name(email_string));

        let now = OffsetDateTime::now_utc();
        ca_params.not_before = now - NOT_BEFORE_OFFSET_SECS;
        ca_params.not_after = now + Duration::days(i64::from(valid_for_days));

        let ca_key_pair = KeyPair::generate()?;

        Self::from_key_cert_params(ca_key_pair, ca_params)
    }

    /// Sign a server-facing component certificate (`ServerAuth` EKU only).
    ///
    /// Use [`sign_client_cert`] for Core gRPC client certificates, or
    /// [`sign_csr_with_validity`] when custom validity is needed.
    pub fn sign_server_cert(&self, csr: &Csr) -> Result<Certificate, CertificateError> {
        self.sign_csr_with_validity(
            csr,
            DEFAULT_CERT_VALIDITY_DAYS,
            &[ExtendedKeyUsagePurpose::ServerAuth],
        )
    }

    /// Sign a Core gRPC client certificate (`ClientAuth` EKU only).
    pub fn sign_client_cert(&self, csr: &Csr) -> Result<Certificate, CertificateError> {
        self.sign_csr_with_validity(
            csr,
            DEFAULT_CERT_VALIDITY_DAYS,
            &[ExtendedKeyUsagePurpose::ClientAuth],
        )
    }

    /// Sign a CSR with explicit validity in days and extended key usages.
    ///
    /// `extended_key_usages` controls which EKUs are encoded in the signed
    /// certificate.  Pass `&[ServerAuth]` for component server certs and
    /// `&[ClientAuth]` for Core gRPC client certs.
    pub fn sign_csr_with_validity(
        &self,
        csr: &Csr,
        days_valid: i64,
        extended_key_usages: &[ExtendedKeyUsagePurpose],
    ) -> Result<Certificate, CertificateError> {
        let mut csr_params = csr.params()?;

        let now = OffsetDateTime::now_utc();
        let not_before = now - NOT_BEFORE_OFFSET_SECS;
        let not_after = now + Duration::days(days_valid);

        csr_params.params.not_before = not_before;
        csr_params.params.not_after = not_after;

        csr_params.params.key_usages = vec![
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::KeyEncipherment,
        ];
        csr_params.params.extended_key_usages = extended_key_usages.to_vec();

        let cert = csr_params.signed_by(&self.issuer)?;
        Ok(cert)
    }

    /// Issue a Core gRPC client certificate for a specific Gateway or Proxy.
    ///
    /// Generates a fresh key pair, creates a CSR with `common_name` as both
    /// the Subject CN and the SAN DNS name, signs it with `ClientAuth` EKU,
    /// and returns all materials needed to store in the database and build a
    /// [`CertBundle`].
    pub fn issue_core_client_cert(
        &self,
        common_name: &str,
    ) -> Result<CoreClientCert, CertificateError> {
        let key_pair = generate_key_pair()?;
        let csr = Csr::new(
            &key_pair,
            &[common_name.to_string()],
            vec![(rcgen::DnType::CommonName, common_name)],
        )?;
        let cert = self.sign_client_cert(&csr)?;
        let expiry = CertificateInfo::from_der(cert.der())?.not_after;
        Ok(CoreClientCert {
            cert_der: cert.der().to_vec(),
            key_der: key_pair.serialized_der().to_vec(),
            expiry,
        })
    }

    pub fn cert_pem(&self) -> Result<String, CertificateError> {
        der_to_pem(self.cert_der.as_ref(), PemLabel::Certificate)
    }

    #[must_use]
    pub fn cert_der(&self) -> &[u8] {
        self.cert_der.as_ref()
    }

    #[must_use]
    pub fn key_pair_der(&self) -> &[u8] {
        self.issuer.key().serialized_der()
    }

    pub fn expiry(&self) -> Result<NaiveDateTime, CertificateError> {
        let CertificateInfo { not_after, .. } = CertificateInfo::from_der(&self.cert_der)?;
        Ok(not_after)
    }
}

/// A Core gRPC client certificate issued for a specific Gateway or Proxy component.
///
/// The DER bytes are stored in the database; the key bytes never leave Core.
pub struct CoreClientCert {
    /// DER-encoded client certificate signed with `ClientAuth` EKU.
    pub cert_der: Vec<u8>,
    /// DER-encoded private key for the client certificate.
    pub key_der: Vec<u8>,
    /// Certificate expiry timestamp (UTC).
    pub expiry: NaiveDateTime,
}

pub struct CertificateInfo {
    pub subject_common_name: String,
    pub subject_email: Option<String>,
    pub not_before: NaiveDateTime,
    pub not_after: NaiveDateTime,
    pub serial: String,
}

impl CertificateInfo {
    /// Parse certificate from DER-encoded bytes.
    pub fn from_der(cert_der: &[u8]) -> Result<Self, CertificateError> {
        let (_, parsed) = parse_x509_certificate(cert_der).map_err(|e| {
            CertificateError::ParsingError(format!("Failed to parse certificate: {e}"))
        })?;

        let subject = &parsed.tbs_certificate.subject;
        let serial = parsed.raw_serial_as_string();
        let subject_email = parsed
            .tbs_certificate
            .extensions()
            .iter()
            .filter_map(|ext| match ext.parsed_extension() {
                ParsedExtension::SubjectAlternativeName(san) => Some(san),
                _ => None,
            })
            .flat_map(|san| san.general_names.iter())
            .find_map(|name| match name {
                GeneralName::RFC822Name(email) => Some(email.to_string()),
                _ => None,
            });

        let cn = subject
            .iter_common_name()
            .next()
            .ok_or_else(|| CertificateError::ParsingError("Common Name not found".to_string()))?
            .as_str()
            .map_err(|e| {
                CertificateError::ParsingError(format!("Failed to parse CN as string: {e}"))
            })?;

        let validity = &parsed.tbs_certificate.validity;
        let not_before = validity.not_before.to_datetime();
        let not_after = validity.not_after.to_datetime();

        Ok(Self {
            subject_common_name: cn.to_string(),
            subject_email,
            not_before: chrono::DateTime::from_timestamp(not_before.unix_timestamp(), 0)
                .ok_or_else(|| {
                    CertificateError::ParsingError(format!(
                        "Failed to convert certificate not_before {not_before} to NaiveDateTime",
                    ))
                })?
                .naive_utc(),
            not_after: chrono::DateTime::from_timestamp(not_after.unix_timestamp(), 0)
                .ok_or_else(|| {
                    CertificateError::ParsingError(format!(
                        "Failed to convert certificate not_after {not_after} to NaiveDateTime",
                    ))
                })?
                .naive_utc(),
            serial,
        })
    }
}

pub struct Csr<'a> {
    csr: CertificateSigningRequestDer<'a>,
}

impl Csr<'_> {
    pub fn new(
        key_pair: &impl SigningKey,
        subject_alt_names: &[String],
        dinstinguished_name: Vec<(rcgen::DnType, &str)>,
    ) -> Result<Self, CertificateError> {
        let mut csr_params = CertificateParams::new(subject_alt_names.to_vec())?;
        for (dn_type, value) in dinstinguished_name {
            csr_params.distinguished_name.push(dn_type, value);
        }
        let request = csr_params.serialize_request(key_pair)?;
        let csr = request.der().clone();
        Ok(Self { csr })
    }

    pub fn from_der(csr_der: &[u8]) -> Result<Self, CertificateError> {
        let csr = CertificateSigningRequestDer::from(csr_der.to_vec());
        Ok(Self { csr })
    }

    pub fn params(&self) -> Result<CertificateSigningRequestParams, CertificateError> {
        let params = CertificateSigningRequestParams::from_der(&self.csr)
            .map_err(|e| CertificateError::ParsingError(e.to_string()))?;
        Ok(params)
    }

    /// Verify that the CSR's SAN list contains exactly `expected_hostname` and
    /// nothing else. The hostname may be a DNS name or an IP address literal.
    ///
    /// This is used during component setup to ensure the component has not
    /// substituted a different hostname in the CSR it returns to Core.
    pub fn verify_hostname(&self, expected_hostname: &str) -> Result<(), CertificateError> {
        let params = self.params()?;
        let sans = &params.params.subject_alt_names;

        if sans.is_empty() {
            return Err(CertificateError::HostnameMismatch(format!(
                "CSR contains no SANs; expected {expected_hostname:?}"
            )));
        }

        let expected_ip: Option<std::net::IpAddr> = expected_hostname.parse().ok();

        for san in sans {
            let matches = match san {
                rcgen::SanType::IpAddress(ip) => expected_ip.is_some_and(|e| &e == ip),
                rcgen::SanType::DnsName(name) => {
                    expected_ip.is_none() && name.as_str() == expected_hostname
                }
                _ => false,
            };
            if !matches {
                return Err(CertificateError::HostnameMismatch(format!(
                    "CSR SAN does not match expected hostname {expected_hostname:?}"
                )));
            }
        }

        Ok(())
    }

    #[must_use]
    pub fn to_der(&self) -> &[u8] {
        self.csr.as_ref()
    }
}

#[derive(Debug, Copy, Clone)]
pub enum PemLabel {
    Certificate,
    PrivateKey,
    PublicKey,
}

impl PemLabel {
    #[must_use]
    pub const fn as_str(&self) -> &str {
        match self {
            Self::Certificate => "CERTIFICATE",
            Self::PrivateKey => "PRIVATE KEY",
            Self::PublicKey => "PUBLIC KEY",
        }
    }
}

pub fn der_to_pem(der: &[u8], label: PemLabel) -> Result<String, CertificateError> {
    let b64 = BASE64_STANDARD.encode(der);
    let pem_string = format!(
        "-----BEGIN {}-----\n{}\n-----END {}-----",
        label.as_str(),
        b64.as_bytes()
            .chunks(64)
            .map(|chunk| std::str::from_utf8(chunk)
                .map_err(|e| CertificateError::ParsingError(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?
            .join("\n"),
        label.as_str(),
    );
    Ok(pem_string)
}

pub fn cert_der_to_pem(cert_der: &[u8]) -> Result<String, CertificateError> {
    der_to_pem(cert_der, PemLabel::Certificate)
}

pub fn generate_key_pair() -> Result<KeyPair, CertificateError> {
    let key_pair = KeyPair::generate()?;
    Ok(key_pair)
}

pub fn parse_pem_certificate(pem_str: &str) -> Result<CertificateDer<'_>, CertificateError> {
    let cert_der = CertificateDer::from_pem_slice(pem_str.as_bytes())
        .map_err(|e| CertificateError::ParsingError(e.to_string()))?;
    Ok(cert_der)
}

pub type DnType = rcgen::DnType;
pub type RcGenKeyPair = rcgen::KeyPair;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_from_der() {
        let key_pair = KeyPair::generate().unwrap();
        let csr = Csr::new(
            &key_pair,
            &["example.com".to_string()],
            vec![(rcgen::DnType::CommonName, "example.com")],
        )
        .unwrap();
        let der = csr.to_der();
        let csr_loaded = Csr::from_der(der).unwrap();
        assert_eq!(csr.to_der(), csr_loaded.to_der());
    }

    #[test]
    fn test_ca_creation() {
        let ca = CertificateAuthority::new("Defguard CA", "email@email.com", 10).unwrap();
        let key = ca.issuer.key();
        let der = &ca.cert_der;
        let pem_string = cert_der_to_pem(der.as_ref()).unwrap();
        let ca_loaded =
            CertificateAuthority::from_ca_cert_pem(&pem_string, &key.serialize_pem()).unwrap();
        assert_eq!(ca.cert_der, ca_loaded.cert_der);
    }

    #[test]
    fn test_sign_server_cert() {
        let ca = CertificateAuthority::new("Defguard CA", "email@email.com", 10).unwrap();
        let cert_key_pair = generate_key_pair().unwrap();
        let csr = Csr::new(
            &cert_key_pair,
            &["example.com".to_string(), "www.example.com".to_string()],
            vec![
                (rcgen::DnType::CommonName, "example.com"),
                (rcgen::DnType::OrganizationName, "Example Org"),
            ],
        )
        .unwrap();
        let signed_cert: Certificate = ca.sign_server_cert(&csr).unwrap();
        assert!(signed_cert.pem().contains("BEGIN CERTIFICATE"));
    }

    #[test]
    fn test_sign_csr_with_validity() {
        use x509_parser::parse_x509_certificate;

        let ca = CertificateAuthority::new("Defguard CA", "email@email.com", 10).unwrap();
        let cert_key_pair = generate_key_pair().unwrap();
        let csr = Csr::new(
            &cert_key_pair,
            &["example.com".to_string()],
            vec![(rcgen::DnType::CommonName, "example.com")],
        )
        .unwrap();
        let signed_cert: Certificate = ca
            .sign_csr_with_validity(&csr, 90, &[ExtendedKeyUsagePurpose::ServerAuth])
            .unwrap();
        let der = signed_cert.der();
        let (_rem, parsed) = parse_x509_certificate(der).unwrap();
        let validity = parsed.tbs_certificate.validity;
        let not_before = validity.not_before.to_datetime();
        let not_after = validity.not_after.to_datetime();
        let days = (not_after - not_before).whole_days();
        assert!((89..=91).contains(&days), "expected 89-91 days, got {days}");
        assert!(not_after > not_before);
    }

    #[test]
    fn test_der_to_pem() {
        assert_eq!(PemLabel::Certificate.as_str(), "CERTIFICATE");
        assert_eq!(PemLabel::PrivateKey.as_str(), "PRIVATE KEY");
        assert_eq!(PemLabel::PublicKey.as_str(), "PUBLIC KEY");

        // chunking: make sure lines are 64 chars except last
        let data = vec![0u8; 200];
        let pem = der_to_pem(&data, PemLabel::PublicKey).unwrap();
        assert!(pem.starts_with("-----BEGIN PUBLIC KEY-----"));
        assert!(pem.ends_with("-----END PUBLIC KEY-----"));
        let inner_lines: Vec<&str> = pem
            .lines()
            .skip(1)
            .take_while(|l| !l.starts_with("-----END"))
            .collect();
        assert!(inner_lines.len() >= 2);
        for (i, line) in inner_lines.iter().enumerate() {
            if i + 1 < inner_lines.len() {
                assert_eq!(line.len(), 64);
            } else {
                assert!(line.len() <= 64);
            }
        }
    }

    #[test]
    fn test_ca_validity() {
        use x509_parser::parse_x509_certificate;

        let valid_days = 365;
        let ca = CertificateAuthority::new("Test CA", "test@example.com", valid_days).unwrap();

        let (_rem, parsed) = parse_x509_certificate(ca.cert_der()).unwrap();
        let validity = parsed.tbs_certificate.validity;
        let not_before = validity.not_before.to_datetime();
        let not_after = validity.not_after.to_datetime();

        let days = (not_after - not_before).whole_days();

        assert!(
            (i64::from(valid_days) - 1..=i64::from(valid_days) + 1).contains(&days),
            "expected validity of {valid_days} days (±1), got {days} days"
        );
        assert!(
            not_after > not_before,
            "not_after should be after not_before"
        );
    }

    #[test]
    fn test_ca_common_name() {
        use x509_parser::parse_x509_certificate;

        let expected_cn = "My Custom CA";
        let ca = CertificateAuthority::new(expected_cn, "admin@example.com", 365).unwrap();

        let (_rem, parsed) = parse_x509_certificate(ca.cert_der()).unwrap();
        let subject = &parsed.tbs_certificate.subject;

        let cn = subject
            .iter_common_name()
            .next()
            .expect("Common Name not found")
            .as_str()
            .expect("Failed to parse CN as string");

        assert_eq!(
            cn, expected_cn,
            "Common Name should match the provided value"
        );
    }

    #[test]
    fn test_ca_email() {
        let expected_email = "contact@defguard.net";
        let ca = CertificateAuthority::new("Test CA", expected_email, 365).unwrap();

        let info = CertificateInfo::from_der(ca.cert_der()).unwrap();

        assert_eq!(
            info.subject_email.as_deref(),
            Some(expected_email),
            "Email should be parsed from Subject Alternative Names"
        );
    }

    #[test]
    fn test_parse_pem_certificate() {
        // Create a CA and get its PEM representation
        let ca = CertificateAuthority::new("Defguard CA", "test@example.com", 365).unwrap();
        let pem = ca.cert_pem().unwrap();

        // Parse the PEM back to DER and ensure it matches the original
        let parsed = parse_pem_certificate(&pem).unwrap();
        assert_eq!(parsed, ca.cert_der);
    }

    #[test]
    fn test_csr_verify_hostname_dns_ok() {
        let key = generate_key_pair().unwrap();
        let csr = Csr::new(&key, &["proxy.example.com".to_string()], vec![]).unwrap();
        assert!(
            csr.verify_hostname("proxy.example.com").is_ok(),
            "matching DNS SAN should pass"
        );
    }

    #[test]
    fn test_csr_verify_hostname_ip_ok() {
        let key = generate_key_pair().unwrap();
        let csr = Csr::new(&key, &["10.0.0.1".to_string()], vec![]).unwrap();
        assert!(
            csr.verify_hostname("10.0.0.1").is_ok(),
            "matching IP SAN should pass"
        );
    }

    #[test]
    fn test_csr_verify_hostname_mismatch() {
        let key = generate_key_pair().unwrap();
        let csr = Csr::new(&key, &["evil.attacker.com".to_string()], vec![]).unwrap();
        assert!(
            csr.verify_hostname("proxy.example.com").is_err(),
            "mismatched DNS SAN should fail"
        );
    }

    #[test]
    fn test_csr_verify_hostname_extra_san_rejected() {
        let key = generate_key_pair().unwrap();
        let csr = Csr::new(
            &key,
            &[
                "proxy.example.com".to_string(),
                "evil.extra.com".to_string(),
            ],
            vec![],
        )
        .unwrap();
        assert!(
            csr.verify_hostname("proxy.example.com").is_err(),
            "CSR with extra SANs beyond the expected hostname should fail"
        );
    }
}
