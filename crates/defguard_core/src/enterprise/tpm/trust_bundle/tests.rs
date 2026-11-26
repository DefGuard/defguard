use chrono::TimeZone;
use rcgen::{CertificateParams, DnType, KeyPair};
use serde_json::{Value, json};

use super::*;
use crate::handlers::pagination::PaginationParams;

fn certificate(common_name: &str, vendor: &str, kind: &str, not_after: &str) -> Value {
    let key_pair = KeyPair::generate().expect("key pair");
    let mut params = CertificateParams::new(Vec::new()).expect("params");
    params
        .distinguished_name
        .push(DnType::CommonName, common_name);
    let der = params
        .self_signed(&key_pair)
        .expect("certificate")
        .der()
        .to_vec();
    json!({
        "kind": kind,
        "vendor": vendor,
        "commonName": common_name,
        "organization": null,
        "subject": format!("CN={common_name}"),
        "issuer": format!("CN={common_name}"),
        "serialNumber": "01",
        "selfSigned": true,
        "keyAlgorithm": "ECC P-256",
        "notBefore": "2020-01-01T00:00:00Z",
        "notAfter": not_after,
        "sha256": sha256::digest(der.as_slice()),
        "spkiSha256": "00",
        "sources": ["microsoft"],
        "der": BASE64_STANDARD.encode(&der),
    })
}

fn bundle_json(certificates: Vec<Value>) -> String {
    json!({
        "generatedAt": "2026-11-01T00:00:00Z",
        "tpmCaCertificates": { "tag": "2026-06-10" },
        "microsoftTrustedTpm": { "cabSha256": "ab", "lastUpdated": "23-October-2026" },
        "certificates": certificates,
    })
    .to_string()
}

fn sample_bundle() -> TpmTrustBundle {
    let mut tpm_ca = certificate(
        "Infineon OPTIGA RSA Root CA",
        "Infineon",
        "root",
        "2040-01-01T00:00:00Z",
    );
    tpm_ca["sources"] = json!(["tpm-ca-certificates", "microsoft"]);
    parse_bundle(&bundle_json(vec![
        tpm_ca,
        certificate(
            "Infineon OPTIGA ECC Manufacturing CA",
            "Infineon",
            "intermediate",
            "2030-01-01T00:00:00Z",
        ),
        certificate(
            "STM TPM EK Intermediate CA 05",
            "STMicroelectronics",
            "intermediate",
            "2020-06-01T00:00:00Z",
        ),
        certificate("Intel TPM EK root", "Intel", "root", "2049-01-01T00:00:00Z"),
        certificate(
            "Intel TPM EK root",
            "Intel",
            "intermediate",
            "2035-01-01T00:00:00Z",
        ),
    ]))
    .expect("sample bundle parses")
}

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 16, 0, 0, 0).unwrap()
}

fn query_page<'a>(
    bundle: &'a TpmTrustBundle,
    filter: &TpmCaFilter,
    sort: &TpmCaSort,
    pagination: PaginationParams,
) -> (Vec<&'a TpmCaEntry>, u32) {
    bundle.query(
        filter,
        sort,
        pagination.offset(),
        pagination.per_page(),
        now(),
    )
}

fn names(entries: &[&TpmCaEntry]) -> Vec<String> {
    entries.iter().map(|e| e.info.common_name.clone()).collect()
}

#[test]
fn test_shipped_bundle_parses() {
    let bundle = parse_bundle(TPM_CA_CERTIFICATES_JSON).expect("shipped bundle parses");
    assert!(bundle.roots().next().is_some());
    assert!(bundle.intermediates().next().is_some());
    assert!(bundle.entries().iter().all(|e| {
        !e.info.vendor.is_empty() && !e.info.common_name.is_empty() && !e.info.sources.is_empty()
    }));
}

#[test]
fn test_rejects_tampered_certificates() {
    let mut wrong_digest = certificate("A", "Intel", "root", "2040-01-01T00:00:00Z");
    wrong_digest["sha256"] = json!("00".repeat(32));
    assert!(matches!(
        parse_bundle(&bundle_json(vec![wrong_digest])),
        Err(TpmBundleError::Digest { .. })
    ));

    let mut bad_base64 = certificate("A", "Intel", "root", "2040-01-01T00:00:00Z");
    bad_base64["der"] = json!("not base64!");
    assert!(matches!(
        parse_bundle(&bundle_json(vec![bad_base64])),
        Err(TpmBundleError::Base64 { .. })
    ));

    let mut not_x509 = certificate("A", "Intel", "root", "2040-01-01T00:00:00Z");
    not_x509["der"] = json!(BASE64_STANDARD.encode([1, 2, 3]));
    not_x509["sha256"] = json!(sha256::digest([1u8, 2, 3].as_slice()));
    assert!(matches!(
        parse_bundle(&bundle_json(vec![not_x509])),
        Err(TpmBundleError::Certificate { .. })
    ));
}

#[test]
fn test_rejects_bundle_without_roots() {
    let intermediate = certificate("A", "Intel", "intermediate", "2040-01-01T00:00:00Z");
    assert!(matches!(
        parse_bundle(&bundle_json(vec![intermediate])),
        Err(TpmBundleError::NoRoots)
    ));
}

#[test]
fn test_search_is_case_insensitive_substring() {
    let bundle = sample_bundle();
    let filter = TpmCaFilter {
        search: Some("  optiga RSA ".into()),
        ..Default::default()
    };
    let (page, total) = query_page(
        &bundle,
        &filter,
        &TpmCaSort::default(),
        PaginationParams::default(),
    );
    assert_eq!(total, 1);
    assert_eq!(names(&page), ["Infineon OPTIGA RSA Root CA"]);

    let filter = TpmCaFilter {
        search: Some("stmicroelectronics".into()),
        ..Default::default()
    };
    let (_, total) = query_page(
        &bundle,
        &filter,
        &TpmCaSort::default(),
        PaginationParams::default(),
    );
    assert_eq!(total, 1);
}

#[test]
fn test_filters_combine() {
    let bundle = sample_bundle();
    let query = |filter: TpmCaFilter| {
        let (page, total) = query_page(
            &bundle,
            &filter,
            &TpmCaSort::default(),
            PaginationParams::default(),
        );
        assert_eq!(total as usize, page.len());
        names(&page)
    };

    assert_eq!(
        query(TpmCaFilter {
            kind: Some(TpmCaKind::Root),
            vendor: Some("infineon".into()),
            ..Default::default()
        }),
        ["Infineon OPTIGA RSA Root CA"]
    );
    assert_eq!(
        query(TpmCaFilter {
            source: Some(TpmCaSource::TpmCaCertificates),
            ..Default::default()
        }),
        ["Infineon OPTIGA RSA Root CA"]
    );
    assert_eq!(
        query(TpmCaFilter {
            expired: Some(true),
            ..Default::default()
        }),
        ["STM TPM EK Intermediate CA 05"]
    );
    assert_eq!(
        query(TpmCaFilter {
            expired: Some(false),
            vendor: Some("Intel".into()),
            kind: Some(TpmCaKind::Intermediate),
            ..Default::default()
        }),
        ["Intel TPM EK root"]
    );
}

#[test]
fn test_sorting_is_stable_across_pages() {
    let bundle = sample_bundle();
    let sort = TpmCaSort {
        sort_by: TpmCaSortKey::Name,
        sort_order: SortOrder::Asc,
    };
    let (all, total) = query_page(
        &bundle,
        &TpmCaFilter::default(),
        &sort,
        PaginationParams::new(1, 100),
    );
    assert_eq!(total, 5);
    assert_eq!(
        names(&all),
        [
            "Infineon OPTIGA ECC Manufacturing CA",
            "Infineon OPTIGA RSA Root CA",
            "Intel TPM EK root",
            "Intel TPM EK root",
            "STM TPM EK Intermediate CA 05",
        ]
    );
    assert!(all[2].info.sha256 < all[3].info.sha256);

    let paged: Vec<_> = (1..=3)
        .flat_map(|page| {
            let (entries, total) = query_page(
                &bundle,
                &TpmCaFilter::default(),
                &sort,
                PaginationParams::new(page, 2),
            );
            assert_eq!(total, 5);
            entries
        })
        .collect();
    assert_eq!(
        paged.iter().map(|e| &e.info.sha256).collect::<Vec<_>>(),
        all.iter().map(|e| &e.info.sha256).collect::<Vec<_>>()
    );

    let (beyond, total) = query_page(
        &bundle,
        &TpmCaFilter::default(),
        &sort,
        PaginationParams::new(4, 2),
    );
    assert!(beyond.is_empty());
    assert_eq!(total, 5);

    let desc = TpmCaSort {
        sort_by: TpmCaSortKey::NotAfter,
        sort_order: SortOrder::Desc,
    };
    let (latest, _) = query_page(
        &bundle,
        &TpmCaFilter::default(),
        &desc,
        PaginationParams::new(1, 1),
    );
    assert_eq!(
        latest[0].info.not_after.to_rfc3339(),
        "2049-01-01T00:00:00+00:00"
    );
}

#[test]
fn test_vendors_are_sorted_and_unique() {
    assert_eq!(
        sample_bundle().vendors(),
        ["Infineon", "Intel", "STMicroelectronics"]
    );
}

#[test]
fn test_find_ignores_case() {
    let bundle = sample_bundle();
    let sha256 = bundle.entries()[0].info.sha256.to_uppercase();
    assert!(bundle.find(&sha256).is_some());
    assert!(bundle.find("missing").is_none());
}

#[test]
fn test_stale_after_ninety_days() {
    let bundle = sample_bundle();
    let limit = bundle.generated_at + TimeDelta::days(90);
    assert!(!bundle.is_stale(limit));
    assert!(bundle.is_stale(limit + TimeDelta::seconds(1)));

    let summary = bundle.summary(limit);
    assert_eq!(summary.roots, 2);
    assert_eq!(summary.intermediates, 3);
    assert!(!summary.stale);
}
