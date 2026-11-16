use std::{
    collections::BTreeMap,
    env, fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, bail, ensure};
use base64::{Engine, prelude::BASE64_STANDARD};
use chrono::NaiveDate;
use reqwest::{
    blocking::{Client, Response},
    header::{ACCEPT, AUTHORIZATION},
};
use rustls_pki_types::{CertificateDer, pem::PemObject};
use serde::Deserialize;

use crate::bundle::{Kind, SourceCertificate, Vendors};

const TPM_CA_RELEASES_URL: &str =
    "https://api.github.com/repos/loicsikidi/tpm-ca-certificates/releases?per_page=100";
const TPM_CA_RELEASE_WORKFLOW: &str =
    "https://github.com/loicsikidi/tpm-ca-certificates/.github/workflows/release-bundle.yaml";
const GITHUB_ACTIONS_ISSUER: &str = "https://token.actions.githubusercontent.com";
const CHECKSUMS: &str = "checksums.txt";
const CHECKSUMS_BUNDLE: &str = "checksums.txt.sigstore.json";
const ROOTS_PEM: &str = "tpm-ca-certificates.pem";
const INTERMEDIATES_PEM: &str = "tpm-intermediate-ca-certificates.pem";

const TRUSTED_TPM_CAB_URL: &str = "https://go.microsoft.com/fwlink/?linkid=2097925";
const MICROSOFT_SIGNER: &str =
    "/C=US/ST=Washington/L=Redmond/O=Microsoft Corporation/CN=Microsoft Corporation";
const MICROSOFT_ROOT_2011_PEM: &[u8] = include_bytes!("../microsoft_root_ca_2011.pem");
const MICROSOFT_ROOT_2010_PEM: &[u8] = include_bytes!("../microsoft_root_ca_2010.pem");
const MICROSOFT_CRL_PREFIXES: [&str; 2] =
    ["http://www.microsoft.com/", "https://www.microsoft.com/"];
const CERTIFICATE_EXTENSIONS: [&str; 4] = ["cer", "crt", "der", "pem"];
const SIGNER_CRL_MARKER: &str = "Ignored CRL distribution point:";
const TSA_CRL_MARKER: &str = "Ignored TSA's CRL distribution point:";

pub struct Verifiers {
    cosign: PathBuf,
    osslsigncode: PathBuf,
}

impl Verifiers {
    pub fn new(cosign: PathBuf, osslsigncode: PathBuf) -> anyhow::Result<Self> {
        for (binary, version_arg, flag, env) in [
            (&cosign, "version", "--cosign", "COSIGN"),
            (&osslsigncode, "--version", "--osslsigncode", "OSSLSIGNCODE"),
        ] {
            let status = Command::new(binary)
                .arg(version_arg)
                .output()
                .map(|output| output.status.success());
            ensure!(
                matches!(status, Ok(true)),
                "{} is required to verify the upstream signatures, install it or point to it \
                 with {flag} or the {env} environment variable",
                binary.display()
            );
        }
        Ok(Self {
            cosign,
            osslsigncode,
        })
    }
}

pub struct Http {
    client: Client,
    github_token: Option<String>,
}

impl Http {
    pub fn new() -> anyhow::Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("defguard_tpm_ca/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(120))
            .build()?;
        Ok(Self {
            client,
            github_token: env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()),
        })
    }

    fn get(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        println!("Downloading {url}");
        let bytes = self
            .client
            .get(url)
            .send()
            .and_then(Response::error_for_status)
            .and_then(Response::bytes)
            .with_context(|| format!("failed to download {url}"))?;
        Ok(bytes.to_vec())
    }

    fn github_api<T: for<'de> Deserialize<'de>>(&self, url: &str) -> anyhow::Result<T> {
        let mut request = self
            .client
            .get(url)
            .header(ACCEPT, "application/vnd.github+json");
        if let Some(token) = &self.github_token {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        request
            .send()
            .and_then(Response::error_for_status)
            .and_then(Response::json)
            .with_context(|| format!("GitHub API request to {url} failed"))
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<ReleaseAsset>,
}

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

pub struct TpmCaCertificates {
    pub tag: String,
    pub certificates: Vec<SourceCertificate>,
}

pub struct MicrosoftTrustedTpm {
    pub cab_sha256: String,
    pub last_updated: String,
    pub certificates: Vec<SourceCertificate>,
}

// the repo also publishes tpmtb CLI releases tagged vX.Y.Z, bundles are tagged by date
fn is_bundle_tag(tag: &str) -> bool {
    tag.len() == 10 && NaiveDate::parse_from_str(tag, "%Y-%m-%d").is_ok()
}

fn capture(command: &mut Command, what: &str) -> anyhow::Result<(bool, String)> {
    let output = command
        .output()
        .with_context(|| format!("failed to run {what}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok((output.status.success(), text))
}

pub fn fetch_tpm_ca_certificates(
    http: &Http,
    verifiers: &Verifiers,
    workdir: &Path,
    vendors: &mut Vendors,
) -> anyhow::Result<TpmCaCertificates> {
    let releases: Vec<Release> = http.github_api(TPM_CA_RELEASES_URL)?;
    let release = releases
        .into_iter()
        .filter(|r| !r.draft && !r.prerelease && is_bundle_tag(&r.tag_name))
        .max_by(|a, b| a.tag_name.cmp(&b.tag_name))
        .context("no tpm-ca-certificates bundle release found")?;
    println!("Using tpm-ca-certificates release {}", release.tag_name);

    let dir = workdir.join("tpm-ca-certificates");
    fs::create_dir(&dir)?;
    for name in [CHECKSUMS, CHECKSUMS_BUNDLE, ROOTS_PEM, INTERMEDIATES_PEM] {
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .with_context(|| format!("release {} has no {name}", release.tag_name))?;
        fs::write(dir.join(name), http.get(&asset.browser_download_url)?)?;
    }

    let identity = format!("{TPM_CA_RELEASE_WORKFLOW}@refs/tags/{}", release.tag_name);
    let (verified, output) = capture(
        Command::new(&verifiers.cosign)
            .current_dir(&dir)
            .args(["verify-blob", "--bundle", CHECKSUMS_BUNDLE])
            .args(["--certificate-identity", &identity])
            .args(["--certificate-oidc-issuer", GITHUB_ACTIONS_ISSUER])
            .arg(CHECKSUMS),
        "cosign",
    )?;
    ensure!(
        verified,
        "cosign could not verify {CHECKSUMS} of release {}:\n{output}",
        release.tag_name
    );
    println!("Verified {CHECKSUMS} signature (identity {identity})");

    let checksums = parse_checksums(&fs::read_to_string(dir.join(CHECKSUMS))?)?;
    let mut certificates = Vec::new();
    for (name, kind) in [
        (ROOTS_PEM, Kind::Root),
        (INTERMEDIATES_PEM, Kind::Intermediate),
    ] {
        let pem = fs::read(dir.join(name))?;
        let expected = checksums
            .get(name)
            .with_context(|| format!("{CHECKSUMS} does not list {name}"))?;
        ensure!(
            sha256::digest(pem.as_slice()) == *expected,
            "{name} does not match its checksum"
        );
        let pem = String::from_utf8(pem).with_context(|| format!("{name} is not UTF-8"))?;
        certificates.extend(parse_owned_pem(&pem, kind, name, vendors)?);
    }

    Ok(TpmCaCertificates {
        tag: release.tag_name,
        certificates,
    })
}

fn parse_checksums(text: &str) -> anyhow::Result<BTreeMap<String, String>> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (hash, name) = line
                .split_once(char::is_whitespace)
                .with_context(|| format!("malformed {CHECKSUMS} line {line:?}"))?;
            let name = name.trim_start().trim_start_matches('*');
            Ok((name.to_string(), hash.to_ascii_lowercase()))
        })
        .collect()
}

/// Reads tpm-ca-certificates PEM bundles, where each certificate is preceded by a
/// "# Owner: <TCG vendor ID>" comment.
fn parse_owned_pem(
    text: &str,
    kind: Kind,
    file: &str,
    vendors: &mut Vendors,
) -> anyhow::Result<Vec<SourceCertificate>> {
    let mut certificates = Vec::new();
    let mut owner = None;
    let mut body: Option<String> = None;
    for line in text.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("# Owner:") {
            owner = Some(value.trim().to_string());
        } else if line == "-----BEGIN CERTIFICATE-----" {
            body = Some(String::new());
        } else if line == "-----END CERTIFICATE-----" {
            let encoded = body
                .take()
                .with_context(|| format!("{file}: END without BEGIN"))?;
            let index = certificates.len() + 1;
            let owner = owner
                .take()
                .with_context(|| format!("{file}: certificate #{index} has no Owner comment"))?;
            certificates.push(SourceCertificate {
                kind,
                vendor: vendors.canonical(&owner),
                der: BASE64_STANDARD
                    .decode(encoded)
                    .with_context(|| format!("{file}: certificate #{index} is not base64"))?,
                origin: format!("{file} #{index}"),
            });
        } else if let Some(body) = body.as_mut() {
            body.push_str(line);
        }
    }
    ensure!(body.is_none(), "{file}: unterminated certificate block");
    ensure!(!certificates.is_empty(), "{file} contains no certificates");
    Ok(certificates)
}

pub fn fetch_microsoft_trusted_tpm(
    http: &Http,
    verifiers: &Verifiers,
    workdir: &Path,
    vendors: &mut Vendors,
) -> anyhow::Result<MicrosoftTrustedTpm> {
    let cab = http.get(TRUSTED_TPM_CAB_URL)?;
    let dir = workdir.join("microsoft");
    fs::create_dir(&dir)?;
    let cab_path = dir.join("TrustedTpm.cab");
    fs::write(&cab_path, &cab)?;
    verify_authenticode(http, verifiers, &dir, &cab_path)?;
    println!("Verified TrustedTpm.cab Authenticode signature, CRLs included");

    let (last_updated, certificates) = extract_cab(&cab, vendors)?;
    Ok(MicrosoftTrustedTpm {
        cab_sha256: sha256::digest(cab.as_slice()),
        last_updated,
        certificates,
    })
}

fn verify_authenticode(
    http: &Http,
    verifiers: &Verifiers,
    dir: &Path,
    cab_path: &Path,
) -> anyhow::Result<()> {
    let signing_root = dir.join("microsoft_root_ca_2011.pem");
    let timestamp_root = dir.join("microsoft_root_ca_2010.pem");
    fs::write(&signing_root, MICROSOFT_ROOT_2011_PEM)?;
    fs::write(&timestamp_root, MICROSOFT_ROOT_2010_PEM)?;
    let verify = || {
        let mut command = Command::new(&verifiers.osslsigncode);
        command
            .args(["verify", "-ignore-cdp", "-in"])
            .arg(cab_path)
            .arg("-CAfile")
            .arg(&signing_root)
            .arg("-TSA-CAfile")
            .arg(&timestamp_root);
        command
    };

    // osslsigncode cannot fetch CRLs itself because Microsoft serves them as
    // application/octet-stream, so this run only lists the distribution points
    let (_, probe) = capture(&mut verify(), "osslsigncode")?;
    let mut command = verify();
    for (marker, flag, file) in [
        (SIGNER_CRL_MARKER, "-CRLfile", "signer_crls.pem"),
        (TSA_CRL_MARKER, "-TSA-CRLfile", "tsa_crls.pem"),
    ] {
        let urls = crl_urls(&probe, marker)?;
        if urls.is_empty() {
            continue;
        }
        let mut pem = String::new();
        for url in urls {
            pem.push_str(&crl_to_pem(&http.get(url)?));
        }
        let path = dir.join(file);
        fs::write(&path, pem)?;
        command.arg(flag).arg(path);
    }

    let (verified, output) = capture(&mut command, "osslsigncode")?;
    ensure!(
        verified && output.lines().any(|line| line.trim() == "Succeeded"),
        "TrustedTpm.cab signature verification failed:\n{output}"
    );
    let signer = output
        .split_once("Signer's certificate:")
        .and_then(|(_, rest)| {
            rest.lines()
                .find_map(|line| line.trim().strip_prefix("Subject: "))
        });
    ensure!(
        signer == Some(MICROSOFT_SIGNER),
        "TrustedTpm.cab is signed by {signer:?}, expected {MICROSOFT_SIGNER}"
    );
    Ok(())
}

fn crl_urls<'a>(output: &'a str, marker: &str) -> anyhow::Result<Vec<&'a str>> {
    let mut urls: Vec<&str> = output
        .lines()
        .filter_map(|line| line.trim().strip_prefix(marker))
        .map(str::trim)
        .collect();
    urls.sort_unstable();
    urls.dedup();
    for url in &urls {
        ensure!(
            MICROSOFT_CRL_PREFIXES
                .iter()
                .any(|prefix| url.starts_with(prefix)),
            "unexpected CRL distribution point {url}"
        );
    }
    Ok(urls)
}

fn crl_to_pem(crl: &[u8]) -> String {
    if crl.starts_with(b"-----BEGIN") {
        return String::from_utf8_lossy(crl).into_owned();
    }
    let encoded = BASE64_STANDARD.encode(crl);
    let mut pem = String::from("-----BEGIN X509 CRL-----\n");
    for chunk in encoded.as_bytes().chunks(64) {
        pem.push_str(&String::from_utf8_lossy(chunk));
        pem.push('\n');
    }
    pem.push_str("-----END X509 CRL-----\n");
    pem
}

/// Reads Vendor\RootCA and Vendor\IntermediateCA certificates and the version.txt update date.
fn extract_cab(
    cab: &[u8],
    vendors: &mut Vendors,
) -> anyhow::Result<(String, Vec<SourceCertificate>)> {
    let mut cabinet =
        cab::Cabinet::new(Cursor::new(cab)).context("TrustedTpm.cab is not a valid cabinet")?;
    let names: Vec<String> = cabinet
        .folder_entries()
        .flat_map(|folder| folder.file_entries().map(|file| file.name().to_string()))
        .collect();

    let mut updated = None;
    let mut certificates = Vec::new();
    for name in names {
        let mut data = Vec::new();
        cabinet
            .read_file(&name)
            .and_then(|mut file| file.read_to_end(&mut data))
            .with_context(|| format!("failed to extract {name} from TrustedTpm.cab"))?;
        if name.eq_ignore_ascii_case("version.txt") {
            updated = Some(last_updated(&String::from_utf8_lossy(&data))?);
            continue;
        }
        let parts: Vec<&str> = name.split('\\').collect();
        let [vendor, folder, file] = parts.as_slice() else {
            continue;
        };
        let is_certificate = Path::new(file).extension().is_some_and(|ext| {
            CERTIFICATE_EXTENSIONS
                .iter()
                .any(|allowed| ext.eq_ignore_ascii_case(allowed))
        });
        if !is_certificate {
            println!("Skipping TrustedTpm.cab {name}");
            continue;
        }
        let kind = match *folder {
            "RootCA" => Kind::Root,
            "IntermediateCA" => Kind::Intermediate,
            _ => bail!("unexpected TrustedTpm.cab folder in {name}"),
        };
        let vendor = vendors.canonical(vendor);
        for der in decode_certificate_file(&data, &name)? {
            certificates.push(SourceCertificate {
                kind,
                vendor: vendor.clone(),
                der,
                origin: format!("TrustedTpm.cab {name}"),
            });
        }
    }

    let updated = updated.context("TrustedTpm.cab has no version.txt")?;
    ensure!(
        !certificates.is_empty(),
        "TrustedTpm.cab contains no certificates"
    );
    Ok((updated, certificates))
}

/// version.txt is a changelog that starts with "The certificates were last updated on:" and a date.
fn last_updated(version_txt: &str) -> anyhow::Result<String> {
    let mut lines = version_txt.lines().map(str::trim).filter(|l| !l.is_empty());
    lines
        .find(|line| line.to_ascii_lowercase().contains("last updated on"))
        .and_then(|_| lines.next())
        .map(ToString::to_string)
        .context("TrustedTpm.cab version.txt has no last updated date")
}

fn decode_certificate_file(data: &[u8], name: &str) -> anyhow::Result<Vec<Vec<u8>>> {
    if !data.trim_ascii_start().starts_with(b"-----BEGIN") {
        return Ok(vec![data.to_vec()]);
    }
    let certificates = CertificateDer::pem_slice_iter(data)
        .map(|cert| cert.map(|cert| cert.to_vec()))
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("{name} is not a valid PEM certificate"))?;
    ensure!(!certificates.is_empty(), "{name} contains no certificates");
    Ok(certificates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_tags_are_dates() {
        assert!(is_bundle_tag("2026-06-10"));
        assert!(!is_bundle_tag("v0.11.3"));
        assert!(!is_bundle_tag("2026-6-1"));
        assert!(!is_bundle_tag("2026-13-01"));
    }

    #[test]
    fn parses_checksums() {
        let checksums = parse_checksums("ABC  a.pem\ndef *b.pem\n\n").unwrap();
        assert_eq!(checksums["a.pem"], "abc");
        assert_eq!(checksums["b.pem"], "def");
    }

    #[test]
    fn owned_pem_requires_owner_per_certificate() {
        let block = "-----BEGIN CERTIFICATE-----\nAAEC\n-----END CERTIFICATE-----\n";
        let mut vendors = Vendors::default();

        let parsed = parse_owned_pem(
            &format!("# Owner: IFX\n{block}# Owner: STM\n{block}"),
            Kind::Root,
            "x.pem",
            &mut vendors,
        )
        .unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].vendor, "Infineon");
        assert_eq!(parsed[0].der, [0, 1, 2]);
        assert_eq!(parsed[1].vendor, "STMicroelectronics");

        let missing = parse_owned_pem(
            &format!("# Owner: IFX\n{block}{block}"),
            Kind::Root,
            "x.pem",
            &mut vendors,
        );
        assert!(missing.is_err());
    }

    #[test]
    fn crl_urls_must_point_at_microsoft() {
        let output = format!(
            "{SIGNER_CRL_MARKER} http://www.microsoft.com/pkiops/crl/a.crl\n\
             {TSA_CRL_MARKER} http://www.microsoft.com/pkiops/crl/b.crl\n\
             {SIGNER_CRL_MARKER} http://www.microsoft.com/pkiops/crl/a.crl\n"
        );
        assert_eq!(
            crl_urls(&output, SIGNER_CRL_MARKER).unwrap(),
            ["http://www.microsoft.com/pkiops/crl/a.crl"]
        );
        assert!(
            crl_urls(
                &format!("{SIGNER_CRL_MARKER} http://evil.test/a.crl"),
                SIGNER_CRL_MARKER
            )
            .is_err()
        );
    }

    #[test]
    fn version_is_the_last_update_date() {
        let text = "The certificates were last updated on:\r\n23-October-2026\r\n    Added x\r\n";
        assert_eq!(last_updated(text).unwrap(), "23-October-2026");
        assert!(last_updated("Added x\r\n").is_err());
    }

    #[test]
    fn crl_der_is_wrapped_as_pem() {
        let pem = crl_to_pem(&[0; 60]);
        assert!(pem.starts_with("-----BEGIN X509 CRL-----\n"));
        assert!(pem.ends_with("-----END X509 CRL-----\n"));
        assert!(pem.lines().all(|line| line.len() <= 64));
    }
}
