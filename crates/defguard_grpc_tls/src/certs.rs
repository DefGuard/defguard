//! Custom TLS verification for proxy and gateway connections.
//!
//! Motivation:
//! - tonic/rustls does not fetch or enforce CRL distribution points, so revocation
//!   has to be enforced by the application.
//! - We pin each component to its expected certificate serial and reject mismatches
//!   at the TLS layer, before any gRPC requests are processed.
//! - A lightweight in-memory cache (refreshed periodically) avoids database access
//!   during the handshake and keeps verification synchronous.

use std::{collections::HashMap, sync::Arc, time::Duration};

use defguard_common::db::{Id, models::proxy::Proxy};
use hyper_rustls::HttpsConnectorBuilder;
use rustls::{
    CertificateError, DistinguishedName, Error as RustlsError, RootCertStore, SignatureScheme,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    crypto,
    pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime},
};
use thiserror::Error;
use tokio::sync::watch;
use tonic::transport::{Certificate, Channel, Endpoint, Identity, ServerTlsConfig};
use tracing::error;
use x509_parser::parse_x509_certificate;

use crate::connector::HttpsSchemeConnector;

const TEN_SECS: Duration = Duration::from_secs(10);

/// Errors that can occur while building a TLS config with a pinned verifier.
#[derive(Debug, Error)]
pub enum CertConfigError {
    #[error("TLS config error: {0}")]
    TlsConfig(String),
}

/// Wraps WebPKI verification to enforce component-specific certificate serials.
#[derive(Debug)]
struct CertVerifier {
    inner: Arc<dyn ServerCertVerifier>,
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    component_id: Id,
}

impl CertVerifier {
    fn new(
        inner: Arc<dyn ServerCertVerifier>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
        component_id: Id,
    ) -> Self {
        Self {
            inner,
            certs_rx,
            component_id,
        }
    }

    /// Validate the peer certificate serial against the expected component serial.
    fn verify(&self, end_entity: &CertificateDer<'_>) -> Result<(), RustlsError> {
        let (_, cert) = parse_x509_certificate(end_entity.as_ref())
            .map_err(|_| RustlsError::InvalidCertificate(CertificateError::BadEncoding))?;
        let serial = cert.tbs_certificate.raw_serial_as_string();
        let certs = self.certs_rx.borrow();
        let Some(expected) = certs.get(&self.component_id) else {
            error!(
                "Missing expected certificate for component id={}, serial={}",
                self.component_id, serial
            );
            return Err(RustlsError::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ));
        };
        if !expected.eq_ignore_ascii_case(&serial) {
            error!(
                "Invalid certificate for component id={}: expected={} got={}.",
                self.component_id, expected, serial
            );
            return Err(RustlsError::InvalidCertificate(
                CertificateError::ApplicationVerificationFailure,
            ));
        }
        Ok(())
    }
}

impl ServerCertVerifier for CertVerifier {
    /// Delegate chain validation to WebPKI, then enforce the component-specific pin.
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        self.inner.verify_server_cert(
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        )?;
        self.verify(end_entity)?;
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, RustlsError> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }

    fn root_hint_subjects(&self) -> Option<&[DistinguishedName]> {
        self.inner.root_hint_subjects()
    }
}

/// Build a root store from the configured CA for WebPKI validation.
fn root_store_from_ca(ca_cert_der: &[u8]) -> Result<RootCertStore, CertConfigError> {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(ca_cert_der.to_vec()))
        .map_err(|err| CertConfigError::TlsConfig(err.to_string()))?;
    Ok(roots)
}

/// Build a tonic [`ServerTlsConfig`] for a gateway or proxy gRPC server that enforces
/// mutual TLS.
///
/// The returned config:
/// - presents `component_cert_pem` / `component_key_pem` as the server identity, and
/// - requires every connecting client (i.e. Core) to present a certificate signed by
///   `ca_cert_pem` (client auth is **not** optional).
///
/// The PEM arguments are the raw PEM bytes (or a string slice coerced to bytes).
/// Both certificate and key must be in PKCS#8 / SEC1 PEM format as produced by
/// `defguard_certs`.
pub fn server_tls_config(
    component_cert_pem: impl AsRef<[u8]>,
    component_key_pem: impl AsRef<[u8]>,
    ca_cert_pem: impl AsRef<[u8]>,
) -> Result<ServerTlsConfig, CertConfigError> {
    let identity = Identity::from_pem(component_cert_pem.as_ref(), component_key_pem.as_ref());
    let ca = Certificate::from_pem(ca_cert_pem.as_ref());
    Ok(ServerTlsConfig::new()
        .identity(identity)
        .client_ca_root(ca)
        .client_auth_optional(false))
}

/// Create a rustls client config that enforces the pinned component certificate serial
/// and presents the Core client certificate for mutual TLS authentication.
///
/// `core_client_cert_der` and `core_client_cert_key_der` are the DER-encoded client
/// certificate and its private key that Core presents to the gateway/proxy during the
/// TLS handshake.  The gateway/proxy verifies this cert against `ca_cert_der`.
pub fn client_config(
    ca_cert_der: &[u8],
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    component_id: Id,
    core_client_cert_der: &[u8],
    core_client_cert_key_der: &[u8],
) -> Result<rustls::ClientConfig, CertConfigError> {
    let provider = Arc::new(crypto::ring::default_provider());
    let roots = root_store_from_ca(ca_cert_der)?;
    let verifier_roots = root_store_from_ca(ca_cert_der)?;
    let verifier = WebPkiServerVerifier::builder_with_provider(
        Arc::new(verifier_roots),
        Arc::clone(&provider),
    )
    .build()
    .map_err(|err| CertConfigError::TlsConfig(err.to_string()))?;

    let client_cert = CertificateDer::from(core_client_cert_der.to_vec());
    let client_key = PrivateKeyDer::try_from(core_client_cert_key_der.to_vec())
        .map_err(|err| CertConfigError::TlsConfig(format!("invalid client key DER: {err}")))?;

    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| CertConfigError::TlsConfig(err.to_string()))?;
    let mut config = builder
        .with_root_certificates(roots)
        .with_client_auth_cert(vec![client_cert], client_key)
        .map_err(|err| CertConfigError::TlsConfig(format!("client auth cert error: {err}")))?;

    let verifier: Arc<dyn ServerCertVerifier> = verifier;
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(CertVerifier::new(
            verifier,
            certs_rx,
            component_id,
        )));
    Ok(config)
}

/// Build an mTLS [`Channel`] to a proxy using its stored per-component client certificate.
///
/// * `proxy` - the full `Proxy<Id>` row from the database; `core_client_cert_der`,
///   `core_client_cert_key_der`, and `certificate_serial` must all be `Some`.
/// * `ca_cert_der` - the core CA certificate in DER form, used as the only trusted root.
/// * `certs_rx` - watch channel carrying the current `{ proxy_id → cert_serial }` map.
///   Pass a long-lived receiver for persistent connections (serial revocation is picked up
///   dynamically) or a one-shot channel seeded with the proxy's current serial for
///   short-lived calls.
///
/// The returned channel uses an `http://` endpoint scheme; TLS is applied by the
/// internal [`HttpsSchemeConnector`](crate::connector::HttpsSchemeConnector).
pub fn proxy_mtls_channel(
    proxy: &Proxy<Id>,
    ca_cert_der: &[u8],
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
) -> Result<Channel, CertConfigError> {
    let cert_der = proxy.core_client_cert_der.as_deref().ok_or_else(|| {
        CertConfigError::TlsConfig(format!(
            "core client certificate not provisioned for proxy id={}",
            proxy.id
        ))
    })?;
    let key_der = proxy.core_client_cert_key_der.as_deref().ok_or_else(|| {
        CertConfigError::TlsConfig(format!(
            "core client certificate key not provisioned for proxy id={}",
            proxy.id
        ))
    })?;

    let tls_config = client_config(ca_cert_der, certs_rx, proxy.id, cert_der, key_der)?;

    let connector = HttpsConnectorBuilder::new()
        .with_tls_config(tls_config)
        .https_only()
        .enable_http2()
        .build();
    let connector = HttpsSchemeConnector::new(connector);

    // Use http:// scheme - the HttpsSchemeConnector rewrites it to https:// internally.
    let endpoint_str = format!("http://{}:{}", proxy.address, proxy.port);
    let endpoint = Endpoint::from_shared(endpoint_str)
        .map_err(|e| CertConfigError::TlsConfig(format!("invalid proxy endpoint URL: {e}")))?
        .http2_keep_alive_interval(TEN_SECS)
        .tcp_keepalive(Some(TEN_SECS))
        .keep_alive_while_idle(true);

    Ok(endpoint.connect_with_connector_lazy(connector))
}
