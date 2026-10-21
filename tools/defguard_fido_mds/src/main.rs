//! Regenerates `defguard_core/src/fido2/trusted_authenticators.json` from the FIDO Metadata
//! Service (MDS3) blob, keeping only what the hardware key attestation check needs.

use std::{
    env,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

use anyhow::{Context, bail, ensure};
use base64::{
    Engine,
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD},
};
use reqwest::blocking::{self, Response};
use rustls_pki_types::{CertificateDer, ServerName, UnixTime, pem::PemObject};
use serde::{Deserialize, Serialize};
use webpki::{
    ALL_VERIFICATION_ALGS, EndEntityCert, KeyUsage, anchor_from_trusted_cert,
    ring::RSA_PKCS1_2048_8192_SHA256,
};

const MDS_URL: &str = "https://mds3.fidoalliance.org/";
const MDS_SIGNER_NAME: &str = "mds.fidoalliance.org";
const MDS_ROOT_PEM: &[u8] = include_bytes!("../globalsign_root_r3.pem");
const DEFAULT_OUTPUT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/defguard_core/src/fido2/trusted_authenticators.json"
);

const TRUSTED_ATTESTATION_TYPES: [&str; 2] = ["basic_full", "attca"];
const ROAMING_ATTACHMENT_HINTS: [&str; 4] = ["external", "wired", "wireless", "nfc"];
const DISQUALIFYING_STATUSES: [&str; 5] = [
    "REVOKED",
    "USER_VERIFICATION_BYPASS",
    "ATTESTATION_KEY_COMPROMISE",
    "USER_KEY_REMOTE_COMPROMISE",
    "USER_KEY_PHYSICAL_COMPROMISE",
];

#[derive(Deserialize)]
struct JwsHeader {
    alg: String,
    x5c: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataBlob {
    no: u64,
    next_update: String,
    entries: Vec<MetadataEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataEntry {
    aaguid: Option<String>,
    metadata_statement: MetadataStatement,
    status_reports: Vec<StatusReport>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataStatement {
    description: String,
    protocol_family: String,
    #[serde(default)]
    attestation_types: Vec<String>,
    #[serde(default)]
    key_protection: Vec<String>,
    #[serde(default)]
    attachment_hint: Vec<String>,
    #[serde(default)]
    attestation_root_certificates: Vec<String>,
}

#[derive(Deserialize)]
struct StatusReport {
    status: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    no: u64,
    next_update: String,
    models: Vec<CatalogModel>,
}

#[derive(Deserialize, Serialize)]
struct CatalogModel {
    aaguid: String,
    description: String,
    roots: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let output = env::args_os()
        .nth(1)
        .map_or_else(|| PathBuf::from(DEFAULT_OUTPUT), PathBuf::from);
    validate_output(&output)?;

    println!("Downloading {MDS_URL}");
    let token = blocking::get(MDS_URL)
        .and_then(Response::error_for_status)
        .and_then(Response::text)
        .context("failed to download the MDS blob")?;
    let payload = verify_jws(token.trim())?;
    let blob: MetadataBlob =
        serde_json::from_slice(&payload).context("failed to parse the MDS blob payload")?;

    if let Ok(current) = fs::read(&output) {
        let current: Catalog = serde_json::from_slice(&current)
            .with_context(|| format!("failed to parse {}", output.display()))?;
        ensure!(
            blob.no >= current.no,
            "downloaded MDS blob #{} is older than the current list (#{})",
            blob.no,
            current.no
        );
    }

    let catalog = Catalog {
        no: blob.no,
        next_update: blob.next_update,
        models: blob.entries.into_iter().filter_map(trusted_model).collect(),
    };
    ensure!(
        !catalog.models.is_empty(),
        "no authenticators passed the filter"
    );

    let mut json = serde_json::to_string_pretty(&catalog)?;
    json.push('\n');
    fs::write(&output, json).with_context(|| format!("failed to write {}", output.display()))?;
    println!(
        "Wrote {} models from MDS blob #{} (next update {}) to {}",
        catalog.models.len(),
        catalog.no,
        catalog.next_update,
        output.display()
    );
    Ok(())
}

fn validate_output(output: &Path) -> anyhow::Result<()> {
    let dir = output
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    ensure!(
        dir.is_dir(),
        "output directory {} does not exist",
        dir.display()
    );
    ensure!(
        !output.is_dir(),
        "output path {} is a directory",
        output.display()
    );

    let not_writable = || format!("{} is not writable", output.display());
    if output.exists() {
        OpenOptions::new()
            .append(true)
            .open(output)
            .with_context(not_writable)?;
    } else {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output)
            .with_context(not_writable)?;
        fs::remove_file(output).with_context(not_writable)?;
    }
    Ok(())
}

fn verify_jws(token: &str) -> anyhow::Result<Vec<u8>> {
    let Some((signing_input, signature)) = token.rsplit_once('.') else {
        bail!("MDS blob is not a compact JWS");
    };
    let Some((header, payload)) = signing_input.split_once('.') else {
        bail!("MDS blob is not a compact JWS");
    };

    let header: JwsHeader = serde_json::from_slice(&BASE64_URL_SAFE_NO_PAD.decode(header)?)?;
    ensure!(
        header.alg == "RS256",
        "unexpected MDS signature algorithm {}",
        header.alg
    );
    let chain = header
        .x5c
        .iter()
        .map(|cert| BASE64_STANDARD.decode(cert).map(CertificateDer::from))
        .collect::<Result<Vec<_>, _>>()?;
    let (signer, intermediates) = chain
        .split_first()
        .context("MDS blob has no signing certificate")?;

    let root = CertificateDer::from_pem_slice(MDS_ROOT_PEM)?;
    let anchors = [anchor_from_trusted_cert(&root)?];
    let signer = EndEntityCert::try_from(signer)?;
    signer
        .verify_for_usage(
            ALL_VERIFICATION_ALGS,
            &anchors,
            intermediates,
            UnixTime::now(),
            KeyUsage::server_auth(),
            None,
            None,
        )
        .context("MDS signing certificate does not chain to GlobalSign Root CA - R3")?;
    signer.verify_is_valid_for_subject_name(&ServerName::try_from(MDS_SIGNER_NAME)?)?;
    signer
        .verify_signature(
            RSA_PKCS1_2048_8192_SHA256,
            signing_input.as_bytes(),
            &BASE64_URL_SAFE_NO_PAD.decode(signature)?,
        )
        .context("MDS blob signature is invalid")?;

    Ok(BASE64_URL_SAFE_NO_PAD.decode(payload)?)
}

fn trusted_model(entry: MetadataEntry) -> Option<CatalogModel> {
    let aaguid = entry.aaguid?;
    let statement = entry.metadata_statement;
    let has_any = |values: &[String], wanted: &[&str]| {
        values.iter().any(|value| wanted.contains(&value.as_str()))
    };

    let certified = entry
        .status_reports
        .iter()
        .any(|report| report.status.starts_with("FIDO_CERTIFIED"));
    let compromised = entry
        .status_reports
        .iter()
        .any(|report| DISQUALIFYING_STATUSES.contains(&report.status.as_str()));

    let trusted = statement.protocol_family == "fido2"
        && certified
        && !compromised
        && has_any(&statement.attestation_types, &TRUSTED_ATTESTATION_TYPES)
        && has_any(&statement.key_protection, &["hardware"])
        && has_any(&statement.attachment_hint, &ROAMING_ATTACHMENT_HINTS)
        && !has_any(&statement.attachment_hint, &["internal"])
        && !statement.attestation_root_certificates.is_empty();

    trusted.then_some(CatalogModel {
        aaguid,
        description: statement.description,
        roots: statement.attestation_root_certificates,
    })
}
