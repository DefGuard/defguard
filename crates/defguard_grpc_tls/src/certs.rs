//! Custom TLS verification for proxy and gateway connections.
//!
//! Motivation:
//! - tonic/rustls does not fetch or enforce CRL distribution points, so revocation
//!   has to be enforced by the application.
//! - We pin each component to its expected certificate serial and reject mismatches
//!   at the TLS layer, before any gRPC requests are processed.
//! - A lightweight in-memory cache (refreshed periodically) avoids database access
//!   during the handshake and keeps verification synchronous.

use std::{collections::HashMap, sync::Arc};

use defguard_common::db::Id;
use rustls::{
    CertificateError, DistinguishedName, Error as RustlsError, RootCertStore, SignatureScheme,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    crypto,
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use thiserror::Error;
use tokio::sync::watch;
use tracing::error;
use x509_parser::parse_x509_certificate;

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

/// Create a rustls client config that enforces the pinned component certificate serial.
pub fn client_config(
    ca_cert_der: &[u8],
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    component_id: Id,
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
    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| CertConfigError::TlsConfig(err.to_string()))?;
    let mut config = builder.with_root_certificates(roots).with_no_client_auth();
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
