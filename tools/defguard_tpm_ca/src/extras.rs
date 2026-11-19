use anyhow::{Context, anyhow, ensure};
use base64::{Engine, prelude::BASE64_STANDARD};
use rustls_pki_types::{CertificateDer, pem::PemObject};
use x509_parser::parse_x509_certificate;

use crate::bundle::{Certificate, Kind, Source, SourceCertificate, Vendors};

/// Intermediates missing from both upstream sources. Only intermediates are allowed, because
/// their signature proves them against an upstream CA wherever the copy came from, while an
/// extra root would rest on nothing but the copy's provenance.
struct Extra {
    file: &'static str,
    pem: &'static [u8],
    vendor: &'static str,
    sha256: &'static str,
}

macro_rules! extra {
    ($file:literal, $vendor:literal, $sha256:literal) => {
        Extra {
            file: $file,
            pem: include_cert!($file),
            vendor: $vendor,
            sha256: $sha256,
        }
    };
}

// copies from IBM TSS utils/certificates, issued by IFX TPM EK Root CA
const EXTRAS: [Extra; 4] = [
    extra!(
        "ifx_tpm_ek_intermediate_ca_48.pem",
        "IFX",
        "3e7e1adda311b09e1852b3e14113962f30482d9c62c5d46d49736b7584c7890a"
    ),
    extra!(
        "ifx_tpm_ek_intermediate_ca_49.pem",
        "IFX",
        "6dbe4ce5a73c2155a6bcd7437ad1100af6c648151bce32ca5f056cb3e8a9f578"
    ),
    extra!(
        "ifx_tpm_ek_intermediate_ca_54.pem",
        "IFX",
        "72e9d37d3ef49f2a0afdd4f7c93e1ef8be7013a0ea81d10c7a26cad192d01727"
    ),
    extra!(
        "ifx_tpm_ek_intermediate_ca_62.pem",
        "IFX",
        "ed9758fec62fc60fbdc0c661bb40850025223f01e853351a0c14139f0ad3d807"
    ),
];

pub fn extra_certificates(vendors: &mut Vendors) -> anyhow::Result<Vec<SourceCertificate>> {
    EXTRAS
        .iter()
        .map(|extra| {
            let der = CertificateDer::from_pem_slice(extra.pem)
                .with_context(|| format!("certs/{} is not a PEM certificate", extra.file))?;
            ensure!(
                sha256::digest(der.as_ref()) == extra.sha256,
                "certs/{} does not match its pinned SHA-256",
                extra.file
            );
            Ok(SourceCertificate {
                kind: Kind::Intermediate,
                vendor: vendors.canonical(extra.vendor),
                der: der.to_vec(),
                origin: format!("certs/{}", extra.file),
            })
        })
        .collect()
}

/// Every extra must be signed by a CA from an upstream source, so a chain through it always ends
/// at an upstream root.
pub fn check_extras(certificates: &[Certificate]) -> anyhow::Result<()> {
    let upstream: Vec<_> = certificates
        .iter()
        .filter(|c| c.sources.iter().any(|source| *source != Source::Defguard))
        .collect();
    for extra in certificates
        .iter()
        .filter(|c| c.sources.contains(&Source::Defguard))
    {
        if extra.sources.len() > 1 {
            println!(
                "note: extra {} is now provided upstream, remove it from extras",
                extra.common_name
            );
            continue;
        }
        let der = BASE64_STANDARD.decode(&extra.der)?;
        let (_, parsed) = parse_x509_certificate(&der)
            .map_err(|err| anyhow!("extra {}: invalid certificate: {err}", extra.common_name))?;
        let issuer = upstream
            .iter()
            .filter(|c| c.subject == extra.issuer)
            .find_map(|issuer| {
                let der = BASE64_STANDARD.decode(&issuer.der).ok()?;
                let (_, issuer_parsed) = parse_x509_certificate(&der).ok()?;
                parsed
                    .verify_signature(Some(issuer_parsed.public_key()))
                    .is_ok()
                    .then_some(*issuer)
            });
        let issuer = issuer.with_context(|| {
            format!(
                "extra {} is not signed by any upstream CA named {}",
                extra.common_name, extra.issuer
            )
        })?;
        println!(
            "Verified extra {} against {} ({})",
            extra.common_name, issuer.common_name, issuer.sha256
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair};

    use super::*;
    use crate::bundle::describe;

    fn ca_params(common_name: &str) -> CertificateParams {
        let mut params = CertificateParams::new(Vec::new()).unwrap();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params
            .distinguished_name
            .push(DnType::CommonName, common_name);
        params
    }

    fn certificate(der: &[u8], kind: Kind, source: Source) -> Certificate {
        let source_certificate = SourceCertificate {
            kind,
            vendor: "Infineon".into(),
            der: der.to_vec(),
            origin: "test".into(),
        };
        describe(&source_certificate, source).unwrap()
    }

    fn chain(forged: bool) -> Vec<Certificate> {
        let root_key = KeyPair::generate().unwrap();
        let forger_key = KeyPair::generate().unwrap();
        let root_params = ca_params("Root");
        let root = root_params.self_signed(&root_key).unwrap();
        let signer = if forged { &forger_key } else { &root_key };
        let intermediate = ca_params("Intermediate")
            .signed_by(
                &KeyPair::generate().unwrap(),
                &Issuer::from_params(&root_params, signer),
            )
            .unwrap();
        vec![
            certificate(root.der(), Kind::Root, Source::Microsoft),
            certificate(intermediate.der(), Kind::Intermediate, Source::Defguard),
        ]
    }

    #[test]
    fn extras_match_their_pins() {
        let certificates = extra_certificates(&mut Vendors::default()).unwrap();
        assert_eq!(certificates.len(), EXTRAS.len());
        assert!(
            certificates
                .iter()
                .all(|c| c.kind == Kind::Intermediate && c.vendor == "Infineon")
        );
    }

    #[test]
    fn extras_must_be_signed_by_an_upstream_ca() {
        assert!(check_extras(&chain(false)).is_ok());
        assert!(check_extras(&chain(true)).is_err());

        let mut issued_by_extra = chain(false);
        issued_by_extra[0].sources = vec![Source::Defguard];
        assert!(check_extras(&issued_by_extra).is_err());
    }
}
