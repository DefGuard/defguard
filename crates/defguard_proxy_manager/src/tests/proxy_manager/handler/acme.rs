use defguard_common::db::models::{Certificates, ProxyCertSource};
use defguard_proto::proxy::{
    AcmeCertificate as AcmeCertPayload, CoreRequest, core_request, core_response,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::timeout;

use super::support::complete_proxy_handshake;
use crate::tests::common::{
    HandlerTestContext, ManagerTestContext, MockProxyHarness, RECEIVE_TIMEOUT, create_proxy,
};

/// A minimal but syntactically valid PEM certificate block (content is
/// arbitrary bytes - the handler stores it verbatim without parsing).
const TEST_CERT_PEM: &str =
    "-----BEGIN CERTIFICATE-----\nMIIBkTCB+wIJ\n-----END CERTIFICATE-----\n";
const TEST_KEY_PEM: &str =
    "-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2Vw\n-----END PRIVATE KEY-----\n";
const TEST_ACCOUNT_JSON: &str = r#"{"account_url":"https://acme.example/account/1"}"#;

const ALT_CERT_PEM: &str = "-----BEGIN CERTIFICATE-----\nQWxURVJOQVRF\n-----END CERTIFICATE-----\n";
const ALT_KEY_PEM: &str =
    "-----BEGIN PRIVATE KEY-----\nQWxURVJOQVRFa2V5\n-----END PRIVATE KEY-----\n";
const ALT_ACCOUNT_JSON: &str = r#"{"account_url":"https://acme.example/account/2"}"#;

/// Build the `CoreRequest` that a proxy sends when it has completed ACME
/// certificate issuance.
fn make_acme_certificate_request(
    cert_pem: &str,
    key_pem: &str,
    account_credentials_json: &str,
) -> CoreRequest {
    CoreRequest {
        id: 9000,
        device_info: None,
        payload: Some(core_request::Payload::AcmeCertificate(AcmeCertPayload {
            cert_pem: cert_pem.to_string(),
            key_pem: key_pem.to_string(),
            account_credentials_json: account_credentials_json.to_string(),
        })),
    }
}

/// Complete the manager-level handshake: wait for the proxy to connect then
/// consume the `InitialInfo` response.
async fn complete_manager_handshake(mock_proxy: &mut MockProxyHarness) {
    mock_proxy.wait_connected().await;
    mock_proxy.recv_initial_info().await;
}

/// Sending `AcmeCertificate` causes the handler to persist the certificate
/// data in the `certificates` DB table.  No outbound `CoreResponse` is
/// produced (the protocol contract is a fire-and-forget push from the proxy).
#[sqlx::test]
async fn test_acme_certificate_saves_to_db(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    context
        .mock_proxy()
        .send_request(make_acme_certificate_request(
            TEST_CERT_PEM,
            TEST_KEY_PEM,
            TEST_ACCOUNT_JSON,
        ));

    // The handler does not send a response for AcmeCertificate; assert silence.
    context.mock_proxy_mut().expect_no_outbound().await;

    // Verify the DB was updated.
    let certs = Certificates::get_or_default(&context.pool)
        .await
        .expect("failed to query certificates from DB");

    assert_eq!(
        certs.proxy_http_cert_pem.as_deref(),
        Some(TEST_CERT_PEM),
        "proxy_http_cert_pem should be updated"
    );
    assert_eq!(
        certs.proxy_http_cert_key_pem.as_deref(),
        Some(TEST_KEY_PEM),
        "proxy_http_cert_key_pem should be updated"
    );
    assert_eq!(
        certs.acme_account_credentials.as_deref(),
        Some(TEST_ACCOUNT_JSON),
        "acme_account_credentials should be updated"
    );
    assert_eq!(
        certs.proxy_http_cert_source,
        ProxyCertSource::LetsEncrypt,
        "proxy_http_cert_source should be set to LetsEncrypt"
    );

    context.finish().await.expect_server_finished().await;
}

/// A second `AcmeCertificate` request overwrites the previously stored values.
#[sqlx::test]
async fn test_acme_certificate_overwrites_existing(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Seed the DB with an earlier certificate.
    let initial_certs = Certificates {
        proxy_http_cert_pem: Some(ALT_CERT_PEM.to_string()),
        proxy_http_cert_key_pem: Some(ALT_KEY_PEM.to_string()),
        acme_account_credentials: Some(ALT_ACCOUNT_JSON.to_string()),
        proxy_http_cert_source: ProxyCertSource::LetsEncrypt,
        ..Default::default()
    };
    initial_certs
        .save(&context.pool)
        .await
        .expect("failed to seed initial certificates");

    // Send a new AcmeCertificate request with different PEM values.
    context
        .mock_proxy()
        .send_request(make_acme_certificate_request(
            TEST_CERT_PEM,
            TEST_KEY_PEM,
            TEST_ACCOUNT_JSON,
        ));

    context.mock_proxy_mut().expect_no_outbound().await;

    // Verify the new values replaced the old ones.
    let certs = Certificates::get_or_default(&context.pool)
        .await
        .expect("failed to query certificates from DB after overwrite");

    assert_eq!(
        certs.proxy_http_cert_pem.as_deref(),
        Some(TEST_CERT_PEM),
        "proxy_http_cert_pem should be overwritten with new value"
    );
    assert_eq!(
        certs.proxy_http_cert_key_pem.as_deref(),
        Some(TEST_KEY_PEM),
        "proxy_http_cert_key_pem should be overwritten with new value"
    );
    assert_eq!(
        certs.acme_account_credentials.as_deref(),
        Some(TEST_ACCOUNT_JSON),
        "acme_account_credentials should be overwritten with new value"
    );

    context.finish().await.expect_server_finished().await;
}

/// When the full `ProxyManager` loop is running, the handler registers itself
/// in `handler_tx_map`.  After processing `AcmeCertificate`, it broadcasts
/// `HttpsCerts` to ALL registered handlers, which forward it to their
/// respective proxy streams.  This test verifies that every connected mock
/// proxy receives the `HttpsCerts` response - including proxies other than the
/// one that sent the certificate.
#[sqlx::test]
async fn test_acme_certificate_broadcasts_to_connected_proxy(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = ManagerTestContext::new(options).await;

    let proxy_a = create_proxy(&context.pool).await;
    let mut mock_a = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_a, &mock_a);

    let proxy_b = create_proxy(&context.pool).await;
    let mut mock_b = MockProxyHarness::start().await;
    context.register_proxy_mock(&proxy_b, &mock_b);

    context.start().await;
    complete_manager_handshake(&mut mock_a).await;
    complete_manager_handshake(&mut mock_b).await;

    // Inject an AcmeCertificate request from proxy A into the running handler.
    mock_a.send_request(make_acme_certificate_request(
        TEST_CERT_PEM,
        TEST_KEY_PEM,
        TEST_ACCOUNT_JSON,
    ));

    // The handler must broadcast HttpsCerts to ALL registered proxies - both
    // the sender (proxy A) and the bystander (proxy B).
    for (label, mock) in [
        ("proxy A (sender)", &mut mock_a),
        ("proxy B (bystander)", &mut mock_b),
    ] {
        let response = timeout(RECEIVE_TIMEOUT, mock.recv_outbound())
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for HttpsCerts broadcast on {label}"));

        match response.payload {
            Some(core_response::Payload::HttpsCerts(h)) => {
                assert_eq!(
                    h.cert_pem, TEST_CERT_PEM,
                    "{label}: broadcast cert_pem should match the submitted certificate"
                );
                assert_eq!(
                    h.key_pem, TEST_KEY_PEM,
                    "{label}: broadcast key_pem should match the submitted key"
                );
            }
            other => panic!(
                "{label}: expected HttpsCerts response from AcmeCertificate broadcast, got: {:?}",
                other.as_ref().map(std::mem::discriminant)
            ),
        }
    }

    context.finish().await;
}
