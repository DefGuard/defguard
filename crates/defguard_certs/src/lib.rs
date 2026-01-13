use base64::{Engine, prelude::BASE64_STANDARD};
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CertificateSigningRequestParams, IsCa,
    Issuer, KeyPair, SigningKey,
};
use rustls_pki_types::{CertificateDer, CertificateSigningRequestDer, pem::PemObject};
use thiserror::Error;

const CA_NAME: &str = "Defguard CA";
const CA_ORG: &str = "Defguard";

#[derive(Debug, Error)]
pub enum CertificateError {
    #[error("Certificate generation error: {0}")]
    RCGenError(#[from] rcgen::Error),
    #[error("Failed to parse: {0}")]
    ParsingError(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
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
        Ok(CertificateAuthority { issuer, cert_der })
    }

    pub fn from_cert_der_key_pair(
        ca_cert_der: &[u8],
        ca_key_pair: &[u8],
    ) -> Result<Self, CertificateError> {
        let key_pair = KeyPair::try_from(ca_key_pair)?;
        let cert_der = CertificateDer::from(ca_cert_der.to_vec());
        let issuer = Issuer::from_ca_cert_der(&cert_der, key_pair)?;
        Ok(CertificateAuthority { issuer, cert_der })
    }

    pub fn from_key_cert_params(
        key_pair: KeyPair,
        ca_cert_params: CertificateParams,
    ) -> Result<Self, CertificateError> {
        let cert = ca_cert_params.self_signed(&key_pair)?;
        let issuer = Issuer::new(ca_cert_params, key_pair);
        let cert_der = cert.der().clone();
        Ok(CertificateAuthority { issuer, cert_der })
    }

    pub fn new() -> Result<Self, CertificateError> {
        let mut ca_params = CertificateParams::new(vec![CA_NAME.to_string()])?;

        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        ca_params
            .distinguished_name
            .push(rcgen::DnType::OrganizationName, CA_ORG);
        ca_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, CA_NAME);

        let ca_key_pair = KeyPair::generate()?;

        CertificateAuthority::from_key_cert_params(ca_key_pair, ca_params)
    }

    pub fn sign_csr(&self, csr: &Csr) -> Result<Certificate, CertificateError> {
        let csr = csr.params()?;
        let cert = csr.signed_by(&self.issuer)?;
        Ok(cert)
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
        let ca = CertificateAuthority::new().unwrap();
        let key = ca.issuer.key();
        let der = &ca.cert_der;
        let pem_string = cert_der_to_pem(der.as_ref()).unwrap();
        let ca_loaded =
            CertificateAuthority::from_ca_cert_pem(&pem_string, &key.serialize_pem()).unwrap();
        assert_eq!(ca.cert_der, ca_loaded.cert_der);
    }

    #[test]
    fn test_sign_csr() {
        let ca = CertificateAuthority::new().unwrap();
        let cert_key_pair = KeyPair::generate().unwrap();
        let csr = Csr::new(
            &cert_key_pair,
            &["example.com".to_string(), "www.example.com".to_string()],
            vec![
                (rcgen::DnType::CommonName, "example.com"),
                (rcgen::DnType::OrganizationName, "Example Org"),
            ],
        )
        .unwrap();
        let signed_cert: Certificate = ca.sign_csr(&csr).unwrap();
        assert!(signed_cert.pem().contains("BEGIN CERTIFICATE"));
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
}
