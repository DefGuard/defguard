use std::{collections::HashSet, time::Duration};

use chrono::{NaiveDateTime, TimeDelta, Utc};
use defguard_common::db::{
    Id,
    models::{Certificates, ProxyCertSource, Settings, WireguardNetwork, proxy::Proxy, wireguard::ServiceLocationMode},
};
use defguard_proto::proxy::{AcmeStep, acme_issue_event};
use sqlx::PgPool;
use tokio::{
    sync::{broadcast::Sender, mpsc::{UnboundedSender, unbounded_channel}},
    time::{Instant, sleep},
};
use tracing::Instrument;

use crate::{
    enterprise::{
        db::models::acl::AclRule,
        directory_sync::{do_directory_sync, get_directory_sync_interval},
        firewall::try_get_location_firewall_config,
        is_business_license_active,
        ldap::{do_ldap_sync, sync::get_ldap_sync_interval},
        limits::update_counts,
    }, grpc::GatewayEvent, handlers::component_setup::{ACME_TIMEOUT_SECS, acme_step_name, call_proxy_trigger_acme, parse_cert_expiry}, location_management::allowed_peers::get_location_allowed_peers, updates::do_new_version_check
};

// Times in seconds
const UTILITY_THREAD_MAIN_SLEEP_TIME: Duration = Duration::from_secs(5);
const COUNT_UPDATE_INTERVAL: u64 = 60 * 60;
const UPDATES_CHECK_INTERVAL: u64 = 60 * 60 * 6;
const EXPIRED_ACL_RULES_CHECK_INTERVAL: u64 = 60 * 5;
const ENTERPRISE_STATUS_CHECK_INTERVAL: u64 = 60 * 5;
// const LETSENCRYPT_EXPIRY_CHECK_INTERVAL: u64 = 60 * 60 * 24;
const LETSENCRYPT_EXPIRY_CHECK_INTERVAL: u64 = 60 * 2;
const LETSENCRYPT_EXPIRY_THRESHOLD: TimeDelta = TimeDelta::days(14);
const ACL_EXPIRY_SYSTEM_ACTOR: &str = "system:acl-expiry";

#[instrument(skip_all)]
pub async fn run_utility_thread(
    pool: &PgPool,
    wireguard_tx: Sender<GatewayEvent>,
) -> Result<(), anyhow::Error> {
    let mut last_count_update = Instant::now();
    let mut last_directory_sync = Instant::now();
    let mut last_updates_check = Instant::now();
    let mut last_ldap_sync = Instant::now();
    let mut last_expired_acl_rules_check = Instant::now();
    let mut last_enterprise_status_check = Instant::now();
    let mut last_letsencrypt_expiry_check = Instant::now();

    // helper variable which stores previous enterprise features status
    let mut enterprise_enabled = is_business_license_active();

    let directory_sync_task = || async {
        if let Err(e) = Box::pin(
            do_directory_sync(pool, &wireguard_tx).instrument(info_span!("directory_sync_task")),
        )
        .await
        {
            error!("There was an error while performing directory sync job: {e:?}",);
        }
    };

    let count_update_task = || async {
        if let Err(e) = update_counts(pool)
            .instrument(info_span!("count_update_task"))
            .await
        {
            error!("There was an error while performing count update job: {e:?}");
        }
    };

    let updates_check_task = || async {
        if let Err(e) = do_new_version_check()
            .instrument(info_span!("updates_check_task"))
            .await
        {
            error!("There was an error while checking for new Defguard version: {e:?}");
        }
    };

    let ldap_sync_task = || async {
        if let Err(e) = do_ldap_sync(pool)
            .instrument(info_span!("ldap_sync_task"))
            .await
        {
            error!("There was an error while performing LDAP sync job: {e}");
        }
    };

    let expired_acl_rules_task = || async {
        if let Err(err) = expired_acl_rules_check(pool, wireguard_tx.clone())
            .instrument(info_span!("expired_acl_rules_task"))
            .await
        {
            error!("Failed to check expired ACL rules: {err}");
        }
    };

    let letsencrypt_refresh_task = || async {
        if let Err(e) = do_letsencrypt_refresh(pool)
            .instrument(info_span!("letsencrypt_refresh_task"))
            .await
        {
            error!("There was an error while performing letsencrypt refresh task: {e}");
        }
    };

    directory_sync_task().await;
    count_update_task().await;
    updates_check_task().await;
    ldap_sync_task().await;
    expired_acl_rules_task().await;
    letsencrypt_refresh_task().await;

    loop {
        sleep(UTILITY_THREAD_MAIN_SLEEP_TIME).await;

        // Count update job for updating device/user/network counts
        if last_count_update.elapsed().as_secs() >= COUNT_UPDATE_INTERVAL {
            count_update_task().await;
            last_count_update = Instant::now();
        }

        // Directory sync job for syncing with the directory service
        if last_directory_sync.elapsed().as_secs() >= get_directory_sync_interval(pool).await {
            directory_sync_task().await;
            last_directory_sync = Instant::now();
        }

        // Check for new Defguard version
        if last_updates_check.elapsed().as_secs() >= UPDATES_CHECK_INTERVAL {
            updates_check_task().await;
            last_updates_check = Instant::now();
        }

        // Perform LDAP sync
        if last_ldap_sync.elapsed().as_secs() >= get_ldap_sync_interval() {
            ldap_sync_task().await;
            last_ldap_sync = Instant::now();
        }

        // Mark expired ACL rules
        if last_expired_acl_rules_check.elapsed().as_secs() >= EXPIRED_ACL_RULES_CHECK_INTERVAL {
            expired_acl_rules_task().await;
            last_expired_acl_rules_check = Instant::now();
        }

        // Check if enterprise features got enabled or disabled
        if last_enterprise_status_check.elapsed().as_secs() >= ENTERPRISE_STATUS_CHECK_INTERVAL {
            let new_enterprise_enabled = is_business_license_active();
            if new_enterprise_enabled == enterprise_enabled {
                continue;
            }
            debug!(
                "Enterprise feature status changed from {enterprise_enabled} to \
                    {new_enterprise_enabled}"
            );
            if let Err(err) =
                enterprise_status_check(pool, wireguard_tx.clone(), new_enterprise_enabled)
                    .instrument(info_span!("enterprise_status_check"))
                    .await
            {
                error!("Failed to check enterprise status: {err}");
            } else {
                // update status
                enterprise_enabled = new_enterprise_enabled;
            }
            last_enterprise_status_check = Instant::now();
        }

        // Check LE cert expiry dates and refresh if necessary
        if last_letsencrypt_expiry_check.elapsed().as_secs() >= LETSENCRYPT_EXPIRY_CHECK_INTERVAL {
            letsencrypt_refresh_task().await;
            last_letsencrypt_expiry_check = Instant::now();
        }
    }
}

async fn do_letsencrypt_refresh(pool: &PgPool) -> Result<(), anyhow::Error> {
    debug!("Performing letsencrypt cert validity check");
    let Some(certs) = Certificates::get(pool).await? else {
        warn!("Missing certificates configuration, aborting letsencrypt expiry check");
        return Ok(());
    };

    if certs.proxy_http_cert_source != ProxyCertSource::LetsEncrypt {
        info!("Edge certificate source is {:?}, skipping letsencrypt expiry check", certs.proxy_http_cert_source);
        return Ok(());
    }

    let Some(expiry) = certs.proxy_http_cert_expiry else {
        info!("Edge certificate has no expiry date, skipping letsencrypt refresh certificate refresh");
        return Ok(());
    };

    let expire_in = expiry - Utc::now().naive_utc();
    if expire_in > LETSENCRYPT_EXPIRY_THRESHOLD {
        info!("Letsencrypt certificates expire in {} days, skipping refresh", expire_in.num_days());
        return Ok(());
    }

    info!("Letsencrypt certificates expire in {} days, performing certificate refresh", expire_in.num_days());
    let settings = Settings::get_current_settings();
    let domain = settings.proxy_hostname()?;
    let account_credentials_json = certs.acme_account_credentials.clone().unwrap_or_default();
    let Ok(proxies) = Proxy::list(&pool).await else {
        error!("Failed to load Edge list from DB");
        return Ok(());
    };
    let Some(proxy) = proxies.into_iter().next() else {
        warn!("No Edge found in database, aborting letsencrypt expiry check");
        return Ok(());
     };

    let proxy_host = proxy.address.clone();
    let proxy_port = proxy.port as u16;
    info!(
        "Triggering ACME HTTP-01 via Edge gRPC TriggerAcme for domain: {domain} \
         Edge={proxy_host}:{proxy_port}"
    );

    let (progress_tx, mut progress_rx) =
        unbounded_channel::<AcmeStep>();
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

    let mut current_step: &'static str = "Connecting";
    let deadline = tokio::time::Instant::now()
        + tokio::time::Duration::from_secs(ACME_TIMEOUT_SECS);

    // Drain progress steps until the ACME task finishes (channel closed) or times out.
    loop {
        tokio::select! {
            maybe_step = progress_rx.recv() => {
                match maybe_step {
                    Some(step) => {
                        current_step = acme_step_name(step);
                        // yield Ok(acme_event(current_step));
                    }
                    None => {
                        // progress_tx dropped - ACME task finished; stop polling progress.
                        break;
                    }
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
                    updated_certs.acme_account_credentials =
                        Some(new_account_credentials_json);
                    updated_certs.proxy_http_cert_source =
                        ProxyCertSource::LetsEncrypt;
                    if let Err(e) = updated_certs.save(pool).await {
                        error!( "Failed to save certificate: {e}");
                        // yield Ok(acme_error_event(
                        //     "Installing",
                        //     format!("Failed to save certificate: {e}"),
                        //     None,
                        // ));
                        return Ok(());
                    }
                }
                Err(e) => {
                    error!( "Failed to reload certificates for saving: {e}");
                    // yield Ok(acme_error_event(
                    //     "Installing",
                    //     format!("Failed to reload certificates for saving: {e}"),
                    //     None,
                    // ));
                    return Ok(());
                }
            }

            // TODO(jck): broadcast new certs
            // // Post-wizard: broadcast certs to the proxy via bidi channel.
            // if let Some(ref tx) = proxy_control_tx {
            //     let msg = ProxyControlMessage::BroadcastHttpsCerts {
            //         cert_pem,
            //         key_pem,
            //     };
            //     if let Err(e) = tx.send(msg).await {
            //         error!("Failed to broadcast HttpsCerts to Edge: {e}");
            //     }
            // }

            info!("ACME certificate issued and saved for domain: {domain}");
            // yield Ok(acme_event("Done"));
        }
        Ok(Err((acme_err, logs))) => {
            let msg = format!("ACME issuance failed: {acme_err}");
            error!("{msg}");
            // yield Ok(acme_error_event(current_step, msg, Some(logs)));
        }
        Err(_) => {
            error!( "ACME task terminated unexpectedly.");
            // yield Ok(acme_error_event(
            //     current_step,
            //     "ACME task terminated unexpectedly.".to_string(),
            //     None,
            // ));
        }
    }

    Ok(())
}

/// Check if enterprise status has changed and perform any necessary actions
async fn enterprise_status_check(
    pool: &PgPool,
    wireguard_tx: Sender<GatewayEvent>,
    enable_enterprise: bool,
) -> Result<(), anyhow::Error> {
    // fetch all ACL-enabled networks
    let locations = WireguardNetwork::all(pool)
        .await?
        .into_iter()
        .filter(|location| location.acl_enabled)
        .collect::<Vec<_>>();

    if enable_enterprise {
        // handle switch from disabled -> enabled
        debug!("Re-enabling gateway firewall configuration for ACL-enabled locations");
        let mut transaction = pool.begin().await?;
        for location in locations {
            debug!("Re-enabling gateway firewall configuration for location {location:?}");
            let firewall_config = try_get_location_firewall_config(&location, &mut transaction)
                .await?
                .expect("ACL-enabled location must have firewall config");

            // Handle service location update or just update the firewall
            if location.service_location_mode == ServiceLocationMode::Disabled {
                wireguard_tx.send(GatewayEvent::FirewallConfigChanged(
                    location.id,
                    firewall_config,
                ))?;
            } else {
                let new_peers = get_location_allowed_peers(&location, &mut *transaction).await?;
                wireguard_tx.send(GatewayEvent::NetworkModified(
                    location.id,
                    location,
                    new_peers,
                    Some(firewall_config),
                ))?;
            }
        }
        transaction.commit().await?;
    } else {
        // handle switch from enabled -> disabled
        debug!("Disabling gateway firewall configuration for ACL-enabled locations");
        for location in locations {
            if location.service_location_mode == ServiceLocationMode::Disabled {
                debug!("Disabling gateway firewall configuration for location {location:?}");
                wireguard_tx.send(GatewayEvent::FirewallDisabled(location.id))?;
            } else {
                debug!(
                    "Disabling gateway firewall configuration and service location client \
                    connections for location {location}"
                );
                wireguard_tx.send(GatewayEvent::NetworkModified(
                    location.id,
                    location,
                    // Send empty peer list, we are disabling the service location
                    Vec::new(),
                    None,
                ))?;
            }
        }
    }

    Ok(())
}

/// Find newly expired ACL rules and update their status.
async fn expired_acl_rules_check(
    pool: &PgPool,
    wireguard_tx: Sender<GatewayEvent>,
) -> Result<(), anyhow::Error> {
    // mark relevant rules as expired
    let updated_rules = sqlx::query_as::<_, AclRule<Id>>(
        "UPDATE aclrule SET state = 'expired'::aclrule_state, modified_at = $1, modified_by = $2 \
        WHERE state = 'applied'::aclrule_state AND expires < NOW() \
        RETURNING id, parent_id, state, name, allow_all_users, deny_all_users, allow_all_groups, \
        deny_all_groups, allow_all_network_devices, deny_all_network_devices, all_locations, \
        addresses, ports, protocols, enabled, expires, any_address, any_port, any_protocol, \
        use_manual_destination_settings, modified_at, modified_by",
    )
    .bind(Utc::now().naive_utc())
    .bind(ACL_EXPIRY_SYSTEM_ACTOR)
    .fetch_all(pool)
    .await?;

    // send firewall config updates to locations which have been affected by updated
    // rules
    debug!(
        "Marked {} ACL rules as expired. Sending firewall config updates to affected locations.",
        updated_rules.len()
    );

    // find affected locations
    let mut affected_locations = HashSet::new();
    for rule in updated_rules {
        let locations = rule.get_networks(pool).await?;
        for location in locations {
            affected_locations.insert(location);
        }
    }

    let affected_locations: Vec<WireguardNetwork<Id>> = affected_locations.into_iter().collect();
    debug!(
        "{} locations affected by expired ACL rules. Sending gateway firewall update events \
            for each location",
        affected_locations.len()
    );

    let mut conn = pool.acquire().await?;
    for location in affected_locations {
        match try_get_location_firewall_config(&location, &mut conn).await? {
            Some(firewall_config) => {
                debug!("Sending firewall update event for location {location}");
                wireguard_tx.send(GatewayEvent::FirewallConfigChanged(
                    location.id,
                    firewall_config,
                ))?;
            }
            None => {
                debug!(
                    "No firewall config generated for location {location}. Not sending a \
                    gateway event"
                );
            }
        }
    }

    Ok(())
}
