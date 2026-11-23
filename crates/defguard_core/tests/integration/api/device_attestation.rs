use defguard_certs::parse_pem_certificate;
use defguard_common::db::setup_pool;
use defguard_core::enterprise::{
    handlers::device_attestation::TpmCaCertificatePem, tpm::trust_bundle::tpm_trust_bundle,
};
use reqwest::StatusCode;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::common::{authenticate_admin, client::TestClient, make_test_client};

const CERTIFICATES_URL: &str = "/api/v1/device-attestation/certificates";

async fn setup(options: PgConnectOptions) -> TestClient {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    authenticate_admin(&mut client).await;
    client
}

async fn list(client: &TestClient, query: &str) -> Value {
    let response = client
        .get(format!("{CERTIFICATES_URL}?{query}"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    response.json().await
}

fn field<'a>(page: &'a Value, name: &str) -> Vec<&'a str> {
    page["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item[name].as_str().unwrap())
        .collect()
}

#[sqlx::test]
async fn test_list_tpm_ca_certificates(_: PgPoolOptions, options: PgConnectOptions) {
    let client = setup(options).await;

    let page = list(&client, "").await;
    let names = field(&page, "common_name");
    assert_eq!(names.len(), 50);
    assert_eq!(
        page["pagination"]["total_items"].as_u64().unwrap(),
        tpm_trust_bundle().entries().len() as u64
    );
    let mut sorted = names.clone();
    sorted.sort_by_key(|name| name.to_lowercase());
    assert_eq!(names, sorted);

    let page = list(&client, "search=INFINEON&per_page=100").await;
    let total = page["pagination"]["total_items"].as_u64().unwrap();
    assert!(total > 0);
    assert!(
        page["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item.to_string().to_lowercase().contains("infineon"))
    );

    let page = list(&client, "search=no-such-certificate-authority").await;
    assert_eq!(page["pagination"]["total_items"].as_u64().unwrap(), 0);
    assert!(page["data"].as_array().unwrap().is_empty());

    let page = list(&client, "sort_by=not_after&sort_order=desc&per_page=100").await;
    let not_after = field(&page, "not_after");
    assert!(not_after.windows(2).all(|pair| pair[0] >= pair[1]));
}

#[sqlx::test]
async fn test_get_tpm_ca_certificate_pem(_: PgPoolOptions, options: PgConnectOptions) {
    let client = setup(options).await;
    let sha256 = &tpm_trust_bundle().entries()[0].info.sha256;

    let response = client
        .get(format!("{CERTIFICATES_URL}/{sha256}/pem"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body: TpmCaCertificatePem = response.json().await;
    let der = parse_pem_certificate(&body.pem).unwrap();
    assert_eq!(sha256::digest(der.as_ref()), *sha256);

    let response = client
        .get(format!("{CERTIFICATES_URL}/{}/pem", "00".repeat(32)))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_tpm_ca_certificates_require_admin(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut client, _) = make_test_client(pool).await;
    client.login_user("hpotter", "pass123").await;
    let sha256 = &tpm_trust_bundle().entries()[0].info.sha256;

    let response = client.get(CERTIFICATES_URL).send().await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = client
        .get(format!("{CERTIFICATES_URL}/{sha256}/pem"))
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
