mod common;
mod gateway_manager;

use chrono::Utc;
use defguard_common::db::{Id, models::gateway::Gateway};

use super::needs_restart;

fn base_gateway() -> Gateway<Id> {
    Gateway {
        id: 1,
        location_id: 1,
        name: "test".to_owned(),
        address: "127.0.0.1".to_owned(),
        port: 50051,
        connected_at: None,
        disconnected_at: None,
        certificate_serial: None,
        certificate_expiry: None,
        version: None,
        enabled: true,
        modified_at: Utc::now().naive_utc(),
        modified_by: "test".to_owned(),
        core_client_cert_der: None,
        core_client_cert_key_der: None,
        core_client_cert_expiry: None,
    }
}

#[test]
fn test_needs_restart_detects_connection_relevant_field_changes() {
    let base = base_gateway();

    // Identical gateways - no restart needed.
    assert!(!needs_restart(&base, &base.clone()));

    // Non-connection-relevant fields - no restart.
    let mut no_restart = base.clone();
    no_restart.version = Some("2.0.0".to_owned());
    no_restart.modified_by = "someone-else".to_owned();
    no_restart.connected_at = Some(Utc::now().naive_utc());
    no_restart.disconnected_at = Some(Utc::now().naive_utc());
    no_restart.certificate_serial = Some("abc".to_owned());
    no_restart.core_client_cert_expiry = Some(Utc::now().naive_utc());
    assert!(!needs_restart(&base, &no_restart));

    // address change - restart required.
    let mut changed = base.clone();
    changed.address = "10.0.0.1".to_owned();
    assert!(needs_restart(&base, &changed));

    // port change - restart required.
    let mut changed = base.clone();
    changed.port = 9999;
    assert!(needs_restart(&base, &changed));

    // enabled change - restart required.
    let mut changed = base.clone();
    changed.enabled = false;
    assert!(needs_restart(&base, &changed));

    // core_client_cert_der change - restart required.
    let mut changed = base.clone();
    changed.core_client_cert_der = Some(vec![1, 2, 3]);
    assert!(needs_restart(&base, &changed));

    // core_client_cert_key_der change - restart required.
    let mut changed = base.clone();
    changed.core_client_cert_key_der = Some(vec![4, 5, 6]);
    assert!(needs_restart(&base, &changed));
}
