use std::time::Duration;

use chrono::{NaiveDateTime, TimeDelta, Utc};
use defguard_certs::der_to_pem;
use defguard_common::{
    VERSION,
    db::models::{Certificates, ProxyCertSource, Settings, User, proxy::Proxy},
    types::proxy::ProxyControlMessage,
};
use defguard_mail::templates;
use defguard_proto::proxy::{
    AcmeChallenge, AcmeLogs, AcmeStep, acme_issue_event, proxy_client::ProxyClient,
};
use defguard_version::{Version, client::ClientVersionInterceptor};
use sqlx::PgPool;
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedSender, unbounded_channel};
use tonic::{
    Request,
    service::Interceptor,
    transport::{Certificate, ClientTlsConfig, Endpoint},
};

/// Maximum time (seconds) allowed for the ACME flow to complete end-to-end.
#[cfg(not(test))]
pub const ACME_TIMEOUT_SECS: u64 = 300;
#[cfg(test)]
pub const ACME_TIMEOUT_SECS: u64 = 1;
const LETSENCRYPT_EXPIRY_THRESHOLD: TimeDelta = TimeDelta::days(14);

#[derive(Debug, Error)]
pub(crate) enum LetsencryptError {
    #[error("Failed to load certificates: {0}")]
    CertificatesLoadFailed(sqlx::Error),
    #[error("Failed to resolve proxy hostname: {0}")]
    ProxyHostnameFailed(String),
    #[error("Failed to load Edge list from DB: {0}")]
    ProxyListLoadFailed(sqlx::Error),
    #[error("No Edge found in database")]
    NoProxyFound,
    #[error("ACME certificate issuance timed out after {timeout_secs} seconds")]
    AcmeTimedOut { timeout_secs: u64 },
    #[error("Failed to reload certificates for saving: {0}")]
    CertificateReloadFailed(sqlx::Error),
    #[error("Failed to save certificate: {0}")]
    CertificateSaveFailed(sqlx::Error),
    #[error("ACME issuance failed: {0}")]
    AcmeIssuanceFailed(String),
}

/// Refreshes the proxy HTTPS certificate through the Edge ACME flow when the
/// currently stored Let's Encrypt certificate is close to expiry.
///
/// Returns `Ok(())` when refresh is not needed or when renewal completes
/// successfully. Returns [`LetsencryptError`] only for operational failures in
/// the refresh flow itself.
pub(crate) async fn do_letsencrypt_refresh(
    pool: &PgPool,
    proxy_control_tx: mpsc::Sender<ProxyControlMessage>,
) -> Result<(), LetsencryptError> {
    debug!("Performing letsencrypt cert validity check");
    let Some(certs) = Certificates::get(pool)
        .await
        .map_err(LetsencryptError::CertificatesLoadFailed)?
    else {
        warn!("Missing certificates configuration, aborting letsencrypt expiry check");
        return Ok(());
    };

    if certs.proxy_http_cert_source != ProxyCertSource::LetsEncrypt {
        info!(
            "Edge certificate source is {:?}, skipping Letsencrypt expiry check",
            certs.proxy_http_cert_source
        );
        return Ok(());
    }

    let Some(expiry) = certs.proxy_http_cert_expiry else {
        info!(
            "Edge certificate has no expiry date, skipping Letsencrypt refresh certificate refresh"
        );
        return Ok(());
    };

    let expire_in = expiry - Utc::now().naive_utc();
    if expire_in > LETSENCRYPT_EXPIRY_THRESHOLD {
        info!(
            "Letsencrypt certificate expires in {} days, skipping refresh",
            expire_in.num_days()
        );
        return Ok(());
    }

    info!(
        "Letsencrypt certificate expires in {} days, performing certificate refresh",
        expire_in.num_days()
    );
    let settings = Settings::get_current_settings();
    let domain = settings
        .proxy_hostname()
        .map_err(|err| LetsencryptError::ProxyHostnameFailed(err.to_string()))?;
    let account_credentials_json = certs.acme_account_credentials.clone().unwrap_or_default();
    let proxies = Proxy::list(pool)
        .await
        .map_err(LetsencryptError::ProxyListLoadFailed)?;
    let Some(proxy) = proxies.into_iter().next() else {
        warn!("No Edge found in database, aborting Letsencrypt expiry check");
        return Err(LetsencryptError::NoProxyFound);
    };

    let proxy_host = proxy.address.clone();
    let proxy_port = proxy.port as u16;
    info!(
        "Triggering ACME HTTP-01 via Edge gRPC TriggerAcme for domain: {domain} \
         Edge={proxy_host}:{proxy_port}"
    );

    let (progress_tx, _progress_rx) = unbounded_channel::<AcmeStep>();

    match tokio::time::timeout(
        tokio::time::Duration::from_secs(ACME_TIMEOUT_SECS),
        call_proxy_trigger_acme(
            pool,
            &proxy_host,
            proxy_port,
            domain.clone(),
            account_credentials_json,
            progress_tx,
        ),
    )
    .await
    {
        Ok(Ok((cert_pem, key_pem, new_account_credentials_json))) => {
            let acme_cert_expiry = parse_cert_expiry(&cert_pem);
            match Certificates::get_or_default(pool).await {
                Ok(mut updated_certs) => {
                    updated_certs.acme_domain = Some(domain.clone());
                    updated_certs.proxy_http_cert_pem = Some(cert_pem.clone());
                    updated_certs.proxy_http_cert_key_pem = Some(key_pem.clone());
                    updated_certs.proxy_http_cert_expiry = acme_cert_expiry;
                    updated_certs.acme_account_credentials = Some(new_account_credentials_json);
                    updated_certs.proxy_http_cert_source = ProxyCertSource::LetsEncrypt;
                    if let Err(e) = updated_certs.save(pool).await {
                        error!("Failed to save certificate: {e}");
                        return Err(LetsencryptError::CertificateSaveFailed(e));
                    }
                }
                Err(e) => {
                    error!("Failed to reload certificates for saving: {e}");
                    return Err(LetsencryptError::CertificateReloadFailed(e));
                }
            }

            // Broadcast certs to the proxy via bidi channel
            let msg = ProxyControlMessage::BroadcastHttpsCerts { cert_pem, key_pem };
            if let Err(e) = proxy_control_tx.send(msg).await {
                error!("Failed to broadcast HttpsCerts to Edge: {e}");
            }

            info!("ACME certificate issued and saved for domain: {domain}");
        }
        Ok(Err((acme_err, logs))) => {
            error!("ACME issuance failed: {acme_err}");
            if let Err(err) = send_le_refresh_failed_emails(pool, &acme_err, &logs).await {
                error!("Sending letsencrypt refresh email notification failed: {err}");
            }
            return Err(LetsencryptError::AcmeIssuanceFailed(acme_err));
        }
        Err(_) => {
            error!(
                "ACME certificate issuance timed out after \
                 {ACME_TIMEOUT_SECS} seconds."
            );
            return Err(LetsencryptError::AcmeTimedOut {
                timeout_secs: ACME_TIMEOUT_SECS,
            });
        }
    }

    Ok(())
}

/// Sends a failed Let's Encrypt refresh notification email to all active
/// administrators.
///
/// The provided log lines are joined into a single text attachment and sent
/// with the notification email.
async fn send_le_refresh_failed_emails(
    pool: &PgPool,
    error_message: &str,
    logs: &[String],
) -> Result<(), anyhow::Error> {
    let mut conn = pool.begin().await?;
    let admin_users = User::find_admins(&mut *conn).await?;
    for user in admin_users {
        templates::letsencrypt_cert_refresh_failed_mail(
            &user.email,
            &mut conn,
            error_message,
            &logs.join("\n"),
        )
        .await?;
    }

    Ok(())
}

/// Parses the expiry timestamp from a PEM-encoded certificate.
///
/// Returns the certificate `not_after` value, or `None` if the PEM cannot be
/// parsed or the expiry cannot be extracted.
pub(crate) fn parse_cert_expiry(cert_pem: &str) -> Option<NaiveDateTime> {
    let der = defguard_certs::parse_pem_certificate(cert_pem)
        .map_err(|e| warn!("Failed to parse ACME cert PEM for expiry: {e}"))
        .ok()?;
    defguard_certs::CertificateInfo::from_der(&der)
        .map(|info| info.not_after)
        .map_err(|e| warn!("Failed to extract expiry from ACME cert: {e}"))
        .ok()
}

/// Maps a proto [`AcmeStep`] to the SSE step string expected by the frontend.
pub(crate) fn acme_step_name(step: AcmeStep) -> &'static str {
    match step {
        AcmeStep::Unspecified | AcmeStep::Connecting => "Connecting",
        AcmeStep::CheckingDomain => "CheckingDomain",
        AcmeStep::ValidatingDomain => "ValidatingDomain",
        AcmeStep::IssuingCertificate => "IssuingCertificate",
    }
}

/// Connects to the proxy's permanent `Proxy` gRPC service and calls `TriggerAcme`.
///
/// Returns `(cert_pem, key_pem, account_credentials_json)` on success, or
/// `(error_message, log_lines)` on failure where `log_lines` are the proxy log entries
/// collected during the ACME run (sent by the proxy via an [`AcmeLogs`] event).
pub(crate) async fn call_proxy_trigger_acme(
    pool: &PgPool,
    proxy_host: &str,
    proxy_port: u16,
    domain: String,
    account_credentials_json: String,
    progress_tx: UnboundedSender<AcmeStep>,
) -> Result<(String, String, String), (String, Vec<String>)> {
    let certs = Certificates::get_or_default(pool)
        .await
        .map_err(|e| (format!("Failed to load certificates: {e}"), Vec::new()))?;
    let ca_cert_der = certs.ca_cert_der.ok_or_else(|| {
        (
            "CA certificate not found in settings".to_string(),
            Vec::new(),
        )
    })?;

    let cert_pem = der_to_pem(&ca_cert_der, defguard_certs::PemLabel::Certificate)
        .map_err(|e| (format!("Failed to convert CA cert to PEM: {e}"), Vec::new()))?;

    let endpoint_str = format!("https://{proxy_host}:{proxy_port}");
    let endpoint = Endpoint::from_shared(endpoint_str)
        .map_err(|e| (format!("Failed to build Edge endpoint: {e}"), Vec::new()))?
        .http2_keep_alive_interval(Duration::from_secs(5))
        .tcp_keepalive(Some(Duration::from_secs(5)))
        .keep_alive_while_idle(true);

    let tls = ClientTlsConfig::new().ca_certificate(Certificate::from_pem(cert_pem));
    let endpoint = endpoint.tls_config(tls).map_err(|e| {
        (
            format!("Failed to configure TLS for Edge endpoint: {e}"),
            Vec::new(),
        )
    })?;

    let version = Version::parse(VERSION)
        .map_err(|e| (format!("Failed to parse core version: {e}"), Vec::new()))?;
    let version_interceptor = ClientVersionInterceptor::new(version);

    let mut client =
        ProxyClient::with_interceptor(endpoint.connect_lazy(), move |req: Request<()>| {
            version_interceptor.clone().call(req)
        });

    let mut stream = client
        .trigger_acme(AcmeChallenge {
            domain: domain.clone(),
            account_credentials_json,
        })
        .await
        .map_err(|e| (format!("TriggerAcme RPC failed: {e}"), Vec::new()))?
        .into_inner();

    let mut collected_logs: Vec<String> = Vec::new();

    loop {
        match stream.message().await {
            Ok(Some(event)) => match event.payload {
                Some(acme_issue_event::Payload::Progress(p)) => {
                    if let Ok(step) = AcmeStep::try_from(p.step) {
                        let _ = progress_tx.send(step);
                    }
                }
                Some(acme_issue_event::Payload::Certificate(cert)) => {
                    return Ok((cert.cert_pem, cert.key_pem, cert.account_credentials_json));
                }
                Some(acme_issue_event::Payload::Logs(AcmeLogs { lines })) => {
                    collected_logs = lines;
                }
                None => {
                    return Err((
                        "TriggerAcme stream sent an event with no payload".to_string(),
                        collected_logs,
                    ));
                }
            },
            Ok(None) => {
                return Err((
                    "TriggerAcme stream ended without delivering a certificate".to_string(),
                    collected_logs,
                ));
            }
            Err(e) => {
                return Err((
                    format!("Failed to read TriggerAcme response: {e}"),
                    collected_logs,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        pin::Pin,
        sync::Arc,
        sync::Once,
        time::Duration,
    };

    use defguard_certs::{CertificateAuthority, Csr, DnType, PemLabel, generate_key_pair};
    use defguard_common::{
        db::{
            models::{Certificates, ProxyCertSource, Settings, User, proxy::Proxy},
            setup_pool,
        },
        secret::SecretStringWrapper,
        types::proxy::ProxyControlMessage,
    };
    use defguard_proto::proxy::{
        AcmeCertificate, AcmeIssueEvent, AcmeLogs, AcmeProgress, AcmeStep, proxy_server,
    };
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use std::str::FromStr;
    use tokio::{
        net::TcpListener,
        sync::{Mutex, mpsc},
        task::JoinHandle,
        time::{sleep, timeout},
    };
    use tokio_stream::{self as stream};
    use tonic::{
        Request, Response, Status, Streaming,
        transport::{Identity, Server, ServerTlsConfig},
    };

    use super::{ACME_TIMEOUT_SECS, LetsencryptError, do_letsencrypt_refresh};

    const TEST_ACCOUNT_JSON: &str = r#"{"account_url":"https://acme.example/account/1"}"#;

    enum MockAcmeBehavior {
        Success {
            cert_pem: String,
            key_pem: String,
            account_credentials_json: String,
            logs: Vec<String>,
        },
        RpcError(Status),
        Hang,
    }

    struct MockProxyService {
        behavior: Arc<Mutex<MockAcmeBehavior>>,
    }

    #[tonic::async_trait]
    impl proxy_server::Proxy for MockProxyService {
        type BidiStream = Pin<
            Box<
                dyn tokio_stream::Stream<Item = Result<defguard_proto::proxy::CoreRequest, Status>>
                    + Send,
            >,
        >;
        type TriggerAcmeStream =
            Pin<Box<dyn tokio_stream::Stream<Item = Result<AcmeIssueEvent, Status>> + Send>>;

        async fn bidi(
            &self,
            _request: Request<Streaming<defguard_proto::proxy::CoreResponse>>,
        ) -> Result<Response<Self::BidiStream>, Status> {
            Ok(Response::new(Box::pin(stream::empty())))
        }

        async fn purge(&self, _request: Request<()>) -> Result<Response<()>, Status> {
            Ok(Response::new(()))
        }

        async fn trigger_acme(
            &self,
            _request: Request<defguard_proto::proxy::AcmeChallenge>,
        ) -> Result<Response<Self::TriggerAcmeStream>, Status> {
            let behavior = self.behavior.lock().await;
            match &*behavior {
                MockAcmeBehavior::Success {
                    cert_pem,
                    key_pem,
                    account_credentials_json,
                    logs,
                } => {
                    let mut events = vec![Ok(AcmeIssueEvent {
                        payload: Some(defguard_proto::proxy::acme_issue_event::Payload::Progress(
                            AcmeProgress {
                                step: AcmeStep::CheckingDomain as i32,
                            },
                        )),
                    })];
                    if !logs.is_empty() {
                        events.push(Ok(AcmeIssueEvent {
                            payload: Some(defguard_proto::proxy::acme_issue_event::Payload::Logs(
                                AcmeLogs {
                                    lines: logs.clone(),
                                },
                            )),
                        }));
                    }
                    events.push(Ok(AcmeIssueEvent {
                        payload: Some(
                            defguard_proto::proxy::acme_issue_event::Payload::Certificate(
                                AcmeCertificate {
                                    cert_pem: cert_pem.clone(),
                                    key_pem: key_pem.clone(),
                                    account_credentials_json: account_credentials_json.clone(),
                                },
                            ),
                        ),
                    }));
                    Ok(Response::new(Box::pin(stream::iter(events))))
                }
                MockAcmeBehavior::RpcError(status) => Err(status.clone()),
                MockAcmeBehavior::Hang => Ok(Response::new(Box::pin(stream::pending::<
                    Result<AcmeIssueEvent, Status>,
                >()))),
            }
        }
    }

    struct MockAcmeServer {
        port: u16,
        task: JoinHandle<()>,
    }

    impl MockAcmeServer {
        async fn start(
            ca: &CertificateAuthority<'_>,
            common_name: &str,
            behavior: MockAcmeBehavior,
        ) -> Self {
            init_rustls_crypto_provider();
            let identity = make_server_identity(ca, common_name);
            let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
                .await
                .expect("failed to bind mock ACME server");
            let port = listener.local_addr().expect("missing local addr").port();
            let service = MockProxyService {
                behavior: Arc::new(Mutex::new(behavior)),
            };
            let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);
            let task = tokio::spawn(async move {
                Server::builder()
                    .tls_config(ServerTlsConfig::new().identity(identity))
                    .expect("failed to configure TLS for mock ACME server")
                    .add_service(proxy_server::ProxyServer::new(service))
                    .serve_with_incoming(incoming)
                    .await
                    .expect("mock ACME server failed");
            });

            tokio::task::yield_now().await;

            Self { port, task }
        }
    }

    impl Drop for MockAcmeServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    fn make_server_identity(ca: &CertificateAuthority<'_>, common_name: &str) -> Identity {
        let key_pair = generate_key_pair().expect("failed to generate key pair");
        let san = vec![common_name.to_string()];
        let dn = vec![(DnType::CommonName, common_name)];
        let csr = Csr::new(&key_pair, &san, dn).expect("failed to create CSR");
        let cert = ca.sign_csr(&csr).expect("failed to sign server cert");
        let cert_pem =
            defguard_certs::der_to_pem(cert.der(), PemLabel::Certificate).expect("cert PEM");
        let key_pem =
            defguard_certs::der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)
                .expect("key PEM");
        Identity::from_pem(cert_pem, key_pem)
    }

    fn init_rustls_crypto_provider() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            rustls::crypto::ring::default_provider()
                .install_default()
                .ok();
        });
    }

    async fn seed_settings(pool: &sqlx::PgPool, hostname: &str) {
        defguard_common::db::models::settings::initialize_current_settings(pool)
            .await
            .expect("failed to initialize settings");
        let mut settings = Settings::get_current_settings();
        settings.public_proxy_url = format!("https://{hostname}");
        settings.smtp_server = Some("smtp.example.com".into());
        settings.smtp_port = Some(587);
        settings.smtp_sender = Some("noreply@example.com".into());
        settings.smtp_user = Some(String::new());
        settings.smtp_password = Some(SecretStringWrapper::from_str("").unwrap());
        defguard_common::db::models::settings::set_settings(Some(settings));
    }

    async fn seed_admin(pool: &sqlx::PgPool) {
        let _ = User::new("admin", None, "Admin", "User", "admin@example.com", None)
            .save(pool)
            .await
            .expect("failed to save admin user");
    }

    fn make_ca() -> CertificateAuthority<'static> {
        CertificateAuthority::new("Test CA", "test@example.com", 365).expect("failed to create CA")
    }

    async fn seed_ca(pool: &sqlx::PgPool, ca: &CertificateAuthority<'_>) {
        Certificates {
            ca_cert_der: Some(ca.cert_der().to_vec()),
            ca_key_der: Some(ca.key_pair_der().to_vec()),
            ca_expiry: Some(ca.expiry().expect("missing CA expiry")),
            ..Default::default()
        }
        .save(pool)
        .await
        .expect("failed to save CA certs");
    }

    async fn seed_letsencrypt_cert(
        pool: &sqlx::PgPool,
        ca: &CertificateAuthority<'_>,
        common_name: &str,
        valid_for_days: i64,
    ) {
        let key_pair = generate_key_pair().expect("failed to generate key pair");
        let san = vec![common_name.to_string()];
        let dn = vec![(DnType::CommonName, common_name)];
        let csr = Csr::new(&key_pair, &san, dn).expect("failed to create CSR");
        let cert = ca
            .sign_csr_with_validity(&csr, valid_for_days)
            .expect("failed to sign cert");
        let cert_pem =
            defguard_certs::der_to_pem(cert.der(), PemLabel::Certificate).expect("cert PEM");
        let key_pem =
            defguard_certs::der_to_pem(key_pair.serialize_der().as_slice(), PemLabel::PrivateKey)
                .expect("key PEM");
        let expiry = super::parse_cert_expiry(&cert_pem).expect("expected cert expiry");

        let mut certs = Certificates::get_or_default(pool)
            .await
            .expect("failed to load certificates");
        certs.proxy_http_cert_source = ProxyCertSource::LetsEncrypt;
        certs.proxy_http_cert_pem = Some(cert_pem);
        certs.proxy_http_cert_key_pem = Some(key_pem);
        certs.proxy_http_cert_expiry = Some(expiry);
        certs.acme_account_credentials = Some(TEST_ACCOUNT_JSON.to_string());
        certs.save(pool).await.expect("failed to save LE certs");
    }

    async fn create_proxy(pool: &sqlx::PgPool, address: &str, port: u16) {
        let mut proxy = Proxy::new("test-proxy", address, i32::from(port), "tester");
        proxy.enabled = true;
        proxy.save(pool).await.expect("failed to save proxy");
    }

    async fn drain_broadcasts(
        rx: &mut mpsc::Receiver<ProxyControlMessage>,
    ) -> Vec<(String, String)> {
        sleep(Duration::from_millis(50)).await;
        let mut broadcasts = Vec::new();
        while let Ok(message) = rx.try_recv() {
            if let ProxyControlMessage::BroadcastHttpsCerts { cert_pem, key_pem } = message {
                broadcasts.push((cert_pem, key_pem));
            }
        }
        broadcasts
    }

    #[sqlx::test]
    async fn letsencrypt_refresh_skips_when_certificate_not_due(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let ca = make_ca();
        seed_settings(&pool, "refresh.example.com").await;
        seed_ca(&pool, &ca).await;
        seed_letsencrypt_cert(&pool, &ca, "refresh.example.com", 89).await;

        let certs_before = Certificates::get_or_default(&pool)
            .await
            .expect("failed to load certificates");

        let (proxy_control_tx, mut proxy_control_rx) = mpsc::channel(8);
        let result = do_letsencrypt_refresh(&pool, proxy_control_tx).await;

        assert!(result.is_ok(), "expected skip to succeed, got {result:?}");

        let certs_after = Certificates::get_or_default(&pool)
            .await
            .expect("failed to reload certificates");
        assert_eq!(
            certs_after.proxy_http_cert_pem,
            certs_before.proxy_http_cert_pem
        );
        assert_eq!(
            certs_after.proxy_http_cert_key_pem,
            certs_before.proxy_http_cert_key_pem
        );
        assert!(drain_broadcasts(&mut proxy_control_rx).await.is_empty());
    }

    #[sqlx::test]
    async fn letsencrypt_refresh_returns_no_proxy_found_when_due(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let ca = make_ca();
        seed_settings(&pool, "refresh.example.com").await;
        seed_ca(&pool, &ca).await;
        seed_letsencrypt_cert(&pool, &ca, "refresh.example.com", 1).await;

        let (proxy_control_tx, _proxy_control_rx) = mpsc::channel(8);
        let result = do_letsencrypt_refresh(&pool, proxy_control_tx).await;

        assert!(matches!(result, Err(LetsencryptError::NoProxyFound)));
    }

    #[sqlx::test]
    async fn letsencrypt_refresh_success_persists_certificate_and_broadcasts(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let ca = make_ca();
        seed_settings(&pool, "localhost").await;
        seed_ca(&pool, &ca).await;
        seed_letsencrypt_cert(&pool, &ca, "localhost", 1).await;

        let (new_cert_pem, new_key_pem) = {
            let key_pair = generate_key_pair().expect("failed to generate key pair");
            let san = vec!["localhost".to_string()];
            let dn = vec![(DnType::CommonName, "localhost")];
            let csr = Csr::new(&key_pair, &san, dn).expect("failed to create CSR");
            let cert = ca.sign_csr(&csr).expect("failed to sign cert");
            (
                defguard_certs::der_to_pem(cert.der(), PemLabel::Certificate).expect("cert PEM"),
                defguard_certs::der_to_pem(
                    key_pair.serialize_der().as_slice(),
                    PemLabel::PrivateKey,
                )
                .expect("key PEM"),
            )
        };

        let mock_server = MockAcmeServer::start(
            &ca,
            "localhost",
            MockAcmeBehavior::Success {
                cert_pem: new_cert_pem.clone(),
                key_pem: new_key_pem.clone(),
                account_credentials_json: r#"{"account_url":"https://acme.example/account/2"}"#
                    .to_string(),
                logs: vec!["proxy log line".to_string()],
            },
        )
        .await;
        create_proxy(&pool, "localhost", mock_server.port).await;

        let (proxy_control_tx, mut proxy_control_rx) = mpsc::channel(8);
        let result = do_letsencrypt_refresh(&pool, proxy_control_tx).await;

        assert!(
            result.is_ok(),
            "expected successful refresh, got {result:?}"
        );

        let certs = Certificates::get_or_default(&pool)
            .await
            .expect("failed to reload certificates");
        assert_eq!(
            certs.proxy_http_cert_pem.as_deref(),
            Some(new_cert_pem.as_str())
        );
        assert_eq!(
            certs.proxy_http_cert_key_pem.as_deref(),
            Some(new_key_pem.as_str())
        );
        assert_eq!(
            certs.acme_account_credentials.as_deref(),
            Some(r#"{"account_url":"https://acme.example/account/2"}"#)
        );
        assert_eq!(certs.acme_domain.as_deref(), Some("localhost"));
        assert_eq!(certs.proxy_http_cert_source, ProxyCertSource::LetsEncrypt);
        assert!(certs.proxy_http_cert_expiry.is_some());

        let broadcasts = drain_broadcasts(&mut proxy_control_rx).await;
        assert_eq!(broadcasts.len(), 1);
        assert_eq!(broadcasts[0].0, new_cert_pem);
        assert_eq!(broadcasts[0].1, new_key_pem);
    }

    #[sqlx::test]
    async fn letsencrypt_refresh_returns_acme_issuance_failed_on_rpc_error(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let ca = make_ca();
        seed_settings(&pool, "localhost").await;
        seed_ca(&pool, &ca).await;
        seed_admin(&pool).await;
        seed_letsencrypt_cert(&pool, &ca, "localhost", 1).await;

        let mock_server = MockAcmeServer::start(
            &ca,
            "localhost",
            MockAcmeBehavior::RpcError(Status::unavailable("rpc unavailable")),
        )
        .await;
        create_proxy(&pool, "localhost", mock_server.port).await;

        let (proxy_control_tx, _proxy_control_rx) = mpsc::channel(8);
        let result = do_letsencrypt_refresh(&pool, proxy_control_tx).await;

        assert!(matches!(
            result,
            Err(LetsencryptError::AcmeIssuanceFailed(message)) if message.contains("TriggerAcme RPC failed")
        ));
    }

    #[sqlx::test]
    async fn letsencrypt_refresh_returns_acme_timed_out_when_stream_hangs(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let ca = make_ca();
        seed_settings(&pool, "localhost").await;
        seed_ca(&pool, &ca).await;
        seed_admin(&pool).await;
        seed_letsencrypt_cert(&pool, &ca, "localhost", 1).await;

        let mock_server = MockAcmeServer::start(&ca, "localhost", MockAcmeBehavior::Hang).await;
        create_proxy(&pool, "localhost", mock_server.port).await;

        let (proxy_control_tx, _proxy_control_rx) = mpsc::channel(8);
        let result = timeout(
            Duration::from_secs(ACME_TIMEOUT_SECS + 5),
            do_letsencrypt_refresh(&pool, proxy_control_tx),
        )
        .await
        .expect("refresh should finish before outer timeout");

        assert!(matches!(
            result,
            Err(LetsencryptError::AcmeTimedOut { timeout_secs }) if timeout_secs == ACME_TIMEOUT_SECS
        ));

        drop(mock_server);
        sleep(Duration::from_millis(50)).await;
    }
}
