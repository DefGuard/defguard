use std::{net::IpAddr, ops::RangeInclusive};

use defguard_common::db::{
    Id,
    models::{WireguardNetwork, user::User},
};
use ipnetwork::IpNetwork;
use sqlx::PgConnection;
use thiserror::Error;

#[cfg(not(test))]
use crate::enterprise::{LicenseFeature, has_enterprise_access};
use crate::enterprise::{
    firewall::get_location_active_acl_rules,
    utils::{extract_subnets_from_range, get_last_ip_in_v6_subnet, merge_ranges},
};

#[cfg(test)]
mod tests;

#[derive(Debug, Error)]
pub enum AllowedIpsError {
    #[error("ACL is not enabled for this location")]
    AclNotEnabled,
    #[error("Business license is not active")]
    LicenseInactive,
    #[error(transparent)]
    DbError(#[from] sqlx::Error),
}

/// Converts an [`IpNetwork`] to a [`RangeInclusive<IpAddr>`] covering all
/// addresses in the subnet, handling IPv4 and IPv6 correctly.
fn ipnetwork_to_range(network: &IpNetwork) -> RangeInclusive<IpAddr> {
    match network {
        IpNetwork::V4(_) => network.network()..=network.broadcast(),
        IpNetwork::V6(subnet) => IpAddr::from(subnet.network())..=get_last_ip_in_v6_subnet(subnet),
    }
}

/// Computes the ACL-derived `AllowedIPs` for a specific user in a given location.
///
/// Iterates over all active, applied ACL rules assigned to the location, checks
/// whether the given user is permitted by each rule's source policy, and collects
/// the destination addresses from every matching rule. The collected addresses are
/// then merged into the smallest possible non-overlapping set of CIDRs.
///
/// Returns `[0.0.0.0/0, ::/0]` (all traffic) if any matching rule or destination has
/// `any_address = true`.
pub async fn get_allowed_ips_from_acl_rules(
    conn: &mut PgConnection,
    location: &WireguardNetwork<Id>,
    user: &User<Id>,
) -> Result<Vec<IpNetwork>, AllowedIpsError> {
    if !location.acl_enabled {
        return Err(AllowedIpsError::AclNotEnabled);
    }

    #[cfg(not(test))]
    if !has_enterprise_access(Some(LicenseFeature::AclAllowedIps)) {
        debug!(
            "Enterprise license is not active, skipping AllowedIPs computation for location {}",
            location.id
        );
        return Err(AllowedIpsError::LicenseInactive);
    }

    debug!(
        "Computing ACL-derived AllowedIPs for user {} in location {}",
        user.id, location.id
    );

    let acl_rules = get_location_active_acl_rules(location, &mut *conn).await?;

    // Collect all destination networks and ranges across all matching rules.
    let mut all_networks: Vec<IpNetwork> = Vec::new();
    let mut all_ranges: Vec<RangeInclusive<IpAddr>> = Vec::new();

    for rule in acl_rules {
        if !rule.user_is_allowed(user.id, &mut *conn).await? {
            continue;
        }

        debug!(
            "Rule {} matches user {} - collecting destinations",
            rule.id, user.id
        );

        // Collect addresses from manually specified destination settings.
        if rule.use_manual_destination_settings {
            if rule.any_address {
                debug!(
                    "Rule {} has any_address enabled. Skipping manual destinations for user {}",
                    rule.id, user.id
                );
                continue;
            }
            all_networks.extend(rule.addresses.iter().copied());
            all_ranges.extend(rule.address_ranges.iter().map(RangeInclusive::from));

            // Aliases expand into additional addresses and ranges.
            for alias in &rule.aliases {
                if alias.any_address {
                    debug!(
                        "Alias {} in rule {} has any_address enabled. Skipping for user {}",
                        alias.id, rule.id, user.id
                    );
                    continue;
                }
                all_networks.extend(alias.addresses.iter().copied());
                let alias_ranges = alias.get_destination_ranges(&mut *conn).await?;
                all_ranges.extend(alias_ranges.iter().map(RangeInclusive::from));
            }
        }

        // Collect addresses from pre-defined Destinations.
        for destination in &rule.destinations {
            if destination.any_address {
                debug!(
                    "Destination {} in rule {} has any_address enabled. Skipping for user {}",
                    destination.id, rule.id, user.id
                );
                continue;
            }
            all_networks.extend(destination.addresses.iter().copied());
            let dest_ranges = destination.get_destination_ranges(&mut *conn).await?;
            all_ranges.extend(dest_ranges.iter().map(RangeInclusive::from));
        }
    }

    // Convert all networks to ranges and combine with the explicit ranges collected above.
    let combined_ranges: Vec<RangeInclusive<IpAddr>> = all_networks
        .iter()
        .map(ipnetwork_to_range)
        .chain(all_ranges)
        .collect();

    // Merge overlapping/adjacent ranges then decompose into minimal non-overlapping subnets.
    let result: Vec<IpNetwork> = merge_ranges(combined_ranges)
        .into_iter()
        .flat_map(|range| {
            let (start, end) = range.into_inner();
            extract_subnets_from_range(start, end)
        })
        .collect();

    debug!(
        "Computed {} AllowedIPs networks for user {} in location {}",
        result.len(),
        user.id,
        location.id
    );

    Ok(result)
}

/// Computes the effective AllowedIPs for a user in a location, merging manual
/// static IPs with ACL-generated IPs when the `allowed_ips_from_acl` flag is
/// enabled on the location.
///
/// The manual (static) AllowedIPs defined on the network are always included.
/// If `allowed_ips_from_acl` is true, an active enterprise license is present,
/// and ACL is enabled on the location, ACL-derived IPs are computed and merged.
/// Any errors during ACL computation are logged and the manual IPs are returned
/// on their own.
pub async fn get_effective_allowed_ips(
    conn: &mut PgConnection,
    network: &WireguardNetwork<Id>,
    user: &User<Id>,
) -> Vec<IpNetwork> {
    let mut ips = network.allowed_ips.clone();

    if network.allowed_ips_from_acl {
        match get_allowed_ips_from_acl_rules(conn, network, user).await {
            Ok(acl_ips) => ips.extend(acl_ips),
            Err(AllowedIpsError::AclNotEnabled) => {
                debug!(
                    "ACL not enabled for location {}, skipping ACL AllowedIPs",
                    network.id
                );
            }
            Err(AllowedIpsError::LicenseInactive) => {
                debug!(
                    "Enterprise license not active for location {}, skipping ACL AllowedIPs",
                    network.id
                );
            }
            Err(AllowedIpsError::DbError(e)) => {
                warn!(
                    "DB error computing ACL AllowedIPs for location {}: {e}",
                    network.id
                );
            }
        }
    }

    // Deduplicate and sort for deterministic output.
    ips.sort();
    ips.dedup();
    ips
}
