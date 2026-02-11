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

pub(crate) async fn refresh_certs(pool: &PgPool, tx: &watch::Sender<Arc<HashMap<Id, String>>>) {
    match Proxy::list(pool).await {
        Ok(proxies) => {
            let mut certs = HashMap::new();
            for proxy in proxies {
                let Some(cert) = proxy.certificate else {
                    continue;
                };
                certs.insert(proxy.id, cert);
            }
            let _ = tx.send(Arc::new(certs));
        }
        Err(err) => {
            warn!("Failed to refresh revoked certificate list: {err}");
        }
    }
}

fn root_store_from_ca(ca_cert_der: &[u8]) -> Result<RootCertStore, ProxyError> {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(ca_cert_der.to_vec()))
        .map_err(|err| ProxyError::TlsConfigError(err.to_string()))?;
    Ok(roots)
}

pub(crate) fn client_config(
    ca_cert_der: &[u8],
    certs_rx: watch::Receiver<Arc<HashMap<Id, String>>>,
    proxy_id: Id,
) -> Result<rustls::ClientConfig, ProxyError> {
    let provider = Arc::new(crypto::aws_lc_rs::default_provider());
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
    let verifier: Arc<dyn ServerCertVerifier> = verifier;
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(CertVerifier::new(verifier, certs_rx, proxy_id)));
    Ok(config)
}
