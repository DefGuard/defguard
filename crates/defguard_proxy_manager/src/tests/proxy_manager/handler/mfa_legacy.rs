//! Tests old single-step MFA behavior for pre-2.2 clients.

use defguard_common::{
    db::{
        Id,
        models::{
            vpn_client_mfa_session::VpnClientMfaSession,
            vpn_client_session::{VpnClientMfaMethod, VpnClientSession},
        },
    },
    gateway_event::GatewayCommand,
};
use defguard_core::events::{BidiStreamEventType, DesktopClientMfaEvent};
use defguard_proto::{
    client_types::{ClientMfaFinishRequest, LocationMfaMode, MfaMethod, NewDevice},
    proxy::{AwaitRemoteMfaFinishRequest, CoreRequest, DeviceInfo, core_request, core_response},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{task, time::timeout};
use tonic::Code;

use super::support::{
    assert_device_config_response, assert_error_response_details, assert_vpn_session_exists,
    biometric_pub_key, complete_proxy_handshake, configure_oidc_provider, create_enrollment_token,
    create_external_mfa_network, create_mfa_network, create_mfa_network_with_methods, create_user,
    create_user_with_device, expect_bidi_mfa_success, generate_totp_code, link_user_oidc_identity,
    make_device_info, register_biometric_key, send_mfa_finish, send_mfa_finish_no_recv,
    send_mfa_finish_raw, send_mfa_start, send_mfa_start_raw, send_mfa_start_with_challenge,
    set_test_license_business, setup_user_email_mfa, setup_user_totp_mfa, sign_challenge,
    start_enrollment_session,
};
use crate::tests::common::{HandlerTestContext, RECEIVE_TIMEOUT};

const AWAIT_ID: u64 = 8000;
const INTERNAL_METHODS_WITH_FIDO2: [VpnClientMfaMethod; 5] = [
    VpnClientMfaMethod::Totp,
    VpnClientMfaMethod::Email,
    VpnClientMfaMethod::Biometric,
    VpnClientMfaMethod::MobileApprove,
    VpnClientMfaMethod::Fido2,
];

#[sqlx::test]
#[allow(deprecated)]
async fn test_legacy_device_config_includes_internal_mode_with_fido2(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network =
        create_mfa_network_with_methods(&context.pool, INTERNAL_METHODS_WITH_FIDO2.to_vec()).await;
    let user = create_user(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    start_enrollment_session(&mut context, &token.id).await;
    let pubkey = "AA0aJzRBTltodYKPnKm2w9Dd6vcEER4rOEVSX2x5hpM=";
    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: Some(DeviceInfo {
            version: Some("2.1.0".to_owned()),
            ..make_device_info()
        }),
        payload: Some(core_request::Payload::NewDevice(NewDevice {
            name: "Legacy MFA Test Device".to_owned(),
            pubkey: pubkey.to_owned(),
            token: Some(token.id),
        })),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let config = assert_device_config_response(&response);
    let location_config = config
        .configs
        .iter()
        .find(|config| config.network_name == network.name)
        .expect("legacy device config should include the FIDO2-capable location");
    assert_eq!(
        location_config.location_mfa_mode,
        Some(LocationMfaMode::Internal as i32),
        "legacy device config should advertise the location as Internal"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_finish_succeeds_with_totp_code(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;

    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Totp,
    )
    .await;

    // Subscribe before finish so the gateway send has a receiver.
    let mut gateway_rx = context.gateway_tx.subscribe();

    let code = generate_totp_code(&user);
    let (_, psk) = send_mfa_finish(&mut context, &token, Some(&code)).await;
    assert!(
        !psk.is_empty(),
        "PSK must not be empty after successful TOTP MFA"
    );

    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(session.preshared_key.is_some());

    // Reuse the receiver; subscribing after send_mfa_finish would miss the event.
    let event = timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayCommand::VpnSessionAuthorized")
        .expect("gateway command channel closed");
    let gateway_loc_id = match event {
        GatewayCommand::VpnSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected VpnSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(gateway_loc_id, network.id);

    let event_loc_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(event_loc_id, network.id);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_totp_login_completes_with_fido2_on_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network =
        create_mfa_network_with_methods(&context.pool, INTERNAL_METHODS_WITH_FIDO2.to_vec()).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Totp,
    )
    .await;
    let mut gateway_rx = context.gateway_tx.subscribe();
    let code = generate_totp_code(&user);
    let (response, psk) = send_mfa_finish(&mut context, &token, Some(&code)).await;
    assert!(
        matches!(
            response.payload,
            Some(core_response::Payload::ClientMfaFinish(_))
        ),
        "legacy TOTP finish should return ClientMfaFinish"
    );
    assert!(!psk.is_empty(), "successful TOTP MFA must return a PSK");
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(matches!(
        timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
            .await
            .expect("timed out waiting for gateway authorization")
            .expect("gateway command channel closed"),
        GatewayCommand::VpnSessionAuthorized(location_id, _, _) if location_id == network.id
    ));

    context.finish().await.expect_server_finished().await;
}

/// Old biometric MFA completes through Start and Finish.
#[sqlx::test]
async fn test_mfa_finish_succeeds_with_biometric_signature(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;

    let (_, token, challenge) = send_mfa_start_with_challenge(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Biometric,
    )
    .await;
    let challenge = challenge.expect("biometric start must return a challenge to sign");

    // Subscribe before finish so the handler's gateway_tx.send() has a receiver.
    let mut gateway_rx = context.gateway_tx.subscribe();

    let signature = sign_challenge(&signing_key, &challenge);
    let (_, psk) = send_mfa_finish(&mut context, &token, Some(&signature)).await;
    assert!(
        !psk.is_empty(),
        "PSK must not be empty after successful biometric MFA"
    );

    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(session.preshared_key.is_some());

    let event = timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayCommand::VpnSessionAuthorized")
        .expect("gateway command channel closed");
    let gateway_loc_id = match event {
        GatewayCommand::VpnSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected VpnSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(gateway_loc_id, network.id);

    let event_loc_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(event_loc_id, network.id);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_finish_rejects_empty_legacy_mobile_approve_proof(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    register_biometric_key(&context.pool, device.id).await;

    let (_, token, _) = send_mfa_start_with_challenge(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::MobileApprove,
    )
    .await;
    let mut gateway_rx = context.gateway_tx.subscribe();

    let response = send_mfa_finish_raw(&mut context, &token, None).await;
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "Signature not found in request");
    assert!(gateway_rx.try_recv().is_err());
    assert!(context.bidi_events_rx.try_recv().is_err());
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
            .await
            .expect("failed to load mobile approval session")
            .is_some()
    );

    context.finish().await.expect_server_finished().await;
}

/// Old mobile approval verifies and connects in one Finish call.
#[sqlx::test]
async fn test_mfa_finish_succeeds_with_mobile_approve_signature(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);

    let (_, token, challenge) = send_mfa_start_with_challenge(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::MobileApprove,
    )
    .await;
    let challenge = challenge.expect("mobile approve start must return a challenge to sign");

    let mut gateway_rx = context.gateway_tx.subscribe();

    context.mock_proxy().send_request(CoreRequest {
        id: AWAIT_ID,
        device_info: None,
        payload: Some(core_request::Payload::AwaitRemoteMfaFinish(
            AwaitRemoteMfaFinishRequest {
                token: token.clone(),
            },
        )),
    });
    task::yield_now().await;

    let signature = sign_challenge(&signing_key, &challenge);
    context.mock_proxy().send_request(CoreRequest {
        id: AWAIT_ID + 1,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(signature),
                auth_pub_key: Some(auth_pub_key),
            },
        )),
    });

    let first = context.mock_proxy_mut().recv_outbound().await;
    let second = context.mock_proxy_mut().recv_outbound().await;
    let mut parked_key = None;
    for response in [&first, &second] {
        match &response.payload {
            Some(core_response::Payload::ClientMfaFinish(result)) => {
                assert_eq!(response.id, AWAIT_ID + 1);
                assert!(result.preshared_key.is_empty());
            }
            Some(core_response::Payload::AwaitRemoteMfaFinish(result)) => {
                assert_eq!(response.id, AWAIT_ID);
                assert!(!result.preshared_key.is_empty());
                parked_key = Some(result.preshared_key.clone());
            }
            _ => panic!("unexpected response"),
        }
    }
    assert!(
        !parked_key
            .as_ref()
            .expect("parked response must contain a key")
            .is_empty()
    );

    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert_eq!(session.preshared_key.as_ref(), parked_key.as_ref());

    let event = timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayCommand::VpnSessionAuthorized")
        .expect("gateway command channel closed");
    let gateway_loc_id = match event {
        GatewayCommand::VpnSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected VpnSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(gateway_loc_id, network.id);

    let event = context
        .bidi_events_rx
        .try_recv()
        .expect("expected mobile-approve success event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Success {
                mobile_auth_device_name,
                ..
            } => assert_eq!(mobile_auth_device_name, Some(device.name.clone())),
            other => panic!("expected MFA success event, got: {other:?}"),
        },
        other => panic!("expected desktop MFA event, got: {other:?}"),
    }

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_mobile_approve_starts_with_fido2_on_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network =
        create_mfa_network_with_methods(&context.pool, INTERNAL_METHODS_WITH_FIDO2.to_vec()).await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    register_biometric_key(&context.pool, device.id).await;
    let (_, token, challenge) = send_mfa_start_with_challenge(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::MobileApprove,
    )
    .await;
    assert!(
        challenge.is_some(),
        "legacy mobile-approve start should return a challenge"
    );
    assert!(
        VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
            .await
            .expect("failed to load mobile-approve session")
            .is_some(),
        "legacy mobile-approve start should create a session"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_finish_succeeds_and_creates_session(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    // Set up email MFA; start and finish reuse the same secret.
    let code = setup_user_email_mfa(&context.pool, &mut user).await;

    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Email,
    )
    .await;

    // Subscribe before finish so the gateway send has a receiver.
    let mut gateway_rx = context.gateway_tx.subscribe();

    let _ = code; // keep binding so the setup_user_email_mfa call is not dead
    // Generate the finish code from the same secret.
    let finish_code = user.generate_email_mfa_code().expect("generate email code");

    let (_, psk) = send_mfa_finish(&mut context, &token, Some(&finish_code)).await;
    assert!(!psk.is_empty(), "preshared key must not be empty");

    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(session.preshared_key.is_some());

    let event = timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayCommand::VpnSessionAuthorized")
        .expect("gateway command channel closed");
    let loc_id = match event {
        GatewayCommand::VpnSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected VpnSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(loc_id, network.id);

    let event_loc_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(event_loc_id, network.id);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_email_login_completes_with_fido2_on_location(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network =
        create_mfa_network_with_methods(&context.pool, INTERNAL_METHODS_WITH_FIDO2.to_vec()).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_email_mfa(&context.pool, &mut user).await;
    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Email,
    )
    .await;
    let mut gateway_rx = context.gateway_tx.subscribe();
    let code = user.generate_email_mfa_code().expect("generate email code");
    let (response, psk) = send_mfa_finish(&mut context, &token, Some(&code)).await;
    assert!(
        matches!(
            response.payload,
            Some(core_response::Payload::ClientMfaFinish(_))
        ),
        "legacy email finish should return ClientMfaFinish"
    );
    assert!(!psk.is_empty(), "successful email MFA must return a PSK");
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(matches!(
        timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
            .await
            .expect("timed out waiting for gateway authorization")
            .expect("gateway command channel closed"),
        GatewayCommand::VpnSessionAuthorized(location_id, _, _) if location_id == network.id
    ));

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_start_rejects_fido2_on_real_location_without_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network =
        create_mfa_network_with_methods(&context.pool, INTERNAL_METHODS_WITH_FIDO2.to_vec()).await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let response = send_mfa_start_raw(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Fido2,
    )
    .await;
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, Code::Unimplemented);
    assert_eq!(message, "Selected MFA method is not supported");

    let session_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM vpn_client_mfa_session WHERE location_id = $1 AND device_id = $2",
    )
    .bind(network.id)
    .bind(device.id)
    .fetch_one(&context.pool)
    .await
    .expect("failed to count MFA sessions after rejected FIDO2 start");
    assert_eq!(
        session_count, 0,
        "rejected FIDO2 start must create no session"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
#[allow(deprecated)]
async fn test_legacy_start_rejects_partial_internal_set_with_generic_update_message(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network_with_methods(
        &context.pool,
        vec![VpnClientMfaMethod::Totp, VpnClientMfaMethod::Fido2],
    )
    .await;
    let mut user = create_user(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    start_enrollment_session(&mut context, &token.id).await;
    let pubkey = "BB1bKzSCUmupeZLQnLn3x0Ee7wfGF5sROR0WY3y6iqM=";
    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(DeviceInfo {
            version: Some("2.2.0".to_owned()),
            ..make_device_info()
        }),
        payload: Some(core_request::Payload::NewDevice(NewDevice {
            name: "Partial MFA Test Device".to_owned(),
            pubkey: pubkey.to_owned(),
            token: Some(token.id),
        })),
    });

    let config_response = context.mock_proxy_mut().recv_outbound().await;
    let config = assert_device_config_response(&config_response);
    let location_config = config
        .configs
        .iter()
        .find(|config| config.network_name == network.name)
        .expect("2.2 device config should include the partial-method location");
    assert_eq!(
        location_config.location_mfa_mode, None,
        "a subset of internal methods plus FIDO2 has no legacy MFA mode"
    );

    let response = send_mfa_start_raw(&mut context, network.id, pubkey, MfaMethod::Totp).await;
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, Code::FailedPrecondition);
    assert_eq!(
        message,
        "Defguard client version is too old to connect to this location. Please update your client."
    );

    context.finish().await.expect_server_finished().await;
}

/// Old OIDC Finish completes after the callback marks the session.
#[sqlx::test]
async fn test_mfa_finish_succeeds_after_oidc_completion(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    configure_oidc_provider(&context.pool).await;

    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    link_user_oidc_identity(&context.pool, &mut user).await;
    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Oidc,
    )
    .await;
    let mut gateway_rx = context.gateway_tx.subscribe();

    let response = send_mfa_finish_raw(&mut context, &token, None).await;
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, Code::FailedPrecondition);
    assert_eq!(message, "OIDC authentication not completed yet");
    assert!(
        gateway_rx.try_recv().is_err(),
        "OIDC poll must not authorize"
    );
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &context.pool,
            network.id,
            device.id
        )
        .await
        .expect("failed to query authorized VPN sessions")
        .is_empty(),
        "OIDC poll must not create a VPN session"
    );

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("failed to load OIDC MFA session")
        .expect("OIDC MFA session must remain active");
    assert_eq!(
        session.failed_attempts, 0,
        "OIDC poll must not charge the cap"
    );
    let attempt_id = session
        .ephemeral_state
        .as_ref()
        .expect("OIDC MFA attempt must remain initialized")
        .step_attempt_id
        .clone();
    assert!(
        context.bidi_events_rx.try_recv().is_err(),
        "OIDC poll must not emit an activity log event"
    );

    let mut conn = context
        .pool
        .acquire()
        .await
        .expect("failed to acquire connection");
    assert!(
        session
            .mark_oidc_completed(&mut conn, &attempt_id)
            .await
            .expect("failed to mark OIDC MFA complete"),
        "current OIDC attempt must be marked complete"
    );

    let (_, preshared_key) = send_mfa_finish(&mut context, &token, None).await;
    assert!(
        !preshared_key.is_empty(),
        "legacy OIDC finish must return a PSK"
    );
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(matches!(
        timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
            .await
            .expect("timed out waiting for gateway authorization")
            .expect("gateway command channel closed"),
        GatewayCommand::VpnSessionAuthorized(location_id, _, _) if location_id == network.id
    ));
    assert_eq!(
        expect_bidi_mfa_success(&mut context.bidi_events_rx).await,
        network.id
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_await_remote_does_not_receive_psk_after_email_finish(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_email_mfa(&context.pool, &mut user).await;

    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Email,
    )
    .await;

    // Park the waiter; no immediate response is expected.
    context.mock_proxy().send_request(CoreRequest {
        id: AWAIT_ID,
        device_info: None,
        payload: Some(core_request::Payload::AwaitRemoteMfaFinish(
            AwaitRemoteMfaFinishRequest {
                token: token.clone(),
            },
        )),
    });

    // Let the handler register the waiter.
    task::yield_now().await;

    // Subscribe before finish so the gateway send has a receiver.
    let _gateway_rx = context.gateway_tx.subscribe();

    // Finish without receiving so the response can be checked below.
    let code = user.generate_email_mfa_code().expect("generate email code");
    send_mfa_finish_no_recv(&mut context, &token, Some(&code)).await;

    let response = context.mock_proxy_mut().recv_outbound().await;
    match response.payload {
        Some(core_response::Payload::ClientMfaFinish(response)) => {
            assert!(!response.preshared_key.is_empty());
        }
        other => panic!(
            "expected ClientMfaFinish response, got {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    }
    context.mock_proxy_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
}
