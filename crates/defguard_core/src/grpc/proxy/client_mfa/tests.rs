use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::{
        Arc, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime},
};

use chrono::Utc;
use defguard_common::db::{
    Id,
    models::{
        Device, DeviceType, User, WireguardNetwork,
        device::WireguardNetworkDevice,
        mfa_flow::{LocationMfaFlowAssignment, MfaFlow},
        polling_token::PollingToken,
        settings::initialize_current_settings,
        user::{TOTP_CODE_DIGITS, TOTP_CODE_VALIDITY_PERIOD},
        vpn_client_mfa_session::{
            MFA_FAILED_ATTEMPT_CAP, MfaAttribution, VPN_MFA_SESSION_TIMEOUT, VpnClientMfaSession,
        },
        vpn_client_session::{VpnClientMfaMethod, VpnClientSession, VpnClientSessionState},
        wireguard::ServiceLocationMode,
    },
    setup_pool,
};
use defguard_proto::{
    client_types::{
        ClientMfaFinishRequest, ClientMfaStartRequest, ClientMfaStepStartRequest, MfaMethod,
    },
    enterprise::posture::{BoolCheck, DevicePostureCheckRequest, DevicePostureData, bool_check},
    proxy::{ClientMfaOidcAuthenticateRequest, ClientMfaTokenValidationRequest, DeviceInfo},
};
use ipnetwork::IpNetwork;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tonic::Code;
use totp_lite::{Sha1, totp_custom};

use super::{ClientMfaServer, ClientMfaStartOutcome};
use crate::{
    enterprise::{
        db::models::device_posture::{
            DevicePosture, DevicePostureLocation, DevicePostureOsRule, OsType,
        },
        handlers::openid_login::build_state,
        license::{License, LicenseTier, SupportType, set_cached_license},
        limits::{Counts, set_counts},
    },
    events::{BidiStreamEvent, BidiStreamEventType, DesktopClientMfaEvent},
    grpc::{GatewayCommand, proto::enterprise::license::LicenseLimits},
    mfa_engine::authorize::{EventChannels, create_new_session},
};

const REPLACEMENT_MFA_PRESHARED_KEY: &str = "replacement-mfa-psk";
const NEW_MFA_PRESHARED_KEY: &str = "new-psk";
const DEVICE_INFO_IP: &str = "10.0.0.7";

/// The `DeviceInfo` the proxy attaches to every bidi request; audit events are built from it.
fn device_info() -> Option<DeviceInfo> {
    Some(DeviceInfo {
        ip_address: DEVICE_INFO_IP.to_owned(),
        user_agent: Some("defguard-client/1.6.0".to_owned()),
        ..Default::default()
    })
}

#[sqlx::test]
async fn test_posture_check_success_emits_vpn_session_authorized_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, _event_rx, mut gateway_rx) = make_server(pool.clone());

    let outcome = server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                device_posture_data: Some(passing_linux_posture_data()),
                token: Some(token.clone()),
            },
            device_info(),
        )
        .await
        .expect("posture check should pass");
    let preshared_key = match outcome {
        super::PostureCheckOutcome::Approved { preshared_key } => preshared_key,
        super::PostureCheckOutcome::Rejected { failed_checks } => {
            panic!("posture check unexpectedly failed: {failed_checks:?}")
        }
    };

    match gateway_rx
        .try_recv()
        .expect("expected VPN authorization gateway event")
    {
        GatewayCommand::VpnSessionAuthorized(location_id, authorized_device, network_info) => {
            assert_eq!(location_id, location.id);
            assert_eq!(authorized_device.id, device.id);
            assert_eq!(network_info.network_id, location.id);
            assert_eq!(
                network_info.preshared_key.as_deref(),
                Some(preshared_key.as_str())
            );
            assert!(network_info.is_authorized);
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }

    let active_sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to fetch active sessions");
    assert_eq!(active_sessions.len(), 1);
    assert_eq!(
        active_sessions[0].preshared_key.as_deref(),
        Some(preshared_key.as_str())
    );
}

#[sqlx::test]
async fn test_replacing_posture_session_emits_vpn_session_deauthorized_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let mut old_session = VpnClientSession::new(
        location.id,
        user.id,
        device.id,
        Some(Utc::now().naive_utc()),
        false,
    );
    old_session.preshared_key = Some("old-posture-psk".to_owned());
    old_session.state = VpnClientSessionState::Connected;
    let old_session = old_session
        .save(&pool)
        .await
        .expect("failed to create previous posture session");
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, mut event_rx, mut gateway_rx) = make_server(pool.clone());

    server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                device_posture_data: Some(passing_linux_posture_data()),
                token: Some(token.clone()),
            },
            device_info(),
        )
        .await
        .expect("replacement posture check should pass");

    match gateway_rx
        .try_recv()
        .expect("expected VPN deauthorization gateway event for replaced posture session")
    {
        GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
            assert_eq!(location_id, location.id);
            assert_eq!(disconnected_device.id, device.id);
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }
    match gateway_rx
        .try_recv()
        .expect("expected VPN authorization gateway event for replacement posture session")
    {
        GatewayCommand::VpnSessionAuthorized(location_id, authorized_device, network_info) => {
            assert_eq!(location_id, location.id);
            assert_eq!(authorized_device.id, device.id);
            assert!(network_info.preshared_key.is_some());
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }

    // the passing posture evaluation is audited first
    let event = event_rx
        .try_recv()
        .expect("expected posture check passed audit event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::PostureCheckPassed { .. } => {}
            other => panic!("unexpected bidi event: {other:?}"),
        },
        other => panic!("unexpected bidi stream event type: {other:?}"),
    }

    // replacing a connected posture-only session emits the unified session
    // superseded audit event, flagged as a non-MFA session
    let event = event_rx
        .try_recv()
        .expect("expected session replaced audit event for replaced posture session");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::SessionSuperseded {
                location: event_location,
                device: event_device,
                is_mfa_session,
            } => {
                assert_eq!(event_location.id, location.id);
                assert_eq!(event_device.id, device.id);
                assert!(!is_mfa_session);
            }
            other => panic!("unexpected bidi event: {other:?}"),
        },
        other => panic!("unexpected bidi stream event type: {other:?}"),
    }

    let old_session = VpnClientSession::find_by_id(&pool, old_session.id)
        .await
        .expect("failed to reload old posture session")
        .expect("expected old posture session");
    assert_eq!(old_session.state, VpnClientSessionState::Disconnected);
}

/// A caller with no token must be refused. Without this, knowing a device's public key is
/// enough to mint a preshared key for it.
#[sqlx::test]
async fn test_posture_check_requires_a_token(_: PgPoolOptions, options: PgConnectOptions) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let (mut server, _, mut gateway_rx) = make_server(pool.clone());

    for token in [None, Some(String::new())] {
        let err = server
            .handle_posture_check(
                DevicePostureCheckRequest {
                    location_id: location.id,
                    pubkey: device.wireguard_pubkey.clone(),
                    device_posture_data: Some(passing_linux_posture_data()),
                    token,
                },
                device_info(),
            )
            .await;
        let err = match err {
            Ok(_) => panic!("posture check without a token must be refused"),
            Err(err) => err,
        };
        assert_eq!(err.code(), Code::Unauthenticated);
    }

    // No session may be created and the gateway must not be touched.
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query sessions")
            .is_empty()
    );
    assert!(gateway_rx.try_recv().is_err());
}

/// An unknown token must be refused, so tokens cannot be guessed or replayed after rotation.
#[sqlx::test]
async fn test_posture_check_rejects_unknown_token(_: PgPoolOptions, options: PgConnectOptions) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let (mut server, _, _) = make_server(pool);

    let err = server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                device_posture_data: Some(passing_linux_posture_data()),
                token: Some("not-a-real-token".to_owned()),
            },
            device_info(),
        )
        .await;
    let err = match err {
        Ok(_) => panic!("posture check with an unknown token must be refused"),
        Err(err) => err,
    };

    assert_eq!(err.code(), Code::Unauthenticated);
}

/// Regression test for the session-hijack denial of service: holding a valid token for *one*
/// device must not allow authorizing — and thereby superseding the live session of — another.
#[sqlx::test]
async fn test_posture_check_rejects_token_belonging_to_another_device(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;

    let victim = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, victim.id).await;

    // The attacker is a legitimately enrolled device with a token of its own.
    let attacker = Device::new(
        "attacker-device".to_owned(),
        "attacker-pubkey".to_owned(),
        user.id,
        DeviceType::User,
        None,
        true,
    )
    .save(&pool)
    .await
    .expect("failed to create attacker device");
    let attacker_token = create_polling_token(&pool, attacker.id).await;

    // The victim holds a live session.
    let mut victim_session = VpnClientSession::new(
        location.id,
        user.id,
        victim.id,
        Some(Utc::now().naive_utc()),
        false,
    );
    victim_session.preshared_key = Some("victim-psk".to_owned());
    victim_session.state = VpnClientSessionState::Connected;
    let victim_session = victim_session
        .save(&pool)
        .await
        .expect("failed to create victim session");

    let (mut server, _, mut gateway_rx) = make_server(pool.clone());

    // Attacker presents its own valid token but claims the victim's public key.
    let err = server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: victim.wireguard_pubkey.clone(),
                device_posture_data: Some(passing_linux_posture_data()),
                token: Some(attacker_token),
            },
            device_info(),
        )
        .await;
    let err = match err {
        Ok(_) => panic!("a token from another device must not authorize this one"),
        Err(err) => err,
    };
    assert_eq!(err.code(), Code::Unauthenticated);

    // The victim's session must survive untouched, and the gateway must see nothing.
    let victim_session = VpnClientSession::find_by_id(&pool, victim_session.id)
        .await
        .expect("failed to reload victim session")
        .expect("victim session should still exist");
    assert_eq!(victim_session.state, VpnClientSessionState::Connected);
    assert_eq!(
        victim_session.preshared_key.as_deref(),
        Some("victim-psk"),
        "the victim's preshared key must not have been rotated"
    );
    assert!(
        gateway_rx.try_recv().is_err(),
        "no peer delete or re-create may be sent to the gateway"
    );
}

#[sqlx::test]
async fn test_posture_check_rejects_mfa_enabled_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_mfa_location(&pool).await;
    // A valid token is needed to get past authentication and reach the check under test.
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, _, _) = make_server(pool);

    let err = match server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: "irrelevant".to_owned(),
                device_posture_data: None,
                token: Some(token),
            },
            device_info(),
        )
        .await
    {
        Ok(_) => panic!("MFA-enabled location should reject posture-only flow"),
        Err(err) => err,
    };

    assert_eq!(err.code(), Code::InvalidArgument);
}

/// A location with no postures assigned hands its peers to the gateway without a preshared
/// key, so the only answer that lets a client connect is an empty one. Approving instead of
/// erroring is what allows a service location whose cached config still demands a posture check
/// to recover after an admin unassigns the last posture.
#[sqlx::test]
async fn test_posture_check_without_postures_approves_with_empty_preshared_key(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, _event_rx, mut gateway_rx) = make_server(pool.clone());

    let outcome = server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                device_posture_data: None,
                token: Some(token),
            },
            device_info(),
        )
        .await
        .expect("location without postures should be approved");

    match outcome {
        super::PostureCheckOutcome::Approved { preshared_key } => assert!(
            preshared_key.is_empty(),
            "a location without postures must not hand out a preshared key"
        ),
        super::PostureCheckOutcome::Rejected { failed_checks } => {
            panic!("posture check unexpectedly failed: {failed_checks:?}")
        }
    }

    // No session may be created and the gateway must not be touched.
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query sessions")
            .is_empty(),
        "no VPN session may be created when a location has no postures"
    );
    assert!(
        gateway_rx.try_recv().is_err(),
        "no gateway command may be sent when a location has no postures"
    );
}

#[sqlx::test]
async fn test_posture_check_without_postures_rejects_device_not_assigned_to_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, mut event_rx, mut gateway_rx) = make_server(pool);

    let status = match server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey,
                device_posture_data: None,
                token: Some(token),
            },
            device_info(),
        )
        .await
    {
        Ok(_) => panic!("a device not assigned to the location must not be approved"),
        Err(status) => status,
    };

    assert_eq!(status.code(), Code::PermissionDenied);
    assert_eq!(status.message(), "device is not assigned to location");
    assert!(event_rx.try_recv().is_err());
    assert!(gateway_rx.try_recv().is_err());
}

/// The empty-preshared-key approval must not outrank the access checks: deactivating a user has
/// to stop their devices from getting anything that reads as approval, even on a location with
/// no postures where the approval grants nothing by itself.
#[sqlx::test]
async fn test_posture_check_without_postures_still_rejects_inactive_user(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    let mut user = create_user(&pool).await;
    user.is_active = false;
    user.save(&pool).await.expect("failed to deactivate user");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, _event_rx, _gateway_rx) = make_server(pool.clone());

    let status = match server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                device_posture_data: None,
                token: Some(token),
            },
            device_info(),
        )
        .await
    {
        Ok(super::PostureCheckOutcome::Approved { .. }) => {
            panic!("an inactive user must not be approved, even without postures")
        }
        Ok(super::PostureCheckOutcome::Rejected { .. }) => {
            panic!("expected an inactive-user error, not a posture rejection")
        }
        Err(status) => status,
    };
    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert_eq!(status.message(), "user is inactive");
}

/// A passing posture evaluation must be auditable, so an operator can see that a headless
/// service location connected and why.
#[sqlx::test]
async fn test_posture_check_pass_emits_posture_check_passed_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, mut event_rx, _gateway_rx) = make_server(pool.clone());

    let posture_data = passing_linux_posture_data();
    match server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                device_posture_data: Some(posture_data.clone()),
                token: Some(token),
            },
            device_info(),
        )
        .await
        .expect("posture check should pass")
    {
        super::PostureCheckOutcome::Approved { preshared_key } => {
            assert!(!preshared_key.is_empty());
        }
        super::PostureCheckOutcome::Rejected { failed_checks } => {
            panic!("posture check unexpectedly failed: {failed_checks:?}")
        }
    }

    let event = event_rx
        .try_recv()
        .expect("expected posture check passed audit event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::PostureCheckPassed {
                device: event_device,
                location: event_location,
                device_posture_data,
            } => {
                assert_eq!(event_device.id, device.id);
                assert_eq!(event_location.id, location.id);
                assert_eq!(device_posture_data, Some(posture_data));
            }
            other => panic!("unexpected bidi event: {other:?}"),
        },
        other => panic!("unexpected bidi stream event type: {other:?}"),
    }
    assert_eq!(event.context.user_id, user.id);
    assert_eq!(event.context.username, user.username);
    assert_eq!(event.context.ip, Some(DEVICE_INFO_IP.parse().unwrap()));
}

/// A failing posture evaluation must be auditable and revoke an existing posture session.
#[sqlx::test]
async fn test_posture_check_failure_revokes_active_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let mut active_session = VpnClientSession::new(
        location.id,
        user.id,
        device.id,
        Some(Utc::now().naive_utc()),
        false,
    );
    active_session.preshared_key = Some("active-posture-psk".to_owned());
    active_session.state = VpnClientSessionState::Connected;
    let active_session = active_session
        .save(&pool)
        .await
        .expect("failed to create active posture session");
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, mut event_rx, mut gateway_rx) = make_server(pool.clone());

    // the policy requires disk encryption
    let posture_data = DevicePostureData {
        disk_encryption: Some(BoolCheck {
            result: Some(bool_check::Result::Value(false)),
        }),
        ..passing_linux_posture_data()
    };
    let rejected_checks = match server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                device_posture_data: Some(posture_data.clone()),
                token: Some(token),
            },
            device_info(),
        )
        .await
        .expect("posture check should complete")
    {
        super::PostureCheckOutcome::Approved { .. } => {
            panic!("posture check with unencrypted disk should be rejected")
        }
        super::PostureCheckOutcome::Rejected { failed_checks } => failed_checks,
    };
    assert!(!rejected_checks.is_empty());

    let event = event_rx
        .try_recv()
        .expect("expected posture check failed audit event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::PostureCheckFailed {
                device: event_device,
                location: event_location,
                device_posture_data,
                failed_checks,
            } => {
                assert_eq!(event_device.id, device.id);
                assert_eq!(event_location.id, location.id);
                assert_eq!(device_posture_data, Some(posture_data));
                assert_eq!(failed_checks, rejected_checks);
            }
            other => panic!("unexpected bidi event: {other:?}"),
        },
        other => panic!("unexpected bidi stream event type: {other:?}"),
    }
    assert_eq!(event.context.user_id, user.id);
    assert_eq!(event.context.username, user.username);

    match gateway_rx
        .try_recv()
        .expect("expected rejected posture session to be deauthorized")
    {
        GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
            assert_eq!(location_id, location.id);
            assert_eq!(disconnected_device.id, device.id);
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }
    assert!(gateway_rx.try_recv().is_err());

    let event = event_rx
        .try_recv()
        .expect("expected session disconnected audit event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Disconnected {
                location: event_location,
                device: event_device,
                is_mfa_session,
            } => {
                assert_eq!(event_location.id, location.id);
                assert_eq!(event_device.id, device.id);
                assert!(!is_mfa_session);
            }
            other => panic!("unexpected bidi event: {other:?}"),
        },
        other => panic!("unexpected bidi stream event type: {other:?}"),
    }
    assert_eq!(event.context.ip, Some(DEVICE_INFO_IP.parse().unwrap()));

    let active_session = VpnClientSession::find_by_id(&pool, active_session.id)
        .await
        .expect("failed to reload active posture session")
        .expect("expected active posture session");
    assert_eq!(active_session.state, VpnClientSessionState::Disconnected);
    assert!(active_session.disconnected_at.is_some());
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to query sessions")
            .is_empty()
    );
}

#[sqlx::test]
async fn test_mfa_start_posture_failure_revokes_active_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let mut active_session = VpnClientSession::new(
        location.id,
        user.id,
        device.id,
        Some(Utc::now().naive_utc()),
        true,
    );
    active_session.preshared_key = Some("active-mfa-psk".to_owned());
    let active_session = active_session
        .save(&pool)
        .await
        .expect("failed to create active MFA session");
    let (mut server, mut event_rx, mut gateway_rx) = make_server(pool.clone());
    let posture_data = DevicePostureData {
        disk_encryption: Some(BoolCheck {
            result: Some(bool_check::Result::Value(false)),
        }),
        ..passing_linux_posture_data()
    };

    let outcome = server
        .start_client_mfa_login(
            ClientMfaStartRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Email as i32,
                posture_data: Some(posture_data),
                selected_methods: Vec::new(),
            },
            device_info(),
        )
        .await
        .expect("posture check should complete");
    assert!(matches!(
        outcome,
        super::ClientMfaStartOutcome::Rejected { .. }
    ));

    match gateway_rx
        .try_recv()
        .expect("expected rejected MFA session to be deauthorized")
    {
        GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
            assert_eq!(location_id, location.id);
            assert_eq!(disconnected_device.id, device.id);
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }

    event_rx
        .try_recv()
        .expect("expected posture check failed audit event");
    let event = event_rx
        .try_recv()
        .expect("expected session disconnected audit event");
    assert_eq!(event.context.ip, Some(DEVICE_INFO_IP.parse().unwrap()));

    let active_session = VpnClientSession::find_by_id(&pool, active_session.id)
        .await
        .expect("failed to reload active MFA session")
        .expect("expected active MFA session");
    assert_eq!(active_session.state, VpnClientSessionState::Disconnected);
    assert!(active_session.disconnected_at.is_some());
}

#[sqlx::test]
async fn test_mfa_start_rejects_non_derivable_location_with_update_message(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;

    // Assign a two-step flow so `derive_legacy_mode` returns `None`.
    let mut tx = pool.begin().await.expect("failed to begin tx");
    let (flow, _) = MfaFlow::create(
        &mut tx,
        "multi-step".to_owned(),
        vec![
            vec![VpnClientMfaMethod::Totp],
            vec![VpnClientMfaMethod::Email],
        ],
    )
    .await
    .expect("failed to create flow");
    MfaFlow::assign_to_location(
        &mut tx,
        location.id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .expect("failed to assign flow");
    tx.commit().await.expect("failed to commit tx");

    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (mut server, _, _) = make_server(pool);

    let error = server
        .start_client_mfa_login(
            ClientMfaStartRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Totp as i32,
                posture_data: None,
                selected_methods: Vec::new(),
            },
            device_info(),
        )
        .await
        .err()
        .expect("non-derivable location should be rejected");

    assert_eq!(error.code(), Code::FailedPrecondition);
    assert_eq!(
        error.message(),
        "Defguard client version is too old to connect to this location. Please update your client."
    );
    assert!(
        !error.message().contains("no valid license"),
        "rejection message must not contain 'no valid license'"
    );
}

#[sqlx::test]
async fn test_session_revocation_survives_unavailable_side_effect_consumers(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_non_mfa_location(&pool).await;
    save_linux_posture_policy(&pool, location.id).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let session = VpnClientSession::new(
        location.id,
        user.id,
        device.id,
        Some(Utc::now().naive_utc()),
        false,
    )
    .save(&pool)
    .await
    .expect("failed to create active posture session");
    let token = create_polling_token(&pool, device.id).await;
    let (mut server, event_rx, gateway_rx) = make_server(pool.clone());
    drop(event_rx);
    drop(gateway_rx);
    let posture_data = DevicePostureData {
        disk_encryption: Some(BoolCheck {
            result: Some(bool_check::Result::Value(false)),
        }),
        ..passing_linux_posture_data()
    };

    let outcome = server
        .handle_posture_check(
            DevicePostureCheckRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                device_posture_data: Some(posture_data),
                token: Some(token),
            },
            device_info(),
        )
        .await
        .expect("side-effect delivery must not prevent posture rejection");
    assert!(matches!(
        outcome,
        super::PostureCheckOutcome::Rejected { .. }
    ));

    let session = VpnClientSession::find_by_id(&pool, session.id)
        .await
        .expect("failed to reload session")
        .expect("expected session");
    assert_eq!(session.state, VpnClientSessionState::Disconnected);
}

#[sqlx::test]
async fn test_replacing_connected_mfa_session_emits_session_superseded_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_mfa_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let old_session = VpnClientSession::new(
        location.id,
        user.id,
        device.id,
        Some(Utc::now().naive_utc()),
        true,
    )
    .save(&pool)
    .await
    .expect("failed to create existing MFA session");

    let (gateway_tx, mut gateway_rx) = broadcast::channel(8);
    let (bidi_event_tx, mut event_rx) = mpsc::unbounded_channel();
    let mut conn = pool.acquire().await.expect("failed to acquire connection");

    let channels = EventChannels::new(gateway_tx, bidi_event_tx);
    create_new_session(
        &channels,
        &mut conn,
        &location,
        &user,
        &device,
        true,
        REPLACEMENT_MFA_PRESHARED_KEY.to_owned(),
    )
    .await
    .expect("should replace connected MFA session");

    let gateway_event = gateway_rx
        .try_recv()
        .expect("expected MFA gateway disconnect event for replaced connected session");
    match gateway_event {
        GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
            assert_eq!(location_id, location.id);
            assert_eq!(disconnected_device.id, device.id);
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }

    let event = event_rx
        .try_recv()
        .expect("expected session replaced audit event for replaced connected session");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::SessionSuperseded {
                location: event_location,
                device: event_device,
                is_mfa_session,
            } => {
                assert_eq!(event_location.id, location.id);
                assert_eq!(event_device.id, device.id);
                assert!(is_mfa_session);
            }
            other => panic!("unexpected bidi event: {other:?}"),
        },
        other => panic!("unexpected bidi stream event type: {other:?}"),
    }
    assert_eq!(event.context.user_id, user.id);
    assert_eq!(event.context.username, user.username);

    let old_session = VpnClientSession::find_by_id(&pool, old_session.id)
        .await
        .expect("failed to query old session")
        .expect("expected old session");
    assert_eq!(old_session.state, VpnClientSessionState::Disconnected);
}

#[sqlx::test]
async fn test_replacing_new_mfa_session_marks_session_disconnected_without_disconnect_audit_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_mfa_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;
    let old_session = VpnClientSession::new(location.id, user.id, device.id, None, true)
        .save(&pool)
        .await
        .expect("failed to create existing new MFA session");

    let (gateway_tx, mut gateway_rx) = broadcast::channel(8);
    let (bidi_event_tx, mut event_rx) = mpsc::unbounded_channel();
    let mut conn = pool.acquire().await.expect("failed to acquire connection");

    let channels = EventChannels::new(gateway_tx, bidi_event_tx);
    create_new_session(
        &channels,
        &mut conn,
        &location,
        &user,
        &device,
        true,
        REPLACEMENT_MFA_PRESHARED_KEY.to_owned(),
    )
    .await
    .expect("should replace new MFA session");

    let gateway_event = gateway_rx
        .try_recv()
        .expect("expected MFA gateway disconnect event for replaced new session");
    match gateway_event {
        GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device) => {
            assert_eq!(location_id, location.id);
            assert_eq!(disconnected_device.id, device.id);
        }
        other => panic!("unexpected gateway event: {other:?}"),
    }

    assert!(matches!(
        event_rx.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));

    let old_session = VpnClientSession::find_by_id(&pool, old_session.id)
        .await
        .expect("failed to query old session")
        .expect("expected old session");
    assert_eq!(old_session.state, VpnClientSessionState::Disconnected);
}

fn make_server(
    pool: PgPool,
) -> (
    ClientMfaServer,
    tokio::sync::mpsc::UnboundedReceiver<BidiStreamEvent>,
    tokio::sync::broadcast::Receiver<GatewayCommand>,
) {
    let (gateway_tx, gateway_rx) = broadcast::channel(8);
    let (bidi_event_tx, bidi_event_rx) = mpsc::unbounded_channel();
    let remote_mfa_responses: Arc<RwLock<HashMap<String, oneshot::Sender<String>>>> =
        Arc::default();

    (
        ClientMfaServer::new(pool, gateway_tx, bidi_event_tx, remote_mfa_responses),
        bidi_event_rx,
        gateway_rx,
    )
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn next_suffix() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

async fn create_user(pool: &PgPool) -> User<Id> {
    let suffix = next_suffix();
    User::new(
        format!("client-mfa-test-{suffix}"),
        Some("pass123"),
        "Tester".to_owned(),
        "ClientMfa".to_owned(),
        format!("client-mfa-{suffix}@example.com"),
        None,
    )
    .save(pool)
    .await
    .expect("failed to create user")
}

async fn create_device(pool: &PgPool, user_id: Id) -> Device<Id> {
    let suffix = next_suffix();
    Device::new(
        format!("client-mfa-device-{suffix}"),
        format!("client-mfa-pubkey-{suffix}"),
        user_id,
        DeviceType::User,
        None,
        true,
    )
    .save(pool)
    .await
    .expect("failed to create device")
}

/// Issues a polling token for a device, as enrollment does. Posture checks require one to
/// authenticate the caller.
async fn create_polling_token(pool: &PgPool, device_id: Id) -> String {
    PollingToken::new(device_id)
        .save(pool)
        .await
        .expect("failed to create polling token")
        .token
}

#[sqlx::test]
async fn test_create_new_mfa_session_disconnects_previous_active_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    let location = create_mfa_location(&pool).await;
    let user = create_user(&pool).await;
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let mut previous_session = VpnClientSession::new(
        location.id,
        user.id,
        device.id,
        Some(Utc::now().naive_utc()),
        true,
    );
    previous_session.preshared_key = Some("old-psk".to_owned());
    previous_session.state = VpnClientSessionState::Connected;
    let previous_session = previous_session
        .save(&pool)
        .await
        .expect("failed to create previous active MFA session");

    let (gateway_tx, mut gateway_rx) = broadcast::channel(4);
    let (bidi_event_tx, _bidi_event_rx) = mpsc::unbounded_channel();
    let mut conn = pool
        .acquire()
        .await
        .expect("failed to acquire database connection");

    let channels = EventChannels::new(gateway_tx, bidi_event_tx);
    let new_session = create_new_session(
        &channels,
        &mut conn,
        &location,
        &user,
        &device,
        true,
        NEW_MFA_PRESHARED_KEY.to_owned(),
    )
    .await
    .expect("failed to create replacement MFA session");

    let previous_session = VpnClientSession::find_by_id(&pool, previous_session.id)
        .await
        .expect("failed to reload previous session")
        .expect("expected previous session to exist");
    assert_eq!(previous_session.state, VpnClientSessionState::Disconnected);
    assert!(previous_session.disconnected_at.is_some());

    let active_sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to fetch active sessions");
    assert_eq!(active_sessions.len(), 1);
    assert_eq!(active_sessions[0].id, new_session.id);
    assert_eq!(
        active_sessions[0].preshared_key.as_deref(),
        Some(NEW_MFA_PRESHARED_KEY)
    );

    match gateway_rx.try_recv() {
        Ok(GatewayCommand::VpnSessionDeauthorized(location_id, disconnected_device)) => {
            assert_eq!(location_id, location.id);
            assert_eq!(disconnected_device.id, device.id);
        }
        Ok(other) => panic!("unexpected gateway event: {other:?}"),
        Err(error) => panic!("expected MFA disconnect gateway event, got {error:?}"),
    }
}

async fn create_mfa_location(pool: &PgPool) -> WireguardNetwork<Id> {
    WireguardNetwork::new(
        "client-mfa-location".to_owned(),
        51820,
        "vpn.example.com".to_owned(),
        None,
        [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        true,
        false,
        false,
        false,
        true, // mfa_enabled
        ServiceLocationMode::Disabled,
    )
    .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 10, 0, 1)), 24).unwrap()])
    .expect("failed to set location address")
    .save(pool)
    .await
    .expect("failed to create location")
}

async fn create_non_mfa_location(pool: &PgPool) -> WireguardNetwork<Id> {
    WireguardNetwork::new(
        "client-posture-location".to_owned(),
        51820,
        "vpn.example.com".to_owned(),
        None,
        [IpNetwork::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0).unwrap()],
        true,
        false,
        false,
        false,
        false, // mfa_enabled
        ServiceLocationMode::Disabled,
    )
    .set_address([IpNetwork::new(IpAddr::V4(Ipv4Addr::new(10, 20, 0, 1)), 24).unwrap()])
    .expect("failed to set location address")
    .save(pool)
    .await
    .expect("failed to create location")
}

async fn attach_device_to_location(pool: &PgPool, location_id: Id, device_id: Id) {
    WireguardNetworkDevice::new(
        location_id,
        device_id,
        vec![IpAddr::V4(Ipv4Addr::new(10, 10, 0, 10))],
    )
    .insert(pool)
    .await
    .expect("failed to attach device to location");
}

async fn create_and_assign_mfa_flow(pool: &PgPool, location_id: Id) {
    let mut tx = pool.begin().await.expect("failed to begin transaction");
    let (flow, _steps) = MfaFlow::create(
        &mut tx,
        "Default Internal MFA".into(),
        vec![vec![
            VpnClientMfaMethod::Totp,
            VpnClientMfaMethod::Email,
            VpnClientMfaMethod::Biometric,
            VpnClientMfaMethod::MobileApprove,
        ]],
    )
    .await
    .expect("failed to create MFA flow");
    MfaFlow::assign_to_location(
        &mut tx,
        location_id,
        &[LocationMfaFlowAssignment {
            flow_id: flow.id,
            is_default: true,
            group_ids: Vec::new(),
        }],
    )
    .await
    .expect("failed to assign MFA flow to location");
    tx.commit().await.expect("failed to commit transaction");
}

#[sqlx::test]
#[allow(deprecated)]
async fn test_finish_client_mfa_login_totp_authorizes_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    create_and_assign_mfa_flow(&pool, location.id).await;
    let mut user = create_user(&pool).await;
    let secret = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    user.totp_secret = Some(secret.clone());
    user.totp_enabled = true;
    user.save(&pool).await.expect("failed to configure TOTP");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (mut server, mut event_rx, _gateway_rx) = make_server(pool.clone());

    let start = server
        .start_client_mfa_login(
            ClientMfaStartRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Totp as i32,
                posture_data: None,
                selected_methods: Vec::new(),
            },
            device_info(),
        )
        .await
        .expect("start should succeed");
    let token = match start {
        ClientMfaStartOutcome::Approved(response) => response.token,
        ClientMfaStartOutcome::Rejected { .. } => panic!("unexpected rejection"),
    };

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let code = totp_custom::<Sha1>(
        TOTP_CODE_VALIDITY_PERIOD,
        TOTP_CODE_DIGITS,
        &secret,
        timestamp,
    );

    let response = server
        .finish_client_mfa_login(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(code),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            device_info(),
        )
        .await
        .expect("finish should succeed");
    assert!(!response.preshared_key.is_empty());

    // The successful finish is audited.
    let event = event_rx
        .try_recv()
        .expect("expected desktop client MFA success event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Success {
                attribution:
                    MfaAttribution {
                        snapshot,
                        flow_name,
                    },
                ..
            } => {
                assert_eq!(flow_name.as_deref(), Some("Default Internal MFA"));
                assert_eq!(snapshot.steps.len(), 1);
                assert_eq!(snapshot.steps[0].satisfied, Some(VpnClientMfaMethod::Totp));
                assert!(
                    snapshot.steps[0]
                        .methods
                        .contains(&VpnClientMfaMethod::Totp)
                );
            }
            other => panic!("unexpected bidi event: {other:?}"),
        },
        other => panic!("unexpected bidi stream event type: {other:?}"),
    }

    // The authorized session records only that MFA was used.
    let sessions =
        VpnClientSession::get_all_active_device_sessions_in_location(&pool, location.id, device.id)
            .await
            .expect("failed to fetch active sessions");
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].is_mfa_session);

    // The in-progress session is gone.
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn test_finish_client_mfa_login_failure_cap_deletes_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    create_and_assign_mfa_flow(&pool, location.id).await;
    let mut user = create_user(&pool).await;
    let secret = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    user.totp_secret = Some(secret);
    user.totp_enabled = true;
    user.save(&pool).await.expect("failed to configure TOTP");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (mut server, mut event_rx, _gateway_rx) = make_server(pool.clone());

    let start = server
        .start_client_mfa_login(
            ClientMfaStartRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Totp as i32,
                posture_data: None,
                selected_methods: Vec::new(),
            },
            device_info(),
        )
        .await
        .expect("start should succeed");
    let token = match start {
        ClientMfaStartOutcome::Approved(response) => response.token,
        ClientMfaStartOutcome::Rejected { .. } => panic!("unexpected rejection"),
    };

    // Repeating a wrong code trips the per-step cap and deletes the session.
    for _ in 0..MFA_FAILED_ATTEMPT_CAP {
        let result = server
            .finish_client_mfa_login(
                ClientMfaFinishRequest {
                    token: token.clone(),
                    code: Some("000000".to_owned()),
                    auth_pub_key: None,
                    step_attempt_id: None,
                },
                device_info(),
            )
            .await;
        assert!(result.is_err());
    }

    // The session is deleted once the cap is reached.
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
            .await
            .unwrap()
            .is_none()
    );

    // Each rejected proof is audited.
    for _ in 0..MFA_FAILED_ATTEMPT_CAP {
        let event = event_rx
            .try_recv()
            .expect("expected desktop client MFA failed event");
        match event.event {
            BidiStreamEventType::DesktopClientMfa(event) => match *event {
                DesktopClientMfaEvent::Failed { .. } => {}
                other => panic!("unexpected bidi event: {other:?}"),
            },
            other => panic!("unexpected bidi stream event type: {other:?}"),
        }
    }
}

async fn start_mfa_session_direct(pool: &PgPool, ttl: Duration) -> String {
    let location = create_mfa_location(pool).await;
    let user = create_user(pool).await;
    let device = create_device(pool, user.id).await;
    let mut tx = pool.begin().await.unwrap();
    let (_, outcome) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        vec![vec![VpnClientMfaMethod::Totp]],
        VpnClientMfaMethod::Totp,
        None,
        ttl,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    outcome.token
}

#[sqlx::test]
async fn test_validate_mfa_token(_: PgPoolOptions, options: PgConnectOptions) {
    let pool = setup_pool(options).await;
    let (mut server, _event_rx, _gateway_rx) = make_server(pool.clone());

    // Unknown token.
    let resp = server
        .validate_mfa_token(ClientMfaTokenValidationRequest {
            token: "nonexistent".to_owned(),
        })
        .await
        .unwrap();
    assert!(!resp.token_valid);

    // Expired token.
    let expired = start_mfa_session_direct(&pool, Duration::ZERO).await;
    let resp = server
        .validate_mfa_token(ClientMfaTokenValidationRequest { token: expired })
        .await
        .unwrap();
    assert!(!resp.token_valid);

    // Active token.
    let active = start_mfa_session_direct(&pool, VPN_MFA_SESSION_TIMEOUT).await;
    let resp = server
        .validate_mfa_token(ClientMfaTokenValidationRequest { token: active })
        .await
        .unwrap();
    assert!(resp.token_valid);
}

#[sqlx::test]
async fn test_client_mfa_step_start_returns_well_formed_response(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    let mut user = create_user(&pool).await;
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    let device = create_device(&pool, user.id).await;

    let mut tx = pool.begin().await.unwrap();
    let (_, started) = VpnClientMfaSession::<Id>::start(
        &mut tx,
        location.id,
        device.id,
        user.id,
        1,
        vec![vec![VpnClientMfaMethod::Totp]],
        VpnClientMfaMethod::Totp,
        None,
        VPN_MFA_SESSION_TIMEOUT,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let (mut server, _event_rx, _gateway_rx) = make_server(pool.clone());
    let response = server
        .client_mfa_step_start(ClientMfaStepStartRequest {
            token: started.token,
            method: MfaMethod::Totp as i32,
        })
        .await
        .expect("step start should succeed");
    assert!(!response.step_attempt_id.is_empty());
    assert!(response.challenge.is_none());
    // Step 0 is born initialized, so this is a re-call: it must supersede the attempt minted
    // by `start` rather than hand the same one back.
    assert_ne!(response.step_attempt_id, started.step_attempt_id);
}

#[sqlx::test]
async fn test_start_client_mfa_login_supersedes_existing_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    create_and_assign_mfa_flow(&pool, location.id).await;
    let mut user = create_user(&pool).await;
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (mut server, mut event_rx, _gateway_rx) = make_server(pool.clone());

    let request = || ClientMfaStartRequest {
        location_id: location.id,
        pubkey: device.wireguard_pubkey.clone(),
        #[allow(deprecated)]
        method: MfaMethod::Totp as i32,
        posture_data: None,
        selected_methods: Vec::new(),
    };

    let first = server
        .start_client_mfa_login(request(), device_info())
        .await
        .expect("first start should succeed");
    let first_token = match first {
        ClientMfaStartOutcome::Approved(response) => response.token,
        ClientMfaStartOutcome::Rejected { .. } => panic!("unexpected rejection"),
    };
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &first_token)
            .await
            .unwrap()
            .is_some()
    );

    let second = server
        .start_client_mfa_login(request(), device_info())
        .await
        .expect("second start should succeed");
    let second_token = match second {
        ClientMfaStartOutcome::Approved(response) => response.token,
        ClientMfaStartOutcome::Rejected { .. } => panic!("unexpected rejection"),
    };

    // The first token no longer validates; the second one does.
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &first_token)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &second_token)
            .await
            .unwrap()
            .is_some()
    );

    let event = event_rx
        .try_recv()
        .expect("expected an audit event for the superseded login");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::MfaLoginSuperseded {
                location: event_location,
                device: event_device,
            } => {
                assert_eq!(event_location.id, location.id);
                assert_eq!(event_device.id, device.id);
            }
            other => panic!("unexpected bidi event: {other:?}"),
        },
        other => panic!("unexpected bidi stream event type: {other:?}"),
    }
}

/// Malformed device info is rejected before anything is written. Were it parsed after the
/// session was persisted, the failing request would leave a live orphan row behind and,
/// worse, would already have superseded the caller's previous session.
#[sqlx::test]
async fn test_start_client_mfa_login_rejects_bad_device_info_without_persisting(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    create_and_assign_mfa_flow(&pool, location.id).await;
    let mut user = create_user(&pool).await;
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (mut server, _event_rx, _gateway_rx) = make_server(pool.clone());

    let request = || ClientMfaStartRequest {
        location_id: location.id,
        pubkey: device.wireguard_pubkey.clone(),
        #[allow(deprecated)]
        method: MfaMethod::Totp as i32,
        posture_data: None,
        selected_methods: Vec::new(),
    };

    let established = server
        .start_client_mfa_login(request(), device_info())
        .await
        .expect("first start should succeed");
    let established_token = match established {
        ClientMfaStartOutcome::Approved(response) => response.token,
        ClientMfaStartOutcome::Rejected { .. } => panic!("unexpected rejection"),
    };

    // A start carrying no device info must fail.
    assert!(
        server
            .start_client_mfa_login(request(), None)
            .await
            .is_err(),
        "start without device info should be rejected"
    );

    // The established session is untouched, and no orphan row was left behind.
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &established_token)
            .await
            .unwrap()
            .is_some()
    );
    let rows = sqlx::query_scalar!(
        "SELECT count(*) FROM vpn_client_mfa_session WHERE location_id = $1 AND device_id = $2",
        location.id,
        device.id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rows, Some(1));
}

#[sqlx::test]
#[allow(deprecated)]
async fn test_finish_survives_server_restart(_: PgPoolOptions, options: PgConnectOptions) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    create_and_assign_mfa_flow(&pool, location.id).await;
    let mut user = create_user(&pool).await;
    let secret = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    user.totp_secret = Some(secret.clone());
    user.totp_enabled = true;
    user.save(&pool).await.expect("failed to configure TOTP");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    // Start the login on one server instance.
    let (mut server_a, _event_rx, _gateway_rx) = make_server(pool.clone());
    let start = server_a
        .start_client_mfa_login(
            ClientMfaStartRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Totp as i32,
                posture_data: None,
                selected_methods: Vec::new(),
            },
            device_info(),
        )
        .await
        .expect("start should succeed");
    let token = match start {
        ClientMfaStartOutcome::Approved(response) => response.token,
        ClientMfaStartOutcome::Rejected { .. } => panic!("unexpected rejection"),
    };

    // A "restart" is a fresh server instance with a fresh in-memory waiter map over the
    // same database. The durable session must survive it.
    let (mut server_b, _event_rx, _gateway_rx) = make_server(pool.clone());

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let code = totp_custom::<Sha1>(
        TOTP_CODE_VALIDITY_PERIOD,
        TOTP_CODE_DIGITS,
        &secret,
        timestamp,
    );

    let response = server_b
        .finish_client_mfa_login(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(code),
                auth_pub_key: None,
                step_attempt_id: None,
            },
            device_info(),
        )
        .await
        .expect("finish should succeed after restart");
    assert!(!response.preshared_key.is_empty());
}

#[sqlx::test]
async fn test_auth_mfa_session_with_oidc_rejects_non_oidc_method(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_enterprise_license();
    let pool = setup_pool(options).await;
    initialize_current_settings(&pool)
        .await
        .expect("failed to init settings");
    let location = create_mfa_location(&pool).await;
    create_and_assign_mfa_flow(&pool, location.id).await;
    let mut user = create_user(&pool).await;
    user.enable_totp(&pool)
        .await
        .expect("failed to enable TOTP");
    let device = create_device(&pool, user.id).await;
    attach_device_to_location(&pool, location.id, device.id).await;

    let (mut server, _event_rx, _gateway_rx) = make_server(pool.clone());
    let start = server
        .start_client_mfa_login(
            ClientMfaStartRequest {
                location_id: location.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Totp as i32,
                posture_data: None,
                selected_methods: Vec::new(),
            },
            device_info(),
        )
        .await
        .expect("start should succeed");
    let token = match start {
        ClientMfaStartOutcome::Approved(response) => response.token,
        ClientMfaStartOutcome::Rejected { .. } => panic!("unexpected rejection"),
    };

    // Build a state that encodes the token and the session's step_attempt_id, as the
    // OIDC redirect does for the MFA flow.
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
        .await
        .unwrap()
        .expect("expected an active session");
    let attempt_id = session
        .ephemeral_state
        .as_ref()
        .expect("expected an attempt in progress")
        .step_attempt_id
        .clone();
    let state = build_state(Some(format!("{token}.{attempt_id}")));
    let status = server
        .auth_mfa_session_with_oidc(
            ClientMfaOidcAuthenticateRequest {
                code: "dummy".to_owned(),
                state: state.secret().to_owned(),
                nonce: "dummy".to_owned(),
            },
            device_info(),
        )
        .await
        .expect_err("a non-OIDC session must be rejected");
    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(status.message(), "invalid MFA method");

    // The mismatched session is deleted.
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&pool, &token)
            .await
            .unwrap()
            .is_none()
    );
}

fn set_enterprise_license() {
    let license = License::new(
        "test".to_owned(),
        true,
        Some(Utc::now() + chrono::TimeDelta::days(1)),
        Some(LicenseLimits {
            users: 100,
            devices: 100,
            locations: 100,
            network_devices: Some(100),
        }),
        None,
        LicenseTier::Enterprise,
        SupportType::Basic,
        Vec::new(),
    );
    set_cached_license(Some(license));
    set_counts(Counts::new(1, 1, 1, 1));
}

fn passing_linux_posture_data() -> DevicePostureData {
    DevicePostureData {
        defguard_client_version: "1.6.0".to_owned(),
        os_type: "linux".to_owned(),
        disk_encryption: Some(BoolCheck {
            result: Some(bool_check::Result::Value(true)),
        }),
        ..Default::default()
    }
}

async fn save_linux_posture_policy(pool: &PgPool, location_id: Id) {
    let policy = DevicePosture {
        id: defguard_common::db::NoId,
        name: "client-mfa-test-posture".to_owned(),
        description: None,
        min_desktop_client_version: None,
        min_mobile_client_version: None,
        allow_prerelease_client: true,
    }
    .save(pool)
    .await
    .expect("failed to save posture policy");

    DevicePostureOsRule {
        id: defguard_common::db::NoId,
        posture_id: policy.id,
        os_type: OsType::Linux,
        min_os_version: None,
        disk_encryption_required: Some(true),
        antivirus_required: None,
        ad_domain_joined_required: None,
        windows_security_update_max_age: None,
        min_kernel_version: None,
        device_integrity_required: None,
        android_security_patch_level_max_age: None,
    }
    .save(pool)
    .await
    .expect("failed to save posture OS rule");

    DevicePostureLocation::set_for_location(
        &mut pool.acquire().await.expect("failed to acquire connection"),
        location_id,
        &[policy.id],
    )
    .await
    .expect("failed to assign posture policy to location");
}
