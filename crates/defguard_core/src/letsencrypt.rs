use chrono::{TimeDelta, Utc};
use defguard_common::{
    db::models::{Certificates, ProxyCertSource, Settings, User, proxy::Proxy},
    types::proxy::ProxyControlMessage,
};
use defguard_mail::templates;
use defguard_proto::proxy::AcmeStep;
use sqlx::PgPool;
use tokio::sync::mpsc::{self, unbounded_channel};

use crate::handlers::component_setup::{
    ACME_TIMEOUT_SECS, call_proxy_trigger_acme, parse_cert_expiry,
};

const LETSENCRYPT_EXPIRY_THRESHOLD: TimeDelta = TimeDelta::days(14);

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

pub(crate) async fn do_letsencrypt_refresh(
    pool: &PgPool,
    proxy_control_tx: mpsc::Sender<ProxyControlMessage>,
) -> Result<(), anyhow::Error> {
    debug!("Performing letsencrypt cert validity check");
    let Some(certs) = Certificates::get(pool).await? else {
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
    let domain = settings.proxy_hostname()?;
    let account_credentials_json = certs.acme_account_credentials.clone().unwrap_or_default();
    let Ok(proxies) = Proxy::list(pool).await else {
        error!("Failed to load Edge list from DB");
        return Ok(());
    };
    let Some(proxy) = proxies.into_iter().next() else {
        warn!("No Edge found in database, aborting Letsencrypt expiry check");
        return Ok(());
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
                return Ok(());
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
                        return Ok(());
                    }
                }
                Err(e) => {
                    error!("Failed to reload certificates for saving: {e}");
                    return Ok(());
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
            return Ok(());
        }
        Err(_) => {
            error!("ACME task terminated unexpectedly.");
            return Ok(());
        }
    }

    Ok(())
}
