//! Custom TLS verification for proxy connections.
//!
//! Motivation:
//! - tonic/rustls does not fetch or enforce CRL distribution points, so revocation
//!   has to be enforced by the application.
//! - We pin each proxy to its expected certificate serial and reject mismatches at
//!   the TLS layer, before any gRPC requests are processed.
//! - A lightweight in-memory cache (refreshed periodically) avoids database access
//!   during the handshake and keeps verification synchronous.

use std::{collections::HashMap, sync::Arc};

use defguard_common::db::{Id, models::proxy::Proxy};
use rustls::{
    CertificateError, DistinguishedName, Error as RustlsError, RootCertStore, SignatureScheme,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    crypto,
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use sqlx::PgPool;
use tokio::sync::watch;
use x509_parser::parse_x509_certificate;

use crate::error::ProxyError;

/// Wraps WebPKI verification to enforce proxy-specific certificate serials.
#[derive(Debug)]
struct CertVerifier {
    inner: Arc<dyn ServerCertVerifier>,
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    proxy_id: Id,
}

impl CertVerifier {
    fn new(
        inner: Arc<dyn ServerCertVerifier>,
        certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
        proxy_id: Id,
    ) -> Self {
        Self {
            inner,
            certs_rx,
            proxy_id,
        }
    }

    /// Validate the peer certificate serial against the expected proxy serial.
    fn verify(&self, end_entity: &CertificateDer<'_>) -> Result<(), RustlsError> {
        let (_, cert) = parse_x509_certificate(end_entity.as_ref())
            .map_err(|_| RustlsError::InvalidCertificate(CertificateError::BadEncoding))?;
        let serial = cert.tbs_certificate.raw_serial_as_string();
        let certs = self.certs_rx.borrow();
        let Some(expected) = certs.get(&self.proxy_id) else {
            error!("Missing expected certificate for proxy: {}", self.proxy_id);
            return Err(RustlsError::InvalidCertificate(CertificateError::Revoked));
        };
        if !expected.eq_ignore_ascii_case(&serial) {
            error!(
                "Invalid certificate for proxy {}: expected={expected} got={serial}",
                self.proxy_id
            );
            return Err(RustlsError::InvalidCertificate(CertificateError::Revoked));
        }
        Ok(())
    }
}

impl ServerCertVerifier for CertVerifier {
    /// Delegate chain validation to WebPKI, then enforce the proxy-specific pin.
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

/// Build a compact id->serial map, skipping proxies without a stored cert.
fn collect_certs<I>(items: I) -> HashMap<Id, String>
where
    I: IntoIterator<Item = (Id, Option<String>)>,
{
    items
        .into_iter()
        .filter_map(|(id, cert)| cert.map(|cert| (id, cert)))
        .collect()
}

/// Refresh the cached cert serials for all proxies.
pub(crate) async fn refresh_certs(pool: &PgPool, tx: &watch::Sender<Arc<HashMap<Id, String>>>) {
    match Proxy::all(pool).await {
        Ok(proxies) => {
            let certs = collect_certs(
                proxies
                    .into_iter()
                    .map(|proxy| (proxy.id, proxy.certificate)),
            );
            let _ = tx.send(Arc::new(certs));
        }
        Err(err) => {
            warn!("Failed to refresh revoked certificate list: {err}");
        }
    }
}

/// Build a root store from the configured CA for WebPKI validation.
fn root_store_from_ca(ca_cert_der: &[u8]) -> Result<RootCertStore, ProxyError> {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(ca_cert_der.to_vec()))
        .map_err(|err| ProxyError::TlsConfigError(err.to_string()))?;
    Ok(roots)
}

/// Create a rustls client config that enforces the pinned proxy certificate serial.
pub(crate) fn client_config(
    ca_cert_der: &[u8],
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    proxy_id: Id,
) -> Result<rustls::ClientConfig, ProxyError> {
    let provider = Arc::new(crypto::ring::default_provider());
    let roots = root_store_from_ca(ca_cert_der)?;
    let verifier_roots = root_store_from_ca(ca_cert_der)?;
    let verifier = WebPkiServerVerifier::builder_with_provider(
        Arc::new(verifier_roots),
        Arc::clone(&provider),
    )
    .build()
    .map_err(|err| ProxyError::TlsConfigError(err.to_string()))?;
    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|err| ProxyError::TlsConfigError(err.to_string()))?;
    let mut config = builder.with_root_certificates(roots).with_no_client_auth();
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(CertVerifier::new(verifier, certs_rx, proxy_id)));
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    use defguard_certs::{CertificateAuthority, Csr, DnType, generate_key_pair};
    use rustls::client::danger::HandshakeSignatureValid;

    #[derive(Debug)]
    struct NoopVerifier;

    impl ServerCertVerifier for NoopVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, RustlsError> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, RustlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, RustlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            Vec::new()
        }

        fn root_hint_subjects(&self) -> Option<&[DistinguishedName]> {
            None
        }
    }

    fn make_cert_and_serial() -> (CertificateDer<'static>, String) {
        let ca = CertificateAuthority::new("Defguard CA", "test@example.com", 30).unwrap();
        let key_pair = generate_key_pair().unwrap();
        let csr = Csr::new(
            &key_pair,
            &["proxy.local".to_string()],
            vec![(DnType::CommonName, "proxy.local")],
        )
        .unwrap();
        let cert = ca.sign_csr(&csr).unwrap();
        let cert_der = CertificateDer::from(cert.der().to_vec());
        let (_, parsed) = parse_x509_certificate(cert_der.as_ref()).unwrap();
        let serial = parsed.tbs_certificate.raw_serial_as_string();
        (cert_der, serial)
    }

    #[test]
    fn collect_certs_skips_missing() {
        let certs = collect_certs(vec![(1, None), (2, Some("abc".to_string()))]);
        assert_eq!(certs.len(), 1);
        assert_eq!(certs.get(&2), Some(&"abc".to_string()));
    }

    #[test]
    fn verify_accepts_expected_serial() {
        let (cert_der, serial) = make_cert_and_serial();
        let (_tx, rx) = watch::channel(Arc::new(HashMap::from([(1, serial.clone())])));
        let verifier = CertVerifier::new(Arc::new(NoopVerifier), rx, 1);
        let result = verifier.verify(&cert_der);
        assert!(result.is_ok());
    }

    #[test]
    fn verify_rejects_missing_expected_cert() {
        let (cert_der, serial) = make_cert_and_serial();
        let (_tx, rx) = watch::channel(Arc::new(HashMap::from([(2, serial)])));
        let verifier = CertVerifier::new(Arc::new(NoopVerifier), rx, 1);
        let result = verifier.verify(&cert_der);
        assert!(matches!(
            result,
            Err(RustlsError::InvalidCertificate(CertificateError::Revoked))
        ));
    }

    #[test]
    fn verify_rejects_mismatched_serial() {
        let (cert_der, _serial) = make_cert_and_serial();
        let (_tx, rx) = watch::channel(Arc::new(HashMap::from([(1, "deadbeef".to_string())])));
        let verifier = CertVerifier::new(Arc::new(NoopVerifier), rx, 1);
        let result = verifier.verify(&cert_der);
        assert!(matches!(
            result,
            Err(RustlsError::InvalidCertificate(CertificateError::Revoked))
        ));
    }

    #[test]
    fn verify_accepts_case_insensitive_serial() {
        let (cert_der, serial) = make_cert_and_serial();
        let expected_lower = serial.to_ascii_lowercase();
        let (_tx, rx) = watch::channel(Arc::new(HashMap::from([(1, expected_lower)])));
        let verifier = CertVerifier::new(Arc::new(NoopVerifier), rx, 1);
        let result = verifier.verify(&cert_der);
        assert!(result.is_ok());

        let expected_upper = serial.to_ascii_uppercase();
        let (_tx, rx) = watch::channel(Arc::new(HashMap::from([(1, expected_upper)])));
        let verifier = CertVerifier::new(Arc::new(NoopVerifier), rx, 1);
        let result = verifier.verify(&cert_der);
        assert!(result.is_ok());
    }
}
