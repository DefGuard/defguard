use defguard_common::{
    db::{Id, models::vpn_client_session::VpnClientSession},
    gateway_event::GatewayCommand,
};
use defguard_proto::{
    client_types::{
        ClientMfaFinishRequest, ClientMfaStartRequest, MfaMethod, MfaStepResult, mfa_step_result,
    },
    proxy::{CoreRequest, core_request, core_response},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::time::timeout;
use tonic::Code;

use super::support::{
    assert_error_response, assert_error_response_with_message, assert_vpn_session_exists,
    clear_test_license, complete_proxy_handshake, create_external_mfa_network, create_mfa_network,
    create_multi_step_mfa_network, create_network, create_user_with_device,
    expect_bidi_mfa_success, generate_totp_code, make_device_info, send_mfa_finish,
    send_mfa_finish_raw, send_mfa_start, send_mfa_start_multi_step, send_mfa_step_start,
    send_token_validation, set_test_license_business, setup_user_email_mfa, setup_user_totp_mfa,
};
use crate::tests::common::{HandlerTestContext, RECEIVE_TIMEOUT};

const WRONG_REQUEST_ID: u64 = 9991;

#[sqlx::test]
async fn test_mfa_start_fails_for_disabled_location(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // create a network with MFA *disabled* (the default)
    let network = create_network(&context.pool).await;
    let (_, device) = create_user_with_device(&context.pool).await;

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Email as i32,
                posture_data: None,
                selected_methods: Vec::new(),
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

    // Create a device so the pubkey lookup succeeds - the handler checks the
    // location_id first, but using a real pubkey avoids masking the error.
    let (_, device) = create_user_with_device(&context.pool).await;

    // Use an ID that is guaranteed not to correspond to any WireguardNetwork row.
    let nonexistent_location_id = Id::MAX;

    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: nonexistent_location_id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Email as i32,
                posture_data: None,
                selected_methods: Vec::new(),
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
                code: Some("000000".to_owned()),
                auth_pub_key: None,
                step_attempt_id: None,
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
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: "no-such-pubkey".to_owned(),
                #[allow(deprecated)]
                method: MfaMethod::Email as i32,
                posture_data: None,
                selected_methods: Vec::new(),
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
    // user.email_mfa_enabled is false by default - no setup call

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Email as i32,
                posture_data: None,
                selected_methods: Vec::new(),
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let code = assert_error_response(&response);
    assert_eq!(code, Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}

/// Email MFA needs a working SMTP server, not just the per-user flag.
///
/// `test_mfa_start_returns_token_for_email_mfa` is the same request with SMTP configured, so the
/// pair pins SMTP as the discriminator rather than another `InvalidArgument` on the path. `Start`
/// is the only chance to report it, since `initiate` sends via `send_and_forget`.
#[sqlx::test]
async fn test_mfa_start_rejects_email_when_smtp_not_configured(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;

    // Enable email MFA on the user directly: `setup_user_email_mfa` would also configure SMTP,
    // which is the condition under test.
    user.new_email_secret(&context.pool)
        .await
        .expect("new_email_secret");
    user.enable_email_mfa(&context.pool)
        .await
        .expect("enable_email_mfa");

    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Email as i32,
                posture_data: None,
                selected_methods: Vec::new(),
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_with_message(&response);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "selected MFA method is not available");

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

    // Subscribe before finish so the handler's gateway_tx.send() has a receiver
    let _gateway_rx = context.gateway_tx.subscribe();

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

    // Send a clearly wrong code - use _raw so we can inspect the error response
    let response = send_mfa_finish_raw(&mut context, &token, Some("000000")).await;
    let code = assert_error_response(&response);
    // invalid code → InvalidArgument or Unauthenticated
    assert!(
        matches!(code, Code::InvalidArgument | Code::Unauthenticated),
        "expected InvalidArgument or Unauthenticated, got: {code:?}"
    );

    context.finish().await.expect_server_finished().await;
}

/// Without a business license, OIDC is removed from the flow's available methods, so selecting
/// it is rejected as unsupported by the location.
///
/// This is written as a differential test on purpose. Every rejection on this path returns
/// `InvalidArgument`, so asserting the code alone proves nothing: it passes just as well when
/// the license gate is not enforced at all. The licensed run pins that down - it must fail for
/// a *different* reason (the unconfigured OIDC provider), which it can only do if the gate
/// changed the outcome.
#[sqlx::test]
async fn test_mfa_oidc_start_requires_license(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // External MFA location + OIDC method, no OIDC provider configured
    let network = create_external_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    // email MFA is irrelevant for OIDC path but user still needs to exist
    setup_user_email_mfa(&context.pool, &mut user).await;

    let request = |id: u64| CoreRequest {
        id,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaStart(
            ClientMfaStartRequest {
                location_id: network.id,
                pubkey: device.wireguard_pubkey.clone(),
                #[allow(deprecated)]
                method: MfaMethod::Oidc as i32,
                posture_data: None,
                selected_methods: Vec::new(),
            },
        )),
    };

    // Unlicensed: the license gate filters OIDC out of the first step, so the method is not
    // among those the location offers.
    clear_test_license();
    context.mock_proxy().send_request(request(1));
    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_with_message(&response);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "selected MFA method is not supported by location");

    // Licensed: OIDC survives the filter, so the request gets past the gate and fails further
    // in, on the provider that was never configured.
    set_test_license_business();
    context.mock_proxy().send_request(request(2));
    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_with_message(&response);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "selected MFA method is not available");

    context.finish().await.expect_server_finished().await;
}

/// When a second MFA cycle completes for the same device+location the handler
/// must:
///  - disconnect the first `VpnClientSession` (state → Disconnected),
///  - emit `GatewayCommand::VpnSessionDeauthorized` for the first session, and
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
    let _gw_rx1 = context.gateway_tx.subscribe();

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

    // Subscribe before finish so both VpnSessionDeauthorized and
    // VpnSessionAuthorized have an active receiver.
    let mut gw_rx2 = context.gateway_tx.subscribe();

    let code2 = generate_totp_code(&user);
    let (_, psk2) = send_mfa_finish(&mut context, &token2, Some(&code2)).await;
    assert!(
        !psk2.is_empty(),
        "second MFA cycle must return a non-empty PSK"
    );

    // Receive events from the gateway broadcast channel.  The handler sends
    // VpnSessionDeauthorized (for the old session) and then VpnSessionAuthorized
    // (for the new session) in that order.
    let mut got_disconnected = false;
    let mut got_authorized = false;
    for _ in 0..2 {
        let event = timeout(RECEIVE_TIMEOUT, gw_rx2.recv())
            .await
            .expect("timed out waiting for gateway command after second MFA finish")
            .expect("gateway command channel closed");

        match event {
            GatewayCommand::VpnSessionDeauthorized(loc_id, ref dev) => {
                assert_eq!(loc_id, network.id, "disconnected session location mismatch");
                assert_eq!(dev.id, device.id, "disconnected session device mismatch");
                got_disconnected = true;
            }
            GatewayCommand::VpnSessionAuthorized(loc_id, _, _) => {
                assert_eq!(loc_id, network.id, "authorized session location mismatch");
                got_authorized = true;
            }
            other => panic!("unexpected gateway command: {other:?}"),
        }
    }
    assert!(got_disconnected, "VpnSessionDeauthorized must be emitted");
    assert!(got_authorized, "VpnSessionAuthorized must be emitted");

    // New session must exist in the DB.
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_multi_step_mfa_full_flow(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network(&context.pool).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    setup_user_email_mfa(&context.pool, &mut user).await;

    // Start the TOTP -> Email flow.
    let (_, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::Totp, MfaMethod::Email],
    )
    .await;
    assert!(!token.is_empty());

    // Subscribe to the gateway broadcast before finishing so the collect path's
    // gateway send has a live receiver.
    let mut gateway_rx = context.gateway_tx.subscribe();

    // Step 0 (TOTP) advances without authorizing.
    let totp = generate_totp_code(&user);
    let response = send_mfa_finish_raw(&mut context, &token, Some(&totp)).await;
    let next_step = match &response.payload {
        Some(core_response::Payload::ClientMfaFinish(r)) => match &r.result {
            Some(MfaStepResult {
                outcome: Some(mfa_step_result::Outcome::Advanced(advanced)),
            }) => advanced.next_step,
            _ => panic!("expected Advanced outcome"),
        },
        _ => panic!("expected ClientMfaFinish response"),
    };
    assert_eq!(next_step, 1);
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &context.pool,
            network.id,
            device.id
        )
        .await
        .expect("failed to fetch sessions")
        .is_empty(),
        "no session may be authorized before the final step"
    );

    // Step 1 (Email) completes the flow.
    let step_started = send_mfa_step_start(&mut context, &token, MfaMethod::Email).await;
    assert!(!step_started.step_attempt_id.is_empty());

    let email = user
        .generate_email_mfa_code()
        .expect("email_mfa_secret must be set");
    let response = send_mfa_finish_raw(&mut context, &token, Some(&email)).await;
    let preshared_key = match &response.payload {
        Some(core_response::Payload::ClientMfaFinish(r)) => match &r.result {
            Some(MfaStepResult {
                outcome: Some(mfa_step_result::Outcome::Completed(completed)),
            }) => completed.preshared_key.clone(),
            _ => panic!("expected Completed outcome"),
        },
        Some(core_response::Payload::CoreError(e)) => panic!(
            "second finish got CoreError status={} msg={}",
            e.status_code, e.message
        ),
        _ => panic!("expected ClientMfaFinish response"),
    };
    assert!(!preshared_key.is_empty());

    let sessions = VpnClientSession::get_all_active_device_sessions_in_location(
        &context.pool,
        network.id,
        device.id,
    )
    .await
    .expect("failed to fetch sessions");
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].is_mfa_session);

    // The gateway authorization and the success event are emitted on completion.
    let event = timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
        .await
        .expect("timed out waiting for VpnSessionAuthorized")
        .expect("gateway command channel closed");
    assert!(
        matches!(event, GatewayCommand::VpnSessionAuthorized(..)),
        "expected VpnSessionAuthorized, got: {event:?}"
    );
    expect_bidi_mfa_success(&mut context.bidi_events_rx).await;

    context.finish().await.expect_server_finished().await;
}
