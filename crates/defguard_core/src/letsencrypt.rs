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
pub const ACME_TIMEOUT_SECS: u64 = 300;
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
    #[error("ACME task terminated unexpectedly")]
    AcmeTaskTerminatedUnexpectedly,
}

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

    let (progress_tx, mut progress_rx) = unbounded_channel::<AcmeStep>();
    let (result_tx, result_rx) =
        tokio::sync::oneshot::channel::<Result<(String, String, String), (String, Vec<String>)>>();

    let pool_clone = pool.clone();
    let domain_clone = domain.clone();
    let acct_creds_clone = account_credentials_json.clone();
    tokio::spawn(async move {
        let result = call_proxy_trigger_acme(
            &pool_clone,
            &proxy_host,
            proxy_port,
            domain_clone,
            acct_creds_clone,
            progress_tx,
        )
        .await;
        let _ = result_tx.send(result);
    });

    let deadline =
        tokio::time::Instant::now() + tokio::time::Duration::from_secs(ACME_TIMEOUT_SECS);

    // Drain progress steps until the ACME task finishes (channel closed) or times out.
    loop {
        tokio::select! {
            maybe_step = progress_rx.recv() => {
                if maybe_step.is_none() {
                    // progress_tx dropped - ACME task finished; stop polling progress.
                    break;
                }
            }

            () = tokio::time::sleep_until(deadline) => {
                error!(
                    "ACME certificate issuance timed out after \
                     {ACME_TIMEOUT_SECS} seconds."
                );
                return Err(LetsencryptError::AcmeTimedOut {
                    timeout_secs: ACME_TIMEOUT_SECS,
                });
            }
        }
    }

    // Progress channel closed - collect the final result.
    match result_rx.await {
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
            if let Err(err) = send_le_refresh_failed_emails(pool, &domain, &logs).await {
                error!("Sending letsencrypt refresh email notification failed: {err}");
            }
            return Err(LetsencryptError::AcmeIssuanceFailed(acme_err));
        }
        Err(_) => {
            error!("ACME task terminated unexpectedly.");
            return Err(LetsencryptError::AcmeTaskTerminatedUnexpectedly);
        }
    }

    Ok(())
}

async fn send_le_refresh_failed_emails(
    pool: &PgPool,
    domain: &str,
    logs: &[String],
) -> Result<(), anyhow::Error> {
    let mut conn = pool.begin().await?;
    let admin_users = User::find_admins(&mut *conn).await?;
    for user in admin_users {
        templates::letsencrypt_cert_refresh_failed_mail(
            &user.email,
            &mut conn,
            domain,
            &logs.join("\n"),
        )
        .await?;
    }

    Ok(())
}

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
