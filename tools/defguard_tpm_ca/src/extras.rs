use anyhow::{Context, ensure};
use rustls_pki_types::{CertificateDer, pem::PemObject};

use crate::bundle::{Certificate, Kind, Source, SourceCertificate, Vendors};

/// Certificates missing from both upstream sources, reviewed by hand and pinned by the SHA-256
/// of their DER.
struct Extra {
    file: &'static str,
    pem: &'static [u8],
    vendor: &'static str,
    kind: Kind,
    sha256: &'static str,
}

macro_rules! extra {
    ($file:literal, $vendor:literal, $kind:expr, $sha256:literal) => {
        Extra {
            file: $file,
            pem: include_bytes!(concat!("../extras/", $file)),
            vendor: $vendor,
            kind: $kind,
            sha256: $sha256,
        }
    };
}

const EXTRAS: [Extra; 5] = [
    // shipped identically by Keylime tpm_cert_store and IBM TSS utils/certificates
    extra!(
        "nuvoton_tpm_root_ca_0100.pem",
        "NTC",
        Kind::Root,
        "887bc9658ffc6ad7f4eeb85c8819772ff609ee64c4dac4a6dca654a39e94b289"
    ),
    // Infineon downloads mirrored by IBM TSS, issued by IFX TPM EK Root CA
    extra!(
        "ifx_tpm_ek_intermediate_ca_48.pem",
        "IFX",
        Kind::Intermediate,
        "3e7e1adda311b09e1852b3e14113962f30482d9c62c5d46d49736b7584c7890a"
    ),
    extra!(
        "ifx_tpm_ek_intermediate_ca_49.pem",
        "IFX",
        Kind::Intermediate,
        "6dbe4ce5a73c2155a6bcd7437ad1100af6c648151bce32ca5f056cb3e8a9f578"
    ),
    extra!(
        "ifx_tpm_ek_intermediate_ca_54.pem",
        "IFX",
        Kind::Intermediate,
        "72e9d37d3ef49f2a0afdd4f7c93e1ef8be7013a0ea81d10c7a26cad192d01727"
    ),
    extra!(
        "ifx_tpm_ek_intermediate_ca_62.pem",
        "IFX",
        Kind::Intermediate,
        "ed9758fec62fc60fbdc0c661bb40850025223f01e853351a0c14139f0ad3d807"
    ),
];

pub fn extra_certificates(vendors: &mut Vendors) -> anyhow::Result<Vec<SourceCertificate>> {
    EXTRAS
        .iter()
        .map(|extra| {
            let der = CertificateDer::from_pem_slice(extra.pem)
                .with_context(|| format!("extras/{} is not a PEM certificate", extra.file))?;
            ensure!(
                sha256::digest(der.as_ref()) == extra.sha256,
                "extras/{} does not match its pinned SHA-256",
                extra.file
            );
            Ok(SourceCertificate {
                kind: extra.kind,
                vendor: vendors.canonical(extra.vendor),
                der: der.to_vec(),
                origin: format!("extras/{}", extra.file),
            })
        })
        .collect()
}

/// An extra intermediate whose issuer is not bundled can never chain to a trusted root.
pub fn check_extras(certificates: &[Certificate]) -> anyhow::Result<()> {
    for extra in certificates
        .iter()
        .filter(|c| c.sources.contains(&Source::Defguard))
    {
        if extra.sources.len() > 1 {
            println!(
                "note: extra {} is now provided upstream, remove it from extras",
                extra.common_name
            );
        }
        ensure!(
            extra.kind == Kind::Root || certificates.iter().any(|c| c.subject == extra.issuer),
            "extra {} is issued by {}, which is not in the bundle",
            extra.common_name,
            extra.issuer
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extras_match_their_pins() {
        let certificates = extra_certificates(&mut Vendors::default()).unwrap();
        assert_eq!(certificates.len(), EXTRAS.len());
        assert!(
            certificates
                .iter()
                .all(|c| c.vendor == "Nuvoton" || c.vendor == "Infineon")
        );
    }
}
