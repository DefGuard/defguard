//! Regenerates defguard_core/src/enterprise/tpm/tpm_ca_certificates.json, the TPM manufacturer CA
//! bundle compiled into Core, from tpm-ca-certificates releases and Microsoft TrustedTpm.cab.
//!
//! Both sources are signature checked before use, which needs cosign and osslsigncode.

mod bundle;
mod sources;

use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

use anyhow::{Context, bail, ensure};
use chrono::{DateTime, Utc};
use clap::Parser;

use crate::{
    bundle::{
        Bundle, Kind, Merger, MicrosoftTrustedTpm, Source, TpmCaCertificatesRelease, Vendors,
        describe, diff, ensure_has_roots,
    },
    sources::{Http, Verifiers, fetch_microsoft_trusted_tpm, fetch_tpm_ca_certificates},
};

const DEFAULT_OUTPUT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/defguard_core/src/enterprise/tpm/tpm_ca_certificates.json"
);

/// Fetch, verify and merge the TPM manufacturer CA certificates compiled into Defguard Core.
#[derive(Parser)]
struct Args {
    /// Bundle file to update.
    #[arg(default_value = DEFAULT_OUTPUT)]
    output: PathBuf,
    /// Accept certificates that disappeared upstream. Review the removal list first.
    #[arg(long)]
    allow_removals: bool,
    /// cosign binary used to verify tpm-ca-certificates releases.
    #[arg(long, env = "COSIGN", default_value = "cosign")]
    cosign: PathBuf,
    /// osslsigncode binary used to verify the Authenticode signature of TrustedTpm.cab.
    #[arg(long, env = "OSSLSIGNCODE", default_value = "osslsigncode")]
    osslsigncode: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    validate_output(&args.output)?;
    let current = read_current(&args.output)?;
    let verifiers = Verifiers::new(args.cosign, args.osslsigncode)?;
    let http = Http::new()?;
    let workdir = tempfile::tempdir()?;
    let mut vendors = Vendors::default();

    let tpm_ca = fetch_tpm_ca_certificates(&http, &verifiers, workdir.path(), &mut vendors)?;
    if let Some(current) = &current {
        ensure!(
            tpm_ca.tag >= current.tpm_ca_certificates.tag,
            "tpm-ca-certificates release {} is older than the current bundle ({})",
            tpm_ca.tag,
            current.tpm_ca_certificates.tag
        );
    }
    let microsoft = fetch_microsoft_trusted_tpm(&http, &verifiers, workdir.path(), &mut vendors)?;

    let mut merger = Merger::default();
    for certificate in &tpm_ca.certificates {
        merger.add(describe(certificate, Source::TpmCaCertificates)?);
    }
    for certificate in &microsoft.certificates {
        merger.add(describe(certificate, Source::Microsoft)?);
    }
    let certificates = merger.finish();
    ensure_has_roots(&certificates)?;

    let candidate = Bundle {
        generated_at: DateTime::from_timestamp(Utc::now().timestamp(), 0)
            .context("system clock is out of range")?,
        tpm_ca_certificates: TpmCaCertificatesRelease { tag: tpm_ca.tag },
        microsoft_trusted_tpm: MicrosoftTrustedTpm {
            cab_sha256: microsoft.cab_sha256,
            last_updated: microsoft.last_updated,
        },
        certificates,
    };

    if let Some(current) = &current {
        let diff = diff(&current.certificates, &candidate.certificates);
        for (label, certificates) in [
            ("added", &diff.added),
            ("removed", &diff.removed),
            ("changed", &diff.changed),
        ] {
            for certificate in certificates {
                println!("{label}: {certificate}");
            }
        }
        if !diff.removed.is_empty() && !args.allow_removals {
            bail!(
                "{} certificates were removed upstream, review them and rerun with \
                 --allow-removals",
                diff.removed.len()
            );
        }
        let unchanged = diff.added.is_empty()
            && diff.removed.is_empty()
            && diff.changed.is_empty()
            && current.tpm_ca_certificates == candidate.tpm_ca_certificates
            && current.microsoft_trusted_tpm == candidate.microsoft_trusted_tpm;
        if unchanged {
            println!("{} is up to date", args.output.display());
            return Ok(());
        }
    }

    let mut json = serde_json::to_string_pretty(&candidate)?;
    json.push('\n');
    fs::write(&args.output, json)
        .with_context(|| format!("failed to write {}", args.output.display()))?;
    print_summary(&candidate, &args.output);
    Ok(())
}

fn read_current(output: &Path) -> anyhow::Result<Option<Bundle>> {
    if !output.exists() {
        return Ok(None);
    }
    let current =
        fs::read(output).with_context(|| format!("failed to read {}", output.display()))?;
    serde_json::from_slice(&current)
        .map(Some)
        .with_context(|| format!("failed to parse {}", output.display()))
}

fn print_summary(bundle: &Bundle, output: &Path) {
    let mut counts: BTreeMap<(Kind, Source), usize> = BTreeMap::new();
    for certificate in &bundle.certificates {
        for source in &certificate.sources {
            *counts.entry((certificate.kind, *source)).or_default() += 1;
        }
    }
    let roots = bundle
        .certificates
        .iter()
        .filter(|c| c.kind == Kind::Root)
        .count();
    println!(
        "Wrote {} roots and {} intermediates to {}",
        roots,
        bundle.certificates.len() - roots,
        output.display()
    );
    for ((kind, source), count) in counts {
        println!("  {kind:?} from {source}: {count}");
    }
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
