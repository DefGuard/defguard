use std::{collections::HashSet, time::Duration};

use chrono::{NaiveDateTime, TimeDelta, Utc};
use defguard_common::{
    db::models::{
        Certificates, CoreCertSource, ProxyCertSource, User, WireguardNetwork,
        vpn_client_mfa_session::reap_expired,
    },
    types::proxy::ProxyControlMessage,
};
use sqlx::{PgConnection, PgPool, query_as};
use tokio::{
    sync::{broadcast, mpsc},
    time::{Instant, sleep},
};
use tracing::Instrument;

use crate::{
    cert_settings::{refresh_core_self_signed_cert, refresh_proxy_self_signed_cert},
    enterprise::{
        LicenseFeature,
        db::models::acl::AclRule,
        directory_sync::{do_directory_sync, get_directory_sync_interval},
        firewall::try_get_location_firewall_config,
        has_enterprise_access, is_business_license_active,
        ldap::{do_ldap_sync, sync::get_ldap_sync_interval},
        limits::update_counts,
    },
    events::{DirectorySyncEvent, LdapSyncEventType},
    grpc::GatewayCommand,
    letsencrypt::do_letsencrypt_refresh,
    location_management::allowed_peers::get_location_allowed_peers,
    mail::templates,
    updates::do_new_version_check,
};

// Times in seconds
const UTILITY_THREAD_MAIN_SLEEP_TIME: Duration = Duration::from_secs(5);
const COUNT_UPDATE_INTERVAL: u64 = 60 * 60;
const UPDATES_CHECK_INTERVAL: u64 = 60 * 60 * 6;
const EXPIRED_ACL_RULES_CHECK_INTERVAL: u64 = 60 * 5;
const MFA_SESSION_REAP_INTERVAL: u64 = 60 * 5;
const LICENSE_CHECK_INTERVAL: u64 = 60 * 5;
const LETSENCRYPT_EXPIRY_CHECK_INTERVAL: u64 = 60 * 60 * 24;
const CERTIFICATE_EXPIRY_CHECK_INTERVAL: u64 = 60 * 60 * 24; // 1 day
const SELF_SIGNED_REFRESH_THRESHOLD: TimeDelta = TimeDelta::days(14);
const ACL_EXPIRY_SYSTEM_ACTOR: &str = "system:acl-expiry";

#[instrument(skip_all)]
pub async fn run_utility_thread(
    pool: &PgPool,
    gateway_tx: broadcast::Sender<GatewayCommand>,
    proxy_control_tx: mpsc::Sender<ProxyControlMessage>,
    web_reload_tx: broadcast::Sender<()>,
    ldap_tx: mpsc::UnboundedSender<LdapSyncEventType>,
    dirsync_tx: mpsc::UnboundedSender<DirectorySyncEvent>,
) -> Result<(), anyhow::Error> {
    let mut last_count_update = Instant::now();
    let mut last_directory_sync = Instant::now();
    let mut last_updates_check = Instant::now();
    let mut last_ldap_sync = Instant::now();
    let mut last_expired_acl_rules_check = Instant::now();
    let mut last_license_check = Instant::now();
    let mut last_letsencrypt_expiry_check = Instant::now();
    let mut last_certificate_check = Instant::now();
    let mut last_mfa_session_reap = Instant::now();

    // Track the previously observed license gates.
    let mut license_gates = LicenseGates::current();

    let directory_sync_task = || async {
        if let Err(e) = Box::pin(
            do_directory_sync(pool, &gateway_tx, &ldap_tx, &dirsync_tx)
                .instrument(info_span!("directory_sync_task")),
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
        if let Err(e) = do_ldap_sync(pool, &gateway_tx, &ldap_tx)
            .instrument(info_span!("ldap_sync_task"))
            .await
        {
            error!("There was an error while performing LDAP sync job: {e}");
        }
    };

    let expired_acl_rules_task = || async {
        if let Err(err) = expired_acl_rules_check(pool, gateway_tx.clone())
            .instrument(info_span!("expired_acl_rules_task"))
            .await
        {
            error!("Failed to check expired ACL rules: {err}");
        }
    };

    let mfa_session_reap_task = || async {
        if let Err(err) = reap_expired(pool)
            .instrument(info_span!("mfa_session_reap_task"))
            .await
        {
            error!("Failed to reap expired MFA sessions: {err}");
        }
    };

    let letsencrypt_refresh_task = || async {
        if let Err(e) = do_letsencrypt_refresh(pool, proxy_control_tx.clone())
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
    mfa_session_reap_task().await;
    letsencrypt_refresh_task().await;
    check_certificates(pool, &proxy_control_tx, &web_reload_tx).await;

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

        // Reap expired in-progress MFA sessions
        if last_mfa_session_reap.elapsed().as_secs() >= MFA_SESSION_REAP_INTERVAL {
            mfa_session_reap_task().await;
            last_mfa_session_reap = Instant::now();
        }

        // Check LE cert expiry dates and refresh if necessary
        if last_letsencrypt_expiry_check.elapsed().as_secs() >= LETSENCRYPT_EXPIRY_CHECK_INTERVAL {
            letsencrypt_refresh_task().await;
            last_letsencrypt_expiry_check = Instant::now();
        }

        // Reconcile gateway state when any license gate changes.
        if last_license_check.elapsed().as_secs() >= LICENSE_CHECK_INTERVAL {
            let new_gates = LicenseGates::current();
            last_license_check = Instant::now();
            if new_gates != license_gates {
                debug!("License gates changed from {license_gates:?} to {new_gates:?}");
                if let Err(err) = license_status_check(pool, gateway_tx.clone())
                    .instrument(info_span!("license_status_check"))
                    .await
                {
                    error!("Failed to reconcile gateway state after a license change: {err}");
                } else {
                    // update status
                    license_gates = new_gates;
                }
            }
        }

        // Check certificates.
        if last_certificate_check.elapsed().as_secs() >= CERTIFICATE_EXPIRY_CHECK_INTERVAL {
            check_certificates(pool, &proxy_control_tx, &web_reload_tx)
                .instrument(info_span!("check_certificates"))
                .await;
            last_certificate_check = Instant::now();
        }
    }
}

/// License gates that determine gateway state.
#[derive(Debug, PartialEq)]
struct LicenseGates {
    business_tier: bool,
    enterprise_tier: bool,
    service_locations: bool,
}

impl LicenseGates {
    fn current() -> Self {
        Self {
            business_tier: is_business_license_active(),
            enterprise_tier: has_enterprise_access(None),
            service_locations: has_enterprise_access(Some(LicenseFeature::ServiceLocations)),
        }
    }
}

async fn license_status_check(
    pool: &PgPool,
    gateway_tx: broadcast::Sender<GatewayCommand>,
) -> Result<(), anyhow::Error> {
    let mut conn = pool.acquire().await?;
    for location in WireguardNetwork::all(pool).await? {
        // These locations have no license-gated gateway state to reconcile.
        if !location.acl_enabled && !location.mfa_enabled && !location.is_service_location() {
            continue;
        }

        let firewall_config = try_get_location_firewall_config(&location, &mut conn).await?;

        if location.mfa_enabled || location.is_service_location() {
            debug!("Rebuilding gateway peer list for location {location}");
            let peers = get_location_allowed_peers(&location, &mut conn).await?;
            let disable_firewall = location.acl_enabled && firewall_config.is_none();
            let location_id = location.id;
            gateway_tx.send(GatewayCommand::NetworkModified(
                location_id,
                location,
                peers,
                firewall_config,
            ))?;
            // `NetworkModified` cannot clear a firewall config. Send an explicit disable event
            // when an ACL location has no config.
            if disable_firewall {
                gateway_tx.send(GatewayCommand::FirewallDisabled(location_id))?;
            }
        } else if let Some(firewall_config) = firewall_config {
            debug!("Re-enabling gateway firewall configuration for location {location}");
            gateway_tx.send(GatewayCommand::FirewallConfigChanged(
                location.id,
                firewall_config,
            ))?;
        } else {
            debug!("Disabling gateway firewall configuration for location {location}");
            gateway_tx.send(GatewayCommand::FirewallDisabled(location.id))?;
        }
    }

    Ok(())
}

/// Find newly expired ACL rules and update their status.
async fn expired_acl_rules_check(
    pool: &PgPool,
    gateway_tx: broadcast::Sender<GatewayCommand>,
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
                gateway_tx.send(GatewayCommand::FirewallConfigChanged(
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

/// Check if certificate is about to expire, or got expired. Send mail accordingly.
async fn expiry_check(conn: &mut PgConnection, certificate_type: &str, expiry: NaiveDateTime) {
    const TIME_CHECK: &[TimeDelta] = &[
        TimeDelta::days(-14),
        TimeDelta::days(-7),
        TimeDelta::days(-3),
        TimeDelta::days(-1),
        TimeDelta::days(0),
    ];

    let now = Utc::now().naive_utc();
    let time_delta = now - expiry;
    for check in TIME_CHECK {
        if check.num_days() == time_delta.num_days() {
            // Send email to admins.
            if time_delta.num_days() >= 0 {
                debug!("Certificate {certificate_type} has expired; notifying admins");
            } else {
                debug!("Certificate {certificate_type} is about to expire; notifying admins");
            }
            let Ok(admin_users) = User::find_admins(&mut *conn).await else {
                error!("Failed to fetch admins from database");
                return;
            };
            for user in admin_users {
                let _ = if time_delta.num_days() >= 0 {
                    templates::certificate_expired_mail(
                        &user.email,
                        &mut *conn,
                        certificate_type,
                        expiry,
                    )
                    .await
                } else {
                    templates::certificate_expiration_mail(
                        &user.email,
                        &mut *conn,
                        certificate_type,
                        expiry,
                    )
                    .await
                };
            }
        }
    }
}

/// Check if any of the certificates are about to expire, or got expired.
async fn check_certificates(
    pool: &PgPool,
    proxy_control_tx: &mpsc::Sender<ProxyControlMessage>,
    web_reload_tx: &broadcast::Sender<()>,
) {
    let cert = match Certificates::get(pool).await {
        Ok(Some(cert)) => cert,
        Ok(None) => {
            debug!("No certificates in the database");
            return;
        }
        Err(err) => {
            error!("Failed to fetch certificates: {err}");
            return;
        }
    };

    let Ok(mut conn) = pool.begin().await else {
        error!("Failed to create database transaction");
        return;
    };

    // Email notifications for custom uploaded certs
    if let ProxyCertSource::Custom = cert.proxy_http_cert_source
        && let Some(proxy_http_cert_expiry) = cert.proxy_http_cert_expiry
    {
        expiry_check(&mut conn, "Edge HTTPS", proxy_http_cert_expiry).await;
    }

    if let CoreCertSource::Custom = cert.core_http_cert_source
        && let Some(core_http_cert_expiry) = cert.core_http_cert_expiry
    {
        expiry_check(&mut conn, "Core HTTPS", core_http_cert_expiry).await;
    }

    // Auto-refresh self-signed certs when close to expiry
    let now = Utc::now().naive_utc();

    if let CoreCertSource::SelfSigned = cert.core_http_cert_source
        && let Some(expiry) = cert.core_http_cert_expiry
    {
        let expire_in = expiry - now;
        if expire_in <= SELF_SIGNED_REFRESH_THRESHOLD {
            info!(
                "Core self-signed HTTPS certificate expires in {} days, refreshing",
                expire_in.num_days()
            );
            match refresh_core_self_signed_cert(pool).await {
                Ok((_, _, new_expiry)) => {
                    info!("Core self-signed HTTPS certificate refreshed, new expiry: {new_expiry}");
                    if let Err(err) = web_reload_tx.send(()) {
                        error!("Failed to trigger core web server reload: {err}");
                    }
                }
                Err(err) => {
                    error!("Failed to refresh Core self-signed HTTPS certificate: {err}");
                }
            }
        }
    }

    if let ProxyCertSource::SelfSigned = cert.proxy_http_cert_source
        && let Some(expiry) = cert.proxy_http_cert_expiry
    {
        let expire_in = expiry - now;
        if expire_in <= SELF_SIGNED_REFRESH_THRESHOLD {
            info!(
                "Proxy self-signed HTTPS certificate expires in {} days, refreshing",
                expire_in.num_days()
            );
            match refresh_proxy_self_signed_cert(pool).await {
                Ok((cert_pem, key_pem, new_expiry)) => {
                    info!(
                        "Proxy self-signed HTTPS certificate refreshed, new expiry: {new_expiry}"
                    );
                    if let Err(err) = proxy_control_tx
                        .send(ProxyControlMessage::BroadcastHttpsCerts { cert_pem, key_pem })
                        .await
                    {
                        error!("Failed to broadcast refreshed proxy HTTPS cert to proxies: {err}");
                    }
                }
                Err(err) => {
                    error!("Failed to refresh Proxy self-signed HTTPS certificate: {err}");
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{net::IpAddr, str::FromStr};

    use defguard_common::{
        db::{
            Id,
            models::{
                Device, DeviceType, MfaFlow,
                device::WireguardNetworkDevice,
                group::Group,
                mfa_flow::LocationMfaFlowAssignment,
                vpn_client_session::{VpnClientMfaMethod, VpnClientSession},
            },
            setup_pool,
        },
        gateway_types::WireguardPeer,
    };
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

    use super::*;
    use crate::enterprise::license::{License, LicenseTier, SupportType, set_cached_license};

    /// Install a license for `tier` with no limits or expiry in the global cache.
    fn set_test_license(tier: LicenseTier) {
        set_cached_license(Some(License::new(
            "test-customer-id".to_owned(),
            false,
            None,
            None,
            None,
            tier,
            SupportType::Basic,
            Vec::new(),
        )));
    }

    /// Create an MFA-enabled location with one configured device and session PSK.
    async fn setup_mfa_location(conn: &mut PgConnection) -> WireguardNetwork<Id> {
        let user = User::new(
            "testuser",
            Some("password123"),
            "Test",
            "User",
            "test@example.com",
            None,
        )
        .save(&mut *conn)
        .await
        .unwrap();

        let device = Device::new(
            "device1".into(),
            "pubkey1".into(),
            user.id,
            DeviceType::User,
            None,
            true,
        )
        .save(&mut *conn)
        .await
        .unwrap();

        let mut location = WireguardNetwork::default()
            .try_set_address("10.7.1.1/24")
            .unwrap();
        location.name = "mfa-location".to_owned();
        location.mfa_enabled = true;
        let location = location.save(&mut *conn).await.unwrap();

        WireguardNetworkDevice::new(
            location.id,
            device.id,
            vec![IpAddr::from_str("10.7.1.2").unwrap()],
        )
        .insert(&mut *conn)
        .await
        .unwrap();

        let mut session = VpnClientSession::new(location.id, user.id, device.id, None, true);
        session.preshared_key = Some("test-psk".into());
        session.save(&mut *conn).await.unwrap();

        location
    }

    /// Reconcile gateway state and return the peer list for `location_id`.
    async fn reconcile_peers(pool: &PgPool, location_id: Id) -> Vec<WireguardPeer> {
        let (gateway_tx, mut gateway_rx) = broadcast::channel(16);
        license_status_check(pool, gateway_tx).await.unwrap();

        while let Ok(command) = gateway_rx.try_recv() {
            if let GatewayCommand::NetworkModified(id, _, peers, _) = command
                && id == location_id
            {
                return peers;
            }
        }
        panic!("No NetworkModified command was sent for location {location_id}");
    }

    /// A Business-tier MFA policy loses its peers when downgraded to Free and regains them after
    /// an upgrade.
    #[sqlx::test]
    async fn test_license_status_check_rebuilds_peers_on_business_to_free_downgrade(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let mut conn = pool.acquire().await.unwrap();
        let location = setup_mfa_location(&mut conn).await;

        // Two MFA steps require Business-tier access.
        let (flow, _) = MfaFlow::create(
            &mut conn,
            "FlowTitle".to_owned(),
            vec![
                vec![VpnClientMfaMethod::Totp],
                vec![VpnClientMfaMethod::Email],
            ],
        )
        .await
        .unwrap();
        MfaFlow::assign_to_location(
            &mut conn,
            location.id,
            &[LocationMfaFlowAssignment {
                flow_id: flow.id,
                is_default: true,
                group_ids: Vec::new(),
            }],
        )
        .await
        .unwrap();
        drop(conn);

        set_test_license(LicenseTier::Business);
        let peers = reconcile_peers(&pool, location.id).await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].pubkey, "pubkey1");

        set_cached_license(None);
        assert!(reconcile_peers(&pool, location.id).await.is_empty());

        set_test_license(LicenseTier::Business);
        assert_eq!(reconcile_peers(&pool, location.id).await.len(), 1);
    }

    /// An Enterprise-tier MFA policy loses its peers when downgraded to Business.
    #[sqlx::test]
    async fn test_license_status_check_rebuilds_peers_on_enterprise_to_business_downgrade(
        _: PgPoolOptions,
        options: PgConnectOptions,
    ) {
        let pool = setup_pool(options).await;
        let mut conn = pool.acquire().await.unwrap();
        let location = setup_mfa_location(&mut conn).await;

        let group = Group::new("TestGroup").save(&mut *conn).await.unwrap();
        let (default_flow, _) = MfaFlow::create(
            &mut conn,
            "DefaultFlow".to_owned(),
            vec![vec![VpnClientMfaMethod::Totp]],
        )
        .await
        .unwrap();
        let (group_flow, _) = MfaFlow::create(
            &mut conn,
            "GroupFlow".to_owned(),
            vec![vec![VpnClientMfaMethod::Email]],
        )
        .await
        .unwrap();

        // A group-scoped assignment requires Enterprise-tier access.
        MfaFlow::assign_to_location(
            &mut conn,
            location.id,
            &[
                LocationMfaFlowAssignment {
                    flow_id: default_flow.id,
                    is_default: true,
                    group_ids: Vec::new(),
                },
                LocationMfaFlowAssignment {
                    flow_id: group_flow.id,
                    is_default: false,
                    group_ids: vec![group.id],
                },
            ],
        )
        .await
        .unwrap();
        drop(conn);

        set_test_license(LicenseTier::Enterprise);
        let peers = reconcile_peers(&pool, location.id).await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].pubkey, "pubkey1");

        set_test_license(LicenseTier::Business);
        assert!(reconcile_peers(&pool, location.id).await.is_empty());
    }
}
