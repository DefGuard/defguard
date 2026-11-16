use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use anyhow::{Context, bail};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use x509_parser::{
    certificate::X509Certificate, parse_x509_certificate, public_key::PublicKey,
    x509::AttributeTypeAndValue,
};

/// TCG vendor IDs used by tpm-ca-certificates and folder names used by TrustedTpm.cab, mapped to
/// one display name per manufacturer.
const VENDOR_ALIASES: [(&str, &str); 20] = [
    ("AMD", "AMD"),
    ("ATML", "Atmel"),
    ("Atmel", "Atmel"),
    ("GOOG", "Google"),
    ("IFX", "Infineon"),
    ("Infineon", "Infineon"),
    ("INTC", "Intel"),
    ("Intel", "Intel"),
    ("MSFT", "Microsoft"),
    ("Microsoft", "Microsoft"),
    ("NSG", "NSING"),
    ("NTC", "Nuvoton"),
    ("Nuvoton", "Nuvoton"),
    ("NTZ", "Nationz"),
    ("NationZ", "Nationz"),
    ("QC", "Qualcomm"),
    ("QCOM", "Qualcomm"),
    ("STM", "STMicroelectronics"),
    ("STMicro", "STMicroelectronics"),
    ("STMicroelectronics", "STMicroelectronics"),
];

const EC_CURVES: [(&str, &str); 4] = [
    ("1.2.840.10045.3.1.7", "ECC P-256"),
    ("1.3.132.0.34", "ECC P-384"),
    ("1.3.132.0.35", "ECC P-521"),
    ("1.2.156.10197.1.301", "SM2"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Root,
    Intermediate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    TpmCaCertificates,
    Microsoft,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TpmCaCertificates => "tpm-ca-certificates",
            Self::Microsoft => "microsoft",
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    pub generated_at: DateTime<Utc>,
    pub tpm_ca_certificates: TpmCaCertificatesRelease,
    pub microsoft_trusted_tpm: MicrosoftTrustedTpm,
    pub certificates: Vec<Certificate>,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TpmCaCertificatesRelease {
    pub tag: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrosoftTrustedTpm {
    pub cab_sha256: String,
    pub last_updated: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Certificate {
    pub kind: Kind,
    pub vendor: String,
    pub common_name: String,
    pub organization: Option<String>,
    pub subject: String,
    pub issuer: String,
    pub serial_number: String,
    pub self_signed: bool,
    pub key_algorithm: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub sha256: String,
    pub spki_sha256: String,
    pub sources: Vec<Source>,
    pub der: String,
}

impl fmt::Display for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            Kind::Root => "root",
            Kind::Intermediate => "intermediate",
        };
        write!(
            f,
            "{kind} {} | {} | {}",
            self.vendor, self.common_name, self.sha256
        )
    }
}

pub struct SourceCertificate {
    pub kind: Kind,
    pub vendor: String,
    pub der: Vec<u8>,
    pub origin: String,
}

#[derive(Default)]
pub struct Vendors {
    unknown: BTreeSet<String>,
}

impl Vendors {
    pub fn canonical(&mut self, name: &str) -> String {
        if let Some((_, canonical)) = VENDOR_ALIASES
            .iter()
            .find(|(alias, _)| alias.eq_ignore_ascii_case(name))
        {
            return (*canonical).to_string();
        }
        if self.unknown.insert(name.to_string()) {
            println!("warning: unknown TPM vendor {name:?}, add it to VENDOR_ALIASES");
        }
        name.to_string()
    }
}

fn name_attr<'a, I>(mut attrs: I) -> Option<String>
where
    I: Iterator<Item = &'a AttributeTypeAndValue<'a>>,
{
    attrs
        .find_map(|attr| attr.as_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn key_algorithm(cert: &X509Certificate) -> String {
    let spki = cert.public_key();
    match spki.parsed() {
        Ok(PublicKey::RSA(key)) => format!("RSA {}", key.key_size()),
        Ok(PublicKey::EC(_)) => {
            let curve = spki
                .algorithm
                .parameters
                .as_ref()
                .and_then(|params| params.as_oid().ok())
                .map(|oid| oid.to_id_string());
            match curve {
                Some(curve) => EC_CURVES
                    .iter()
                    .find(|(oid, _)| *oid == curve)
                    .map_or_else(|| format!("EC {curve}"), |(_, name)| (*name).to_string()),
                None => "EC".to_string(),
            }
        }
        _ => spki.algorithm.algorithm.to_id_string(),
    }
}

fn timestamp(seconds: i64) -> anyhow::Result<DateTime<Utc>> {
    DateTime::from_timestamp(seconds, 0).context("certificate validity is out of range")
}

pub fn describe(cert: &SourceCertificate, source: Source) -> anyhow::Result<Certificate> {
    let (rest, parsed) = parse_x509_certificate(&cert.der)
        .map_err(|err| anyhow::anyhow!("{}: invalid certificate: {err}", cert.origin))?;
    // trailing bytes are dropped so the stored DER and its SHA-256 are canonical
    let der = &cert.der[..cert.der.len() - rest.len()];
    let subject = parsed.subject().to_string();
    let common_name = name_attr(parsed.subject().iter_common_name()).unwrap_or(subject.clone());
    let serial_number = parsed
        .raw_serial()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();

    Ok(Certificate {
        kind: cert.kind,
        vendor: cert.vendor.clone(),
        common_name,
        organization: name_attr(parsed.subject().iter_organization()),
        subject,
        issuer: parsed.issuer().to_string(),
        serial_number,
        self_signed: parsed.subject().as_raw() == parsed.issuer().as_raw(),
        key_algorithm: key_algorithm(&parsed),
        not_before: timestamp(parsed.validity().not_before.timestamp())?,
        not_after: timestamp(parsed.validity().not_after.timestamp())?,
        sha256: sha256::digest(der),
        spki_sha256: sha256::digest(parsed.public_key().raw),
        sources: vec![source],
        der: BASE64_STANDARD.encode(der),
    })
}

#[derive(Default)]
pub struct Merger {
    certificates: BTreeMap<String, Certificate>,
}

impl Merger {
    pub fn add(&mut self, certificate: Certificate) {
        let Some(existing) = self.certificates.get_mut(&certificate.sha256) else {
            self.certificates
                .insert(certificate.sha256.clone(), certificate);
            return;
        };
        for source in certificate.sources {
            if !existing.sources.contains(&source) {
                existing.sources.push(source);
                existing.sources.sort();
            }
        }
        if existing.kind != certificate.kind {
            println!(
                "note: sources disagree on the kind of {}, keeping it as a root",
                existing.common_name
            );
            existing.kind = Kind::Root;
        }
        if existing.vendor != certificate.vendor {
            println!(
                "note: sources disagree on the vendor of {} ({} vs {}), keeping {}",
                existing.common_name, existing.vendor, certificate.vendor, existing.vendor
            );
        }
    }

    #[must_use]
    pub fn finish(self) -> Vec<Certificate> {
        let mut certificates: Vec<_> = self.certificates.into_values().collect();
        certificates
            .sort_by(|a, b| (a.kind, &a.vendor, &a.sha256).cmp(&(b.kind, &b.vendor, &b.sha256)));
        certificates
    }
}

pub struct Diff<'a> {
    pub added: Vec<&'a Certificate>,
    pub removed: Vec<&'a Certificate>,
    pub changed: Vec<&'a Certificate>,
}

#[must_use]
pub fn diff<'a>(current: &'a [Certificate], candidate: &'a [Certificate]) -> Diff<'a> {
    let current_by_hash: BTreeMap<_, _> = current.iter().map(|c| (&c.sha256, c)).collect();
    let candidate_hashes: BTreeSet<_> = candidate.iter().map(|c| &c.sha256).collect();
    let mut added = Vec::new();
    let mut changed = Vec::new();
    for certificate in candidate {
        match current_by_hash.get(&certificate.sha256) {
            None => added.push(certificate),
            Some(existing) if *existing != certificate => changed.push(certificate),
            Some(_) => {}
        }
    }
    let removed = current
        .iter()
        .filter(|c| !candidate_hashes.contains(&c.sha256))
        .collect();
    Diff {
        added,
        removed,
        changed,
    }
}

pub fn ensure_has_roots(certificates: &[Certificate]) -> anyhow::Result<()> {
    if !certificates.iter().any(|c| c.kind == Kind::Root) {
        bail!("the merged bundle contains no root certificates");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn certificate(sha256: &str, kind: Kind, source: Source) -> Certificate {
        Certificate {
            kind,
            vendor: "Infineon".into(),
            common_name: format!("CA {sha256}"),
            organization: None,
            subject: String::new(),
            issuer: String::new(),
            serial_number: "01".into(),
            self_signed: false,
            key_algorithm: "RSA 2048".into(),
            not_before: DateTime::UNIX_EPOCH,
            not_after: DateTime::UNIX_EPOCH,
            sha256: sha256.into(),
            spki_sha256: sha256.into(),
            sources: vec![source],
            der: String::new(),
        }
    }

    #[test]
    fn merge_dedupes_and_prefers_root() {
        let mut merger = Merger::default();
        merger.add(certificate("b", Kind::Intermediate, Source::Microsoft));
        merger.add(certificate("a", Kind::Intermediate, Source::Microsoft));
        merger.add(certificate("b", Kind::Root, Source::TpmCaCertificates));
        let merged = merger.finish();

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].sha256, "b");
        assert_eq!(merged[0].kind, Kind::Root);
        assert_eq!(
            merged[0].sources,
            [Source::TpmCaCertificates, Source::Microsoft]
        );
        assert_eq!(merged[1].sha256, "a");
    }

    #[test]
    fn diff_reports_added_removed_and_changed() {
        let current = [
            certificate("a", Kind::Root, Source::Microsoft),
            certificate("b", Kind::Root, Source::Microsoft),
        ];
        let candidate = [
            certificate("b", Kind::Root, Source::TpmCaCertificates),
            certificate("c", Kind::Root, Source::Microsoft),
        ];
        let diff = diff(&current, &candidate);

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].sha256, "c");
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].sha256, "a");
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].sha256, "b");
    }

    #[test]
    fn vendor_aliases_are_case_insensitive() {
        let mut vendors = Vendors::default();
        assert_eq!(vendors.canonical("IFX"), "Infineon");
        assert_eq!(vendors.canonical("stmicro"), "STMicroelectronics");
        assert_eq!(vendors.canonical("NationZ"), "Nationz");
        assert_eq!(vendors.canonical("Acme"), "Acme");
    }
}
