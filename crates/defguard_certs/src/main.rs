use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CertificateSigningRequest,
    CertificateSigningRequestParams, IsCa, Issuer, KeyPair, PublicKey, RcgenError,
};
use rustls_pki_types::{CertificateDer, pem::PemObject};
use thiserror::Error;

const CA_NAME: &str = "DefGuard CA";
const CA_ORG: &str = "DefGuard";

#[derive(Debug, Error)]
pub enum CAError {
    #[error("Certificate generation error: {0}")]
    RCGenError(#[from] rcgen::Error),
    #[error("PEM error: {0}")]
    PemParsingError(String),
}

pub struct CertificateAuthority<'a> {
    issuer: Issuer<'a, KeyPair>,
    cert_pem: String,
}

impl CertificateAuthority<'_> {
    pub fn from_ca_cert_pem(ca_cert_pem: &str, ca_key_pair: &str) -> Result<Self, CAError> {
        let key_pair = KeyPair::from_pem(ca_key_pair)?;
        let cert_der = CertificateDer::from_pem_slice(ca_cert_pem.as_bytes())
            .map_err(|e| CAError::PemParsingError(e.to_string()))?;
        let issuer = Issuer::from_ca_cert_der(&cert_der, key_pair)?;
        Ok(CertificateAuthority {
            issuer,
            cert_pem: ca_cert_pem.to_string(),
        })
    }

    pub fn from_key_cert_params(
        key_pair: KeyPair,
        ca_cert_params: CertificateParams,
    ) -> Result<Self, CAError> {
        let cert = ca_cert_params.self_signed(&key_pair)?;
        let issuer = Issuer::new(ca_cert_params, key_pair);
        Ok(CertificateAuthority {
            issuer,
            cert_pem: cert.pem(),
        })
    }

    pub fn new() -> Result<Self, CAError> {
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

    pub fn sign_csr(&self, csr: &CertificateSigningRequestParams) -> Result<Certificate, CAError> {
        let cert = csr.signed_by(&self.issuer)?;
        Ok(cert)
    }

    pub fn save(&self) {}
}

fn main() {
    let ca = CertificateAuthority::new().unwrap();

    let key = ca.issuer.key();

    let pem = ca.cert_pem;

    let ca = CertificateAuthority::from_ca_cert_pem(&pem, &key.serialize_pem()).unwrap();

    println!("Loaded CA: {:?}", ca.cert_pem);
    // let der_key = CertificateDer::from_slice(&pem);

    // let mut cert_params = CertificateParams::new(vec![
    //     "example.com".to_string(),
    //     "www.example.com".to_string(),
    // ])
    // .unwrap();

    // cert_params
    //     .distinguished_name
    //     .push(rcgen::DnType::CommonName, "example.com");
    // cert_params
    //     .distinguished_name
    //     .push(rcgen::DnType::OrganizationName, "Example Org");

    // let cert_key_pair = KeyPair::generate().unwrap();

    // let cert = cert_params.serialize_request(&cert_key_pair).unwrap();

    // let pem = cert.pem().unwrap();

    // let csr = CertificateSigningRequestParams::from_pem(&pem).unwrap();

    // let cert = ca.sign_csr(&csr).unwrap();

    // println!("CSR PEM:\n{:?}", cert);

    // let mut ca_params = CertificateParams::new(vec!["My CA".to_string()]).unwrap();

    // // Configure as a Certificate Authority
    // ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    // ca_params
    //     .distinguished_name
    //     .push(rcgen::DnType::OrganizationName, "My Organization");
    // ca_params
    //     .distinguished_name
    //     .push(rcgen::DnType::CommonName, "My Root CA");

    // // Generate key pair for CA
    // let ca_key_pair = KeyPair::generate().unwrap();

    // // Create the CA certificate
    // let ca_cert = ca_params.self_signed(&ca_key_pair).unwrap();

    // println!("=== CA Certificate ===");
    // println!("{}", ca_cert.pem());
    // println!("\n=== CA Private Key ===");
    // println!("{}", ca_key_pair.serialize_pem());

    // let test_ca_string = ca_cert.pem();

    // let ca = Issuer::from_ca_cert_pem(&test_ca_string, &ca_key_pair).unwrap();
    // println!("Loaded CA: {:?}", ca);

    // let ca_cert_pem = ca_cert.pem();
    // std::fs::write("ca_certificate.pem", &ca_cert_pem)
    //     .expect("Failed to write CA certificate to file");

    // let mut cert_params = CertificateParams::new(vec![
    //     "example.com".to_string(),
    //     "www.example.com".to_string(),
    // ])
    // .unwrap();

    // cert_params
    //     .distinguished_name
    //     .push(rcgen::DnType::CommonName, "example.com");
    // cert_params
    //     .distinguished_name
    //     .push(rcgen::DnType::OrganizationName, "Example Org");

    // let cert_key_pair = KeyPair::generate().unwrap();

    // let issuer = Issuer::from_params(&ca_params, &ca_key_pair);

    // // Sign the certificate with the CA
    // // let signed_cert = cert_params.signed_by(&cert_key_pair, &issuer).unwrap();
    // let signed_cert = cert_params.self_signed(&cert_key_pair).unwrap();

    // println!("\n=== Signed Certificate ===");
    // println!("{}", signed_cert.pem());
    // println!("\n=== Certificate Private Key ===");
    // println!("{}", cert_key_pair.serialize_pem());
}
