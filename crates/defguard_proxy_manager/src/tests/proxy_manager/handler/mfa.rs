use std::time::Duration;

use defguard_common::db::Id;
use defguard_core::grpc::GatewayEvent;
use defguard_proto::proxy::{
    AwaitRemoteMfaFinishRequest, ClientMfaFinishRequest, ClientMfaStartRequest, CoreRequest,
    MfaMethod, core_request, core_response,
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{task, time::timeout};
use tonic::Code;

use super::support::{
    assert_error_response, assert_vpn_session_exists, clear_test_license, complete_proxy_handshake,
    create_external_mfa_network, create_mfa_network, create_network, create_user_with_device,
    expect_bidi_mfa_success, generate_totp_code, make_device_info, send_mfa_finish,
    send_mfa_finish_no_recv, send_mfa_finish_raw, send_mfa_start, send_token_validation,
    setup_user_email_mfa, setup_user_totp_mfa,
};
use crate::tests::common::HandlerTestContext;

const EVENT_RECEIVE_TIMEOUT: Duration = Duration::from_secs(5);
const WRONG_REQUEST_ID: u64 = 9991;
const AWAIT_ID: u64 = 8000;

#[sqlx::test]
async fn test_mfa_start_fails_for_disabled_location(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // create a network with MFA *disabled* (the default)
    let network = create_network(&context.pool).await;
    let (_, device) = create_user_with_device(&context.pool).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_start_fails_for_unknown_location(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Create a device so the pubkey lookup succeeds — the handler checks the
    // location_id first, but using a real pubkey avoids masking the error.
    let (_, device) = create_user_with_device(&context.pool).await;

    // Use an ID that is guaranteed not to correspond to any WireguardNetwork row.
    let nonexistent_location_id = Id::MAX;

    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: nonexistent_location_id,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(
        code,
        Code::InvalidArgument,
        "unknown location_id must return InvalidArgument"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_start_returns_token_for_totp(_: PgPoolOptions, options: PgConnectOptions) {
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
    assert!(!token.is_empty(), "TOTP start token must not be empty");

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

    // Subscribe before finish so the handler's wireguard_tx.send() has a receiver,
    // and keep the receiver alive so we can assert on the event.
    let mut gateway_rx = context.wireguard_tx.subscribe();

    let code = generate_totp_code(&user);
    let (_, psk) = send_mfa_finish(&mut context, &token, Some(&code)).await;
    assert!(
        !psk.is_empty(),
        "PSK must not be empty after successful TOTP MFA"
    );

    // Verify VpnClientSession was persisted.
    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(session.preshared_key.is_some());

    // Verify GatewayEvent::MfaSessionAuthorized was broadcast.
    // Use the already-subscribed receiver — subscribing after send_mfa_finish would miss the event.
    let event = timeout(EVENT_RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayEvent::MfaSessionAuthorized")
        .expect("gateway event channel closed");
    let gateway_loc_id = match event {
        GatewayEvent::MfaSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected MfaSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(gateway_loc_id, network.id);

    // Verify BidiStreamEvent::DesktopClientMfa(Success) was emitted.
    let event_loc_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(event_loc_id, network.id);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_finish_fails_with_wrong_totp_code(_: PgPoolOptions, options: PgConnectOptions) {
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

    // Send a clearly wrong code.
    context.mock_proxy().send_request(CoreRequest {
        id: WRONG_REQUEST_ID,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some("000000".to_string()),
                auth_pub_key: None,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert!(
        matches!(code, Code::InvalidArgument | Code::Unauthenticated),
        "wrong TOTP code should return InvalidArgument or Unauthenticated, got: {code:?}"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_start_fails_for_unknown_device(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: "no-such-pubkey".to_string(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_start_fails_when_email_mfa_not_enabled(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    // device is created after the network so add_to_all_networks picks it up
    let (_, device) = create_user_with_device(&context.pool).await;
    // user.email_mfa_enabled is false by default — no setup call

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Email as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_start_returns_token_for_email_mfa(_: PgPoolOptions, options: PgConnectOptions) {
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
    assert!(!token.is_empty(), "token must not be empty");

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_finish_succeeds_and_creates_session(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    // Setup email MFA — the code is the same one that start_client_mfa_login
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
    // handler's wireguard_tx.send() has at least one active receiver (without
    // one the send would fail with SendError and return Internal).
    let mut gateway_rx = context.wireguard_tx.subscribe();

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

    // Verify GatewayEvent::MfaSessionAuthorized was broadcast
    let event = timeout(EVENT_RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for GatewayEvent::MfaSessionAuthorized")
        .expect("gateway event channel closed");
    let loc_id = match event {
        GatewayEvent::MfaSessionAuthorized(loc_id, _, _) => loc_id,
        other => panic!("expected MfaSessionAuthorized, got: {other:?}"),
    };
    assert_eq!(loc_id, network.id);

    // Verify BidiStreamEvent::DesktopClientMfa(Success) was sent
    let event_loc_id = expect_bidi_mfa_success(&mut context.bidi_events_rx).await;
    assert_eq!(event_loc_id, network.id);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_token_valid_before_finish_invalid_after(
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

    // Token should be valid while session is in-progress
    let valid = send_token_validation(&mut context, &token).await;
    assert!(valid, "token must be valid after start");

    // Subscribe before finish so the handler's wireguard_tx.send() has a receiver
    let _gateway_rx = context.wireguard_tx.subscribe();

    let code = user.generate_email_mfa_code().expect("generate email code");
    send_mfa_finish(&mut context, &token, Some(&code)).await;

    // After finish the session is removed, so token is no longer valid
    let valid_after = send_token_validation(&mut context, &token).await;
    assert!(!valid_after, "token must be invalid after finish");

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_finish_fails_with_wrong_code(_: PgPoolOptions, options: PgConnectOptions) {
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

    // Send a clearly wrong code — use _raw so we can inspect the error response
    let response = send_mfa_finish_raw(&mut context, &token, Some("000000")).await;
    let code = assert_error_response(&response);
    // invalid code → InvalidArgument or Unauthenticated
    assert!(
        matches!(code, Code::InvalidArgument | Code::Unauthenticated),
        "expected InvalidArgument or Unauthenticated, got: {code:?}"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_oidc_start_requires_license(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    clear_test_license();

    // External MFA location + OIDC method but no business license
    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    // email MFA is irrelevant for OIDC path but user still needs to exist
    setup_user_email_mfa(&context.pool, &mut user).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: None,
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                method: MfaMethod::Oidc as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
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

    // Send AwaitRemoteMfaFinish first — no immediate response expected
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

    // Subscribe before finish so the handler's wireguard_tx.send() has a receiver
    let _gateway_rx = context.wireguard_tx.subscribe();

    // Now finish the MFA login with the correct code.  Use the no-recv variant
    // because two responses will arrive (ClientMfaFinish + AwaitRemoteMfaFinish)
    // and we collect them both below.
    let code = user.generate_email_mfa_code().expect("generate email code");
    send_mfa_finish_no_recv(&mut context, &token, Some(&code)).await;

    // Two responses should arrive: one ClientMfaFinish and one
    // AwaitRemoteMfaFinish — order is not guaranteed.
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

/// When a second MFA cycle completes for the same device+location the handler
/// must:
///  - disconnect the first `VpnClientSession` (state → Disconnected),
///  - emit `GatewayEvent::MfaSessionDisconnected` for the first session, and
///  - create a new active `VpnClientSession`.
#[sqlx::test]
async fn test_mfa_finish_replaces_existing_session_disconnects_old(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;

    // ---- First MFA cycle ----
    // Must subscribe before finish so the send has a receiver.
    let _gw_rx1 = context.wireguard_tx.subscribe();

    let (_, token1) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Totp,
    )
    .await;

    let code1 = generate_totp_code(&user);
    let (_, psk1) = send_mfa_finish(&mut context, &token1, Some(&code1)).await;
    assert!(
        !psk1.is_empty(),
        "first MFA cycle must return a non-empty PSK"
    );

    // First session must exist in the DB.
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;

    // Rotate to a fresh TOTP secret before the second cycle.
    // This guarantees the second code is different from the first without
    // waiting for the 30-second window to advance.
    user.new_totp_secret(&context.pool)
        .await
        .expect("new_totp_secret (second cycle)");
    user.enable_totp(&context.pool)
        .await
        .expect("enable_totp (second cycle)");

    // ---- Second MFA cycle ----
    let (_, token2) = send_mfa_start(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        MfaMethod::Totp,
    )
    .await;

    // Subscribe before finish so both MfaSessionDisconnected and
    // MfaSessionAuthorized have an active receiver.
    let mut gw_rx2 = context.wireguard_tx.subscribe();

    let code2 = generate_totp_code(&user);
    let (_, psk2) = send_mfa_finish(&mut context, &token2, Some(&code2)).await;
    assert!(
        !psk2.is_empty(),
        "second MFA cycle must return a non-empty PSK"
    );

    // Receive events from the gateway broadcast channel.  The handler sends
    // MfaSessionDisconnected (for the old session) and then MfaSessionAuthorized
    // (for the new session) in that order.
    let mut got_disconnected = false;
    let mut got_authorized = false;
    for _ in 0..2 {
        let event = timeout(EVENT_RECEIVE_TIMEOUT, gw_rx2.recv())
            .await
            .expect("timed out waiting for gateway event after second MFA finish")
            .expect("gateway event channel closed");

        match event {
            GatewayEvent::MfaSessionDisconnected(loc_id, ref dev) => {
                assert_eq!(loc_id, network.id, "disconnected session location mismatch");
                assert_eq!(dev.id, device.id, "disconnected session device mismatch");
                got_disconnected = true;
            }
            GatewayEvent::MfaSessionAuthorized(loc_id, _, _) => {
                assert_eq!(loc_id, network.id, "authorized session location mismatch");
                got_authorized = true;
            }
            other => panic!("unexpected gateway event: {other:?}"),
        }
    }
    assert!(got_disconnected, "MfaSessionDisconnected must be emitted");
    assert!(got_authorized, "MfaSessionAuthorized must be emitted");

    // New session must exist in the DB.
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;

    context.finish().await.expect_server_finished().await;
}
