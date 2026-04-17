use std::{collections::HashSet, time::Duration};

use chrono::{NaiveDateTime, TimeDelta, Utc};
use defguard_common::db::models::{Certificates, WireguardNetwork, wireguard::ServiceLocationMode};
use sqlx::{PgPool, query_as};
use tokio::{
    sync::broadcast::Sender,
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
    },
    grpc::GatewayEvent,
    location_management::allowed_peers::get_location_allowed_peers,
    updates::do_new_version_check,
};

// Times in seconds
const UTILITY_THREAD_MAIN_SLEEP_TIME: Duration = Duration::from_secs(5);
const COUNT_UPDATE_INTERVAL: u64 = 60 * 60;
const UPDATES_CHECK_INTERVAL: u64 = 60 * 60 * 6;
const EXPIRED_ACL_RULES_CHECK_INTERVAL: u64 = 60 * 5;
const ENTERPRISE_STATUS_CHECK_INTERVAL: u64 = 60 * 5;
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

    // let certificates_task = || async {
    //     if let Err(err) = check_certificates(pool) {
    //         error!("Failed to check certificates: {err}");
    //     }
    // };

    directory_sync_task().await;
    count_update_task().await;
    updates_check_task().await;
    ldap_sync_task().await;
    expired_acl_rules_task().await;
    check_certificates(pool).await;

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
    }
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
    let updated_rules = query_as!(
        AclRule,
        "UPDATE aclrule SET state = 'expired'::aclrule_state, modified_at = NOW(), \
        modified_by = $1 \
        WHERE state = 'applied'::aclrule_state AND expires < NOW() \
        RETURNING id, parent_id, state \"state: _\", name, allow_all_users, deny_all_users, \
        allow_all_groups, deny_all_groups, allow_all_network_devices, deny_all_network_devices, \
        all_locations, addresses, ports, protocols, enabled, expires, any_address, any_port, \
        any_protocol, use_manual_destination_settings, modified_at, modified_by",
        ACL_EXPIRY_SYSTEM_ACTOR
    )
    .fetch_all(pool)
    .await?;

    // Send firewall config updates to locations which have been affected by updated rules.
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

    let affected_locations = affected_locations.into_iter().collect::<Vec<_>>();
    debug!(
        "{} locations affected by expired ACL rules. Sending gateway firewall update events for \
        each location",
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

fn expiry_check(expiry: NaiveDateTime) {
    const TIME_CHECK: &[TimeDelta] = &[
        TimeDelta::days(14),
        TimeDelta::days(7),
        TimeDelta::days(3),
        TimeDelta::days(1),
    ];

    let now = Utc::now().naive_utc();
    let time_delta = now - expiry;
    for check in TIME_CHECK {
        if check.num_days() == time_delta.num_days() {
            // Send email
        }
    }
}

/// Check if certificates are about to expire, or got expired.
async fn check_certificates(pool: &PgPool) {
    let cert = match Certificates::get(pool).await {
        Ok(Some(cert)) => cert,
        Ok(None) => {
            debug!("No certificates in the databae");
            return;
        }
        Err(err) => {
            error!("Failed to fetch certificates {err}");
            return;
        }
    };

    if let Some(ca_expiry) = cert.ca_expiry {
        expiry_check(ca_expiry);
    }

    if let Some(proxy_http_cert_expiry) = cert.proxy_http_cert_expiry {
        expiry_check(proxy_http_cert_expiry);
    }

    if let Some(core_http_cert_expiry) = cert.core_http_cert_expiry {
        expiry_check(core_http_cert_expiry);
    }
}
