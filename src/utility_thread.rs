use std::{collections::HashSet, time::Duration};

use sqlx::{query_as, PgPool};
use tokio::{
    sync::broadcast::Sender,
    time::{sleep, Instant},
};

use crate::{
    db::{GatewayEvent, Id, WireguardNetwork},
    enterprise::{
        db::models::acl::{AclRule, RuleState},
        directory_sync::{do_directory_sync, get_directory_sync_interval},
        is_enterprise_enabled,
        ldap::{do_ldap_sync, sync::get_ldap_sync_interval},
        limits::do_count_update,
    },
    updates::do_new_version_check,
};

// Times in seconds
const UTILITY_THREAD_MAIN_SLEEP_TIME: u64 = 5;
const COUNT_UPDATE_INTERVAL: u64 = 60 * 60;
const UPDATES_CHECK_INTERVAL: u64 = 60 * 60 * 6;
const EXPIRED_ACL_RULES_CHECK_INTERVAL: u64 = 60 * 5;
const ENTERPRISE_STATUS_CHECK_INTERVAL: u64 = 60 * 5;

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
    let mut enterprise_enabled = is_enterprise_enabled();

    let directory_sync_task = || async {
        if let Err(e) = do_directory_sync(pool, &wireguard_tx).await {
            error!("There was an error while performing directory sync job: {e:?}",);
        }
    };

    let count_update_task = || async {
        if let Err(e) = do_count_update(pool).await {
            error!("There was an error while performing count update job: {e:?}");
        }
    };

    let updates_check_task = || async {
        if let Err(e) = do_new_version_check().await {
            error!("There was an error while checking for new Defguard version: {e:?}");
        }
    };

    let ldap_sync_task = || async {
        if let Err(e) = do_ldap_sync(pool).await {
            error!("There was an error while performing LDAP sync job: {e}");
        }
    };

    directory_sync_task().await;
    count_update_task().await;
    updates_check_task().await;
    ldap_sync_task().await;

    loop {
        sleep(Duration::from_secs(UTILITY_THREAD_MAIN_SLEEP_TIME)).await;

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
            if let Err(err) = expired_acl_rules_check(pool, wireguard_tx.clone()).await {
                error!("Failed to check expired ACL rules: {err}");
            };
            last_expired_acl_rules_check = Instant::now();
        }

        // Check if enterprise features got enabled or disabled
        if last_enterprise_status_check.elapsed().as_secs() >= ENTERPRISE_STATUS_CHECK_INTERVAL {
            let new_enterprise_enabled = is_enterprise_enabled();
            if let Err(err) = enterprise_status_check(
                pool,
                wireguard_tx.clone(),
                enterprise_enabled,
                new_enterprise_enabled,
            )
            .await
            {
                error!("Failed to check enterprise status: {err}");
            } else {
                // update status
                enterprise_enabled = new_enterprise_enabled;
            };
            last_enterprise_status_check = Instant::now();
        }
    }
}

/// Check if enterprise status has changed and perform any necessary actions
async fn enterprise_status_check(
    pool: &PgPool,
    wireguard_tx: Sender<GatewayEvent>,
    current_enterprise_enabled: bool,
    new_enterprise_enabled: bool,
) -> Result<(), anyhow::Error> {
    if new_enterprise_enabled != current_enterprise_enabled {
        debug!("Enterprise feature status changed from {current_enterprise_enabled} to {new_enterprise_enabled}");

        // fetch all ACL-enabled networks
        let locations: Vec<WireguardNetwork<Id>> = WireguardNetwork::all(pool)
            .await?
            .into_iter()
            .filter(|location| location.acl_enabled)
            .collect();

        if new_enterprise_enabled {
            // handle switch from disabled -> enabled
            debug!("Re-enabling gateway firewall configuration for ACL-enabled locations");
            let mut conn = pool.acquire().await?;
            for location in locations {
                debug!("Re-enabling gateway firewall configuration for location {location:?}");
                let firewall_config = location
                    .try_get_firewall_config(&mut conn)
                    .await?
                    .expect("ACL-enabled location must have firewall config");

                wireguard_tx.send(GatewayEvent::FirewallConfigChanged(
                    location.id,
                    firewall_config,
                ))?;
            }
        } else {
            // handle switch from enabled -> disabled
            debug!("Disabling gateway firewall configuration for ACL-enabled locations");
            for location in locations {
                debug!("Disabling gateway firewall configuration for location {location:?}");
                wireguard_tx.send(GatewayEvent::FirewallDisabled(location.id))?;
            }
        }
    };
    Ok(())
}

/// Find newly expired ACL rules and update their status.
async fn expired_acl_rules_check(
    pool: &PgPool,
    wireguard_tx: Sender<GatewayEvent>,
) -> Result<(), anyhow::Error> {
    // mark relevant rules as expired
    let updated_rules = query_as!(
            AclRule::<Id>,
            "UPDATE aclrule SET state = 'expired'::aclrule_state \
            WHERE state = 'applied'::aclrule_state AND expires < NOW() \
            RETURNING id, parent_id, state AS \"state: RuleState\", name, allow_all_users, deny_all_users, \
                allow_all_network_devices, deny_all_network_devices, all_networks, \
                destination, ports, protocols, enabled, expires"
        )
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
            "{} locations affected by expired ACL rules. Sending gateway firewall update events for each location",
            affected_locations.len()
        );

    let mut conn = pool.acquire().await?;
    for location in affected_locations {
        match location.try_get_firewall_config(&mut conn).await? {
            Some(firewall_config) => {
                debug!("Sending firewall update event for location {location}");
                wireguard_tx.send(GatewayEvent::FirewallConfigChanged(
                    location.id,
                    firewall_config,
                ))?;
            }
            None => {
                debug!("No firewall config generated for location {location}. Not sending a gateway event")
            }
        }
    }

    Ok(())
}
