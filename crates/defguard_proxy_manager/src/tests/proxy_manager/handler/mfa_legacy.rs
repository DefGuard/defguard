//! Pins the deprecated single-step MFA wire contract for pre-2.2 clients.

use defguard_common::{
    db::{
        Id,
        models::{
            vpn_client_mfa_session::VpnClientMfaSession, vpn_client_session::VpnClientSession,
        },
    },
    gateway_event::GatewayCommand,
};
use defguard_core::events::{BidiStreamEventType, DesktopClientMfaEvent};
use defguard_proto::{
    client_types::MfaMethod,
    proxy::{AwaitRemoteMfaFinishRequest, CoreRequest, core_request, core_response},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{task, time::timeout};
use tonic::Code;

use super::support::{
    assert_error_response_with_message, assert_vpn_session_exists, biometric_pub_key,
    complete_proxy_handshake, configure_oidc_provider, create_external_mfa_network,
    create_mfa_network, create_user_with_device, expect_bidi_mfa_success, generate_totp_code,
    link_user_oidc_identity, register_biometric_key, send_mfa_finish, send_mfa_finish_no_recv,
    send_mfa_finish_raw, send_mfa_finish_signed, send_mfa_start, send_mfa_start_with_challenge,
    set_test_license_business, setup_user_email_mfa, setup_user_totp_mfa, sign_challenge,
};
use crate::tests::common::{HandlerTestContext, RECEIVE_TIMEOUT};

const AWAIT_ID: u64 = 8000;

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

    // Subscribe before finish so the handler's gateway_tx.send() has a receiver,
    // and keep the receiver alive so we can assert on the event.
    let mut gateway_rx = context.gateway_tx.subscribe();

    let code = generate_totp_code(&user);
    let (_, psk) = send_mfa_finish(&mut context, &token, Some(&code)).await;
    assert!(
        !psk.is_empty(),
        "PSK must not be empty after successful TOTP MFA"
    );

    // Verify VpnClientSession was persisted.
    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(session.preshared_key.is_some());

    // Verify GatewayCommand::VpnSessionAuthorized was broadcast.
    // Use the already-subscribed receiver - subscribing after send_mfa_finish would miss the event.
    let event = timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayCommand::VpnSessionAuthorized")
        .expect("gateway command channel closed");
    let gateway_loc_id = match event {
        GatewayCommand::VpnSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected VpnSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(gateway_loc_id, network.id);

    // Verify BidiStreamEvent::DesktopClientMfa(Success) was emitted.
    let event_loc_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(event_loc_id, network.id);

    context.finish().await.expect_server_finished().await;
}

/// The legacy single-step biometric flow completes end-to-end against the DB-backed session.
///
/// `start` issues a challenge bound to the device's enrolled key; `finish` returns the signature
/// as `code` and the handler verifies it against that key.
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
    let (code, message) = assert_error_response_with_message(&response);
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

/// The legacy single-step mobile-approve flow completes end-to-end against the DB-backed session.
///
/// This is the fused path: the approving device's key rides in `auth_pub_key` on `finish` and the
/// handler verifies the signature and authorizes in one call. The durable-mark route, where an
/// out-of-band approval is collected by a later `finish` poll, is covered by Chunk 2; only parked
/// waiter wake and relay delivery remain for Chunk 3 and #3046.
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

    let signature = sign_challenge(&signing_key, &challenge);
    let (_, psk) =
        send_mfa_finish_signed(&mut context, &token, Some(&signature), Some(&auth_pub_key)).await;
    assert!(
        !psk.is_empty(),
        "PSK must not be empty after successful mobile-approve MFA"
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
async fn test_mfa_finish_succeeds_and_creates_session(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    // Setup email MFA - the code is the same one that start_client_mfa_login
    // will regenerate internally, so we can generate it once here.
    let code = setup_user_email_mfa(&context.pool, &mut user).await;

    let (_, token) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Email,
    )
    .await;

    // Subscribe to the gateway broadcast BEFORE calling finish, so that the
    // handler's gateway_tx.send() has at least one active receiver (without
    // one the send would fail with SendError and return Internal).
    let mut gateway_rx = context.gateway_tx.subscribe();

    // The start handler has already called generate_email_mfa_code internally
    // and the in-memory secret is still the same, so regenerating here gives
    // the same code.
    let _ = code; // keep binding so the setup_user_email_mfa call is not dead
    // Regenerate for the finish call (same secret → same code while within window)
    let finish_code = user.generate_email_mfa_code().expect("generate email code");

    let (_, psk) = send_mfa_finish(&mut context, &token, Some(&finish_code)).await;
    assert!(!psk.is_empty(), "preshared key must not be empty");

    // Verify VpnClientSession was persisted
    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(session.preshared_key.is_some());

    // Verify GatewayCommand::VpnSessionAuthorized was broadcast
    let event = timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayCommand::VpnSessionAuthorized")
        .expect("gateway command channel closed");
    let loc_id = match event {
        GatewayCommand::VpnSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected VpnSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(loc_id, network.id);

    // Verify BidiStreamEvent::DesktopClientMfa(Success) was sent
    let event_loc_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(event_loc_id, network.id);

    context.finish().await.expect_server_finished().await;
}

/// The callback itself is covered by the core OIDC handler. This pins the legacy Start/Finish
/// contract around its durable completion mark without needing an external identity provider.
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
    let (code, message) = assert_error_response_with_message(&response);
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
    let event = context
        .bidi_events_rx
        .try_recv()
        .expect("legacy OIDC poll must emit its failure audit");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Failed {
                method, message, ..
            } => {
                assert_eq!(method, MfaMethod::Oidc);
                assert_eq!(
                    message,
                    "tried to finish OIDC MFA login but they haven't completed OIDC authentication yet"
                );
            }
            other => panic!("expected MFA failure audit, got {other:?}"),
        },
        other => panic!("expected desktop MFA event, got {other:?}"),
    }

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
#[allow(deprecated)]
async fn test_mfa_await_remote_receives_psk_after_finish(
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

    // Send AwaitRemoteMfaFinish first - no immediate response expected
    context.mock_proxy().send_request(CoreRequest {
        id: AWAIT_ID,
        device_info: None,
        payload: Some(core_request::Payload::AwaitRemoteMfaFinish(
            AwaitRemoteMfaFinishRequest {
                token: token.clone(),
            },
        )),
    });

    // Give the handler one poll cycle to register the oneshot receiver before
    // we proceed with the finish call.
    task::yield_now().await;

    // Subscribe before finish so the handler's gateway_tx.send() has a receiver
    let _gateway_rx = context.gateway_tx.subscribe();

    // Now finish the MFA login with the correct code.  Use the no-recv variant
    // because two responses will arrive (ClientMfaFinish + AwaitRemoteMfaFinish)
    // and we collect them both below.
    let code = user.generate_email_mfa_code().expect("generate email code");
    send_mfa_finish_no_recv(&mut context, &token, Some(&code)).await;

    // Two responses should arrive: one ClientMfaFinish and one
    // AwaitRemoteMfaFinish - order is not guaranteed.
    let r1 = context.mock_proxy_mut().recv_outbound().await;
    let r2 = context.mock_proxy_mut().recv_outbound().await;

    let mut got_finish = false;
    let mut got_await = false;
    for r in [&r1, &r2] {
        match &r.payload {
            Some(core_response::Payload::ClientMfaFinish(fr)) => {
                assert!(!fr.preshared_key.is_empty());
                got_finish = true;
            }
            Some(core_response::Payload::AwaitRemoteMfaFinish(ar)) => {
                assert!(!ar.preshared_key.is_empty());
                got_await = true;
            }
            other => panic!(
                "unexpected response payload: {:?}",
                other.as_ref().map(std::mem::discriminant)
            ),
        }
    }
    assert!(got_finish, "missing ClientMfaFinish response");
    assert!(got_await, "missing AwaitRemoteMfaFinish response");

    context.finish().await.expect_server_finished().await;
}
