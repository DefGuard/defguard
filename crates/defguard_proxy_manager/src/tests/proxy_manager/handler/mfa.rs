use std::sync::atomic::{AtomicU64, Ordering};

use defguard_common::{
    db::{
        Id,
        models::{
            ThrottleScope, vpn_client_mfa_session::VpnClientMfaSession,
            vpn_client_session::VpnClientSession,
        },
    },
    gateway_event::GatewayCommand,
};
use defguard_core::{
    events::{BidiStreamEventType, DesktopClientMfaEvent},
    grpc::proxy::client_mfa::LEGACY_MOBILE_CLIENT_MESSAGE,
};
use defguard_proto::{
    client_types::{
        ClientMfaFinishRequest, ClientMfaStartRequest, MfaBiometricSignature, MfaCodeCredential,
        MfaFlowApproveRequest, MfaFlowRemoteRequest, MfaFlowStartRequest, MfaFlowStepFinishRequest,
        MfaFlowStepStartRequest, MfaMethod, MfaMobileApprovalProof, MfaStepResult,
        mfa_flow_start_response, mfa_flow_step_finish_request, mfa_step_result, mfa_step_started,
    },
    proxy::{CoreRequest, CoreResponse, core_request, core_response},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::{task, time::timeout};
use tonic::Code;

use super::support::{
    assert_error_response, assert_error_response_details, assert_vpn_session_exists,
    biometric_pub_key, clear_test_license, complete_proxy_handshake, configure_oidc_provider,
    create_external_mfa_network, create_mfa_network, create_multi_step_mfa_network,
    create_multi_step_mfa_network_with_steps, create_network, create_user_with_device,
    expect_bidi_mfa_success, generate_totp_code, link_user_oidc_identity, make_device_info,
    register_biometric_key, send_mfa_finish, send_mfa_finish_raw, send_mfa_start,
    send_token_validation, set_test_license_business, setup_user_email_mfa, setup_user_totp_mfa,
    sign_challenge,
};
use crate::tests::common::{CORE_RESPONSE_TIMEOUT, HandlerTestContext, RECEIVE_TIMEOUT};

const WRONG_REQUEST_ID: u64 = 9991;
async fn send_flow_start(
    context: &mut HandlerTestContext,
    id: u64,
    location_id: Id,
    pubkey: &str,
    selected_methods: &[MfaMethod],
) -> CoreResponse {
    context.mock_proxy().send_request(CoreRequest {
        id,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowStart(MfaFlowStartRequest {
            location_id,
            pubkey: pubkey.to_owned(),
            posture_data: None,
            selected_methods: selected_methods
                .iter()
                .map(|method| *method as i32)
                .collect(),
        })),
    });
    context.mock_proxy_mut().recv_outbound().await
}

async fn assert_unknown_token_response_matches(
    context: &mut HandlerTestContext,
    request_id: u64,
    expected_response: &CoreResponse,
) {
    let (expected_code, expected_message) = assert_error_response_details(expected_response);
    context.mock_proxy().send_request(CoreRequest {
        id: request_id,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: "unknown-mfa-token".to_owned(),
                code: None,
                auth_pub_key: None,
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, request_id);
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, expected_code);
    assert_eq!(message, expected_message);
}

async fn assert_no_active_vpn_session(context: &HandlerTestContext, network_id: Id, device_id: Id) {
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &context.pool,
            network_id,
            device_id
        )
        .await
        .expect("VPN session lookup must succeed")
        .is_empty(),
        "legacy mobile proof must not authorize a VPN session"
    );
}

fn accepted_flow_start(response: CoreResponse) -> (String, String, Option<String>) {
    let Some(core_response::Payload::MfaFlowStart(response)) = response.payload else {
        panic!("expected MfaFlowStart response");
    };
    let Some(mfa_flow_start_response::Outcome::Accepted(accepted)) = response.outcome else {
        panic!("expected accepted flow start response");
    };
    let first_step = accepted
        .first_step
        .expect("flow start must include the first step");
    let challenge = first_step.challenge.map(|challenge| match challenge {
        mfa_step_started::Challenge::Signature(challenge) => challenge.challenge,
        mfa_step_started::Challenge::Fido2(_) => panic!("unexpected FIDO2 challenge"),
    });
    (accepted.token, first_step.step_attempt_id, challenge)
}

static FLOW_REQUEST_ID: AtomicU64 = AtomicU64::new(10_000);

fn next_flow_request_id() -> u64 {
    FLOW_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

async fn send_mfa_start_multi_step(
    context: &mut HandlerTestContext,
    location_id: Id,
    pubkey: &str,
    selected_methods: &[MfaMethod],
) -> (String, String) {
    let (token, attempt_id, _) = accepted_flow_start(
        send_flow_start(
            context,
            next_flow_request_id(),
            location_id,
            pubkey,
            selected_methods,
        )
        .await,
    );
    (attempt_id, token)
}

struct FlowStepStarted {
    step_attempt_id: String,
    challenge: Option<String>,
}

async fn send_mfa_step_start(
    context: &mut HandlerTestContext,
    token: &str,
    method: MfaMethod,
) -> FlowStepStarted {
    context.mock_proxy().send_request(CoreRequest {
        id: next_flow_request_id(),
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowStepStart(
            MfaFlowStepStartRequest {
                token: token.to_owned(),
                method: method as i32,
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    let Some(core_response::Payload::MfaFlowStepStart(response)) = response.payload else {
        panic!("expected MfaFlowStepStart response");
    };
    let started = response
        .started
        .expect("step start must include attempt data");
    let challenge = started.challenge.map(|challenge| match challenge {
        mfa_step_started::Challenge::Signature(challenge) => challenge.challenge,
        mfa_step_started::Challenge::Fido2(_) => panic!("unexpected FIDO2 challenge"),
    });
    FlowStepStarted {
        step_attempt_id: started.step_attempt_id,
        challenge,
    }
}

async fn send_flow_step_finish(
    context: &mut HandlerTestContext,
    token: &str,
    step_attempt_id: &str,
    submission: Option<mfa_flow_step_finish_request::Submission>,
) -> CoreResponse {
    context.mock_proxy().send_request(CoreRequest {
        id: next_flow_request_id(),
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowStepFinish(
            MfaFlowStepFinishRequest {
                token: token.to_owned(),
                step_attempt_id: step_attempt_id.to_owned(),
                submission,
            },
        )),
    });
    context.mock_proxy_mut().recv_outbound().await
}

async fn send_flow_code_finish(
    context: &mut HandlerTestContext,
    token: &str,
    step_attempt_id: &str,
    code: String,
) -> CoreResponse {
    send_flow_step_finish(
        context,
        token,
        step_attempt_id,
        Some(mfa_flow_step_finish_request::Submission::Code(
            MfaCodeCredential { code },
        )),
    )
    .await
}

async fn send_flow_approve(
    context: &mut HandlerTestContext,
    token: &str,
    step_attempt_id: &str,
    signature: &str,
    auth_pub_key: &str,
) -> CoreResponse {
    context.mock_proxy().send_request(CoreRequest {
        id: next_flow_request_id(),
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowApprove(
            MfaFlowApproveRequest {
                token: token.to_owned(),
                step_attempt_id: step_attempt_id.to_owned(),
                proof: Some(MfaMobileApprovalProof {
                    signature: signature.to_owned(),
                    auth_pub_key: auth_pub_key.to_owned(),
                }),
            },
        )),
    });
    context.mock_proxy_mut().recv_outbound().await
}

fn flow_step_result(response: CoreResponse) -> MfaStepResult {
    match response.payload {
        Some(core_response::Payload::MfaFlowStepFinish(response)) => {
            response.result.expect("step finish must include a result")
        }
        Some(core_response::Payload::CoreError(error)) => panic!(
            "step finish failed with status={} msg={}",
            error.status_code, error.message
        ),
        _ => panic!("expected MfaFlowStepFinish response"),
    }
}

fn flow_remote_result(response: CoreResponse) -> MfaStepResult {
    match response.payload {
        Some(core_response::Payload::MfaFlowRemote(response)) => response
            .result
            .expect("remote finish must include a result"),
        Some(core_response::Payload::CoreError(error)) => panic!(
            "remote finish failed with status={} msg={}",
            error.status_code, error.message
        ),
        _ => panic!("expected MfaFlowRemote response"),
    }
}

fn biometric_submission(signature: &str) -> mfa_flow_step_finish_request::Submission {
    mfa_flow_step_finish_request::Submission::Biometric(MfaBiometricSignature {
        signature: signature.to_owned(),
    })
}

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
                method: MfaMethod::Email as i32,
                posture_data: None,
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
                method: MfaMethod::Email as i32,
                posture_data: None,
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
async fn test_mfa_flow_start_dispatches_response(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network =
        create_multi_step_mfa_network_with_steps(&context.pool, vec![vec![MfaMethod::Totp.into()]])
            .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;

    let response = send_flow_start(
        &mut context,
        1,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::Totp],
    )
    .await;
    assert_eq!(response.id, 1);
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::MfaFlowStart(_))
    ));

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_flow_step_start_dispatches_response(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network =
        create_multi_step_mfa_network_with_steps(&context.pool, vec![vec![MfaMethod::Totp.into()]])
            .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;

    let (token, _, _) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::Totp],
        )
        .await,
    );
    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowStepStart(
            MfaFlowStepStartRequest {
                token,
                method: MfaMethod::Totp as i32,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, 2);
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::MfaFlowStepStart(_))
    ));

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_flow_step_finish_dispatches_response(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![vec![MfaMethod::Totp.into()], vec![MfaMethod::Email.into()]],
    )
    .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let _ = setup_user_email_mfa(&context.pool, &mut user).await;

    let (token, step_attempt_id, _) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::Totp, MfaMethod::Email],
        )
        .await,
    );
    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowStepFinish(
            MfaFlowStepFinishRequest {
                token,
                step_attempt_id,
                submission: Some(mfa_flow_step_finish_request::Submission::Code(
                    MfaCodeCredential {
                        code: generate_totp_code(&user),
                    },
                )),
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, 2);
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::MfaFlowStepFinish(_))
    ));

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_flow_remote_dispatches_response(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![
            vec![MfaMethod::MobileApprove.into()],
            vec![MfaMethod::Totp.into()],
        ],
    )
    .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);

    let (token, step_attempt_id, challenge) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::MobileApprove, MfaMethod::Totp],
        )
        .await,
    );
    let challenge = challenge.expect("mobile approval must include a signature challenge");

    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowApprove(
            MfaFlowApproveRequest {
                token: token.clone(),
                step_attempt_id: step_attempt_id.clone(),
                proof: Some(MfaMobileApprovalProof {
                    signature: sign_challenge(&signing_key, &challenge),
                    auth_pub_key,
                }),
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, 2);
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::Empty(()))
    ));

    context.mock_proxy().send_request(CoreRequest {
        id: 3,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowRemote(MfaFlowRemoteRequest {
            token,
            step_attempt_id,
        })),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, 3);
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::MfaFlowRemote(_))
    ));

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_mfa_flow_approve_dispatches_empty_response(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![vec![MfaMethod::MobileApprove.into()]],
    )
    .await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);

    let (token, step_attempt_id, challenge) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::MobileApprove],
        )
        .await,
    );
    let challenge = challenge.expect("mobile approval must include a signature challenge");

    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowApprove(
            MfaFlowApproveRequest {
                token,
                step_attempt_id,
                proof: Some(MfaMobileApprovalProof {
                    signature: sign_challenge(&signing_key, &challenge),
                    auth_pub_key,
                }),
            },
        )),
    });

    let response = timeout(
        CORE_RESPONSE_TIMEOUT,
        context.mock_proxy_mut().recv_outbound(),
    )
    .await
    .expect("MfaFlowApprove must return a CoreResponse");
    assert_eq!(response.id, 2);
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::Empty(()))
    ));

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_mfa_finish_rejects_valid_multi_step_mobile_proof(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![vec![MfaMethod::MobileApprove.into()]],
    )
    .await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);
    let (token, step_attempt_id, challenge) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::MobileApprove],
        )
        .await,
    );
    let challenge = challenge.expect("mobile approval must include a signature challenge");
    let mut gateway_rx = context.take_gateway_rx();

    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowRemote(MfaFlowRemoteRequest {
            token: token.clone(),
            step_attempt_id: step_attempt_id.clone(),
        })),
    });
    context.mock_proxy().send_request(CoreRequest {
        id: 3,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(sign_challenge(&signing_key, &challenge)),
                auth_pub_key: Some(auth_pub_key.clone()),
            },
        )),
    });

    let first = timeout(
        CORE_RESPONSE_TIMEOUT,
        context.mock_proxy_mut().recv_outbound(),
    )
    .await
    .expect("timed out waiting for first MFA response");
    let second = timeout(
        CORE_RESPONSE_TIMEOUT,
        context.mock_proxy_mut().recv_outbound(),
    )
    .await
    .expect("timed out waiting for second MFA response");
    let (remote, phone) = if first.id == 2 {
        (first, second)
    } else {
        (second, first)
    };
    assert_eq!(remote.id, 2);
    assert_eq!(phone.id, 3);

    let (remote_code, remote_message) = assert_error_response_details(&remote);
    let (phone_code, phone_message) = assert_error_response_details(&phone);
    assert_eq!(remote_code, Code::FailedPrecondition);
    assert_eq!(remote_message, LEGACY_MOBILE_CLIENT_MESSAGE);
    assert_eq!(phone_code, Code::FailedPrecondition);
    assert_eq!(phone_message, LEGACY_MOBILE_CLIENT_MESSAGE);

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("MFA session lookup must succeed")
        .expect("gated MFA session must remain active");
    assert_eq!(session.failed_attempts, 0);
    let attempt = session
        .ephemeral_state
        .expect("mobile attempt must remain active")
        .0;
    assert_eq!(attempt.step_attempt_id, step_attempt_id);
    assert!(!attempt.mobile_approved);
    assert_no_active_vpn_session(&context, network.id, device.id).await;
    assert!(gateway_rx.try_recv().is_err());
    assert!(context.bidi_events_rx.try_recv().is_err());

    let signature = sign_challenge(&signing_key, &challenge);
    let approval = send_flow_approve(
        &mut context,
        &token,
        &step_attempt_id,
        &signature,
        &auth_pub_key,
    )
    .await;
    assert!(matches!(
        approval.payload,
        Some(core_response::Payload::Empty(()))
    ));
    context.mock_proxy().send_request(CoreRequest {
        id: 4,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowRemote(MfaFlowRemoteRequest {
            token: token.clone(),
            step_attempt_id,
        })),
    });
    let resumed = timeout(
        CORE_RESPONSE_TIMEOUT,
        context.mock_proxy_mut().recv_outbound(),
    )
    .await
    .expect("timed out waiting for 2.2 remote MFA completion");
    assert_eq!(resumed.id, 4);
    let Some(mfa_step_result::Outcome::Completed(completed)) = flow_remote_result(resumed).outcome
    else {
        panic!("2.2 mobile approval must complete the flow");
    };
    assert!(!completed.preshared_key.is_empty());
    let session = assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert_eq!(
        session.preshared_key.as_ref(),
        Some(&completed.preshared_key)
    );
    assert!(matches!(
        timeout(RECEIVE_TIMEOUT, gateway_rx.recv()).await,
        Ok(Some(GatewayCommand::VpnSessionAuthorized(location_id, _, _))) if location_id == network.id
    ));

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_mfa_finish_rejects_valid_proof_without_waiter_on_two_step_flow(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![
            vec![MfaMethod::MobileApprove.into()],
            vec![MfaMethod::Totp.into()],
        ],
    )
    .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);
    let (token, step_attempt_id, challenge) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::MobileApprove, MfaMethod::Totp],
        )
        .await,
    );
    let challenge = challenge.expect("mobile approval must include a signature challenge");
    let mut gateway_rx = context.take_gateway_rx();

    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(sign_challenge(&signing_key, &challenge)),
                auth_pub_key: Some(auth_pub_key),
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, 2);
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, Code::FailedPrecondition);
    assert_eq!(message, LEGACY_MOBILE_CLIENT_MESSAGE);

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("MFA session lookup must succeed")
        .expect("gated MFA session must remain active");
    assert_eq!(session.failed_attempts, 0);
    let attempt = session
        .ephemeral_state
        .expect("mobile attempt must remain active")
        .0;
    assert_eq!(attempt.step_attempt_id, step_attempt_id);
    assert!(!attempt.mobile_approved);
    assert_no_active_vpn_session(&context, network.id, device.id).await;
    assert!(gateway_rx.try_recv().is_err());
    assert!(context.bidi_events_rx.try_recv().is_err());

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_mfa_finish_rejects_previous_mobile_attempt_challenge(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![
            vec![MfaMethod::MobileApprove.into()],
            vec![MfaMethod::MobileApprove.into()],
        ],
    )
    .await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);
    let (token, first_attempt_id, first_challenge) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::MobileApprove, MfaMethod::MobileApprove],
        )
        .await,
    );
    let first_challenge = first_challenge.expect("first approval must include a challenge");

    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowApprove(
            MfaFlowApproveRequest {
                token: token.clone(),
                step_attempt_id: first_attempt_id.clone(),
                proof: Some(MfaMobileApprovalProof {
                    signature: sign_challenge(&signing_key, &first_challenge),
                    auth_pub_key: auth_pub_key.clone(),
                }),
            },
        )),
    });
    assert!(matches!(
        context.mock_proxy_mut().recv_outbound().await.payload,
        Some(core_response::Payload::Empty(()))
    ));

    context.mock_proxy().send_request(CoreRequest {
        id: 3,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowStepFinish(
            MfaFlowStepFinishRequest {
                token: token.clone(),
                step_attempt_id: first_attempt_id.clone(),
                submission: None,
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, 3);
    let Some(core_response::Payload::MfaFlowStepFinish(response)) = response.payload else {
        panic!("expected step finish response");
    };
    assert!(matches!(
        response.result.and_then(|result| result.outcome),
        Some(mfa_step_result::Outcome::Advanced(_))
    ));

    context.mock_proxy().send_request(CoreRequest {
        id: 4,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowStepStart(
            MfaFlowStepStartRequest {
                token: token.clone(),
                method: MfaMethod::MobileApprove as i32,
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    let Some(core_response::Payload::MfaFlowStepStart(response)) = response.payload else {
        panic!("expected second MobileApprove attempt");
    };
    let started = response
        .started
        .expect("step start must return the attempt");
    let current_attempt_id = started.step_attempt_id.clone();
    assert_ne!(current_attempt_id, first_attempt_id);
    let Some(mfa_step_started::Challenge::Signature(current_challenge)) = started.challenge else {
        panic!("second MobileApprove attempt must include a signature challenge");
    };
    assert_ne!(current_challenge.challenge, first_challenge);
    let mut gateway_rx = context.take_gateway_rx();

    context.mock_proxy().send_request(CoreRequest {
        id: 5,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(sign_challenge(&signing_key, &first_challenge)),
                auth_pub_key: Some(auth_pub_key),
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(response.id, 5);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "login session not found");

    assert_unknown_token_response_matches(&mut context, 6, &response).await;

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("MFA session lookup must succeed")
        .expect("stale proof must leave the session active");
    assert_eq!(session.failed_attempts, 0);
    let current = session
        .ephemeral_state
        .expect("current mobile attempt must remain active")
        .0;
    assert_eq!(current.step_attempt_id, current_attempt_id);
    assert!(!current.mobile_approved);
    assert_no_active_vpn_session(&context, network.id, device.id).await;
    assert!(gateway_rx.try_recv().is_err());

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_mfa_finish_hides_non_mobile_multi_step_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![vec![MfaMethod::Totp.into()], vec![MfaMethod::Totp.into()]],
    )
    .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let (token, _, _) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::Totp, MfaMethod::Totp],
        )
        .await,
    );

    let unknown_token = "unknown-mfa-token";
    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: unknown_token.to_owned(),
                code: None,
                auth_pub_key: None,
            },
        )),
    });
    let unknown_response = context.mock_proxy_mut().recv_outbound().await;
    context.mock_proxy().send_request(CoreRequest {
        id: 3,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token,
                code: None,
                auth_pub_key: None,
            },
        )),
    });
    let flow_response = context.mock_proxy_mut().recv_outbound().await;
    let (unknown_code, unknown_message) = assert_error_response_details(&unknown_response);
    let (flow_code, flow_message) = assert_error_response_details(&flow_response);
    assert_eq!(unknown_response.id, 2);
    assert_eq!(flow_response.id, 3);
    assert_eq!(unknown_code, Code::InvalidArgument);
    assert_eq!(unknown_message, "login session not found");
    assert_eq!(flow_code, unknown_code);
    assert_eq!(flow_message, unknown_message);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_legacy_mfa_finish_hides_bad_mobile_signature(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_test_license_business();

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![vec![MfaMethod::MobileApprove.into()]],
    )
    .await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);
    let (token, step_attempt_id, challenge) = accepted_flow_start(
        send_flow_start(
            &mut context,
            1,
            network.id,
            &device.wireguard_pubkey,
            &[MfaMethod::MobileApprove],
        )
        .await,
    );
    let challenge = challenge.expect("MobileApprove must include a challenge");
    let invalid_signature =
        sign_challenge(&signing_key, &format!("{challenge} is not the challenge"));
    let mut gateway_rx = context.take_gateway_rx();

    context.mock_proxy().send_request(CoreRequest {
        id: 2,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaFinish(
            ClientMfaFinishRequest {
                token: token.clone(),
                code: Some(invalid_signature.clone()),
                auth_pub_key: Some(auth_pub_key.clone()),
            },
        )),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(response.id, 2);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "login session not found");

    assert_unknown_token_response_matches(&mut context, 3, &response).await;

    let limit = u64::try_from(ThrottleScope::VpnMfaCode.limit())
        .expect("MFA code throttle limit must be positive");
    let last_request_id = 3 + limit;
    let mut last_response = response;
    for request_id in 4..=last_request_id {
        context.mock_proxy().send_request(CoreRequest {
            id: request_id,
            device_info: Some(make_device_info()),
            payload: Some(core_request::Payload::ClientMfaFinish(
                ClientMfaFinishRequest {
                    token: token.clone(),
                    code: Some(invalid_signature.clone()),
                    auth_pub_key: Some(auth_pub_key.clone()),
                },
            )),
        });
        last_response = context.mock_proxy_mut().recv_outbound().await;
        let (retry_code, retry_message) = assert_error_response_details(&last_response);
        assert_eq!(retry_code, Code::InvalidArgument);
        assert_eq!(retry_message, "login session not found");
    }
    assert_unknown_token_response_matches(&mut context, last_request_id + 1, &last_response).await;

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("MFA session lookup must succeed")
        .expect("bad signature must leave the session active");
    assert_eq!(session.failed_attempts, 0);
    let attempt = session
        .ephemeral_state
        .expect("mobile attempt must remain active")
        .0;
    assert_eq!(attempt.step_attempt_id, step_attempt_id);
    assert!(!attempt.mobile_approved);
    assert_no_active_vpn_session(&context, network.id, device.id).await;
    assert!(gateway_rx.try_recv().is_err());
    assert!(context.bidi_events_rx.try_recv().is_err());

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
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, Code::Unauthenticated);
    assert_eq!(message, "unauthorized");

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
                method: MfaMethod::Email as i32,
                posture_data: None,
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
                method: MfaMethod::Email as i32,
                posture_data: None,
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
                method: MfaMethod::Email as i32,
                posture_data: None,
            },
        )),
    });

    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_details(&response);
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
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, Code::Unauthenticated);
    assert_eq!(message, "unauthorized");

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
                method: MfaMethod::Oidc as i32,
                posture_data: None,
            },
        )),
    };

    // Unlicensed: the license gate filters OIDC out of the first step, so the method is not
    // among those the location offers.
    clear_test_license();
    context.mock_proxy().send_request(request(1));
    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_details(&response);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "selected MFA method is not supported by location");

    // Licensed: OIDC survives the filter, so the request gets past the gate and fails further
    // in, on the provider that was never configured.
    set_test_license_business();
    context.mock_proxy().send_request(request(2));
    let response = context.mock_proxy_mut().recv_outbound().await;
    let (code, message) = assert_error_response_details(&response);
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
    let mut gw_rx2 = context.take_gateway_rx();

    let code2 = generate_totp_code(&user);
    let (_, psk2) = send_mfa_finish(&mut context, &token2, Some(&code2)).await;
    assert!(
        !psk2.is_empty(),
        "second MFA cycle must return a non-empty PSK"
    );

    // Receive events from the gateway channel.  The handler sends
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
    let (first_attempt_id, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::Totp, MfaMethod::Email],
    )
    .await;
    assert_ne!(token, "");

    // Subscribe to the gateway channel before finishing so the collect path's
    // gateway send has a live receiver.
    let mut gateway_rx = context.take_gateway_rx();

    // Step 0 (TOTP) advances without authorizing.
    let totp = generate_totp_code(&user);
    let response = send_flow_step_finish(
        &mut context,
        &token,
        &first_attempt_id,
        Some(mfa_flow_step_finish_request::Submission::Code(
            MfaCodeCredential { code: totp },
        )),
    )
    .await;
    let next_step = match flow_step_result(response).outcome {
        Some(mfa_step_result::Outcome::Advanced(advanced)) => advanced.next_step,
        _ => panic!("expected Advanced outcome"),
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
    assert_ne!(step_started.step_attempt_id, "");

    let email = user
        .generate_email_mfa_code()
        .expect("email_mfa_secret must be set");
    let response = send_flow_step_finish(
        &mut context,
        &token,
        &step_started.step_attempt_id,
        Some(mfa_flow_step_finish_request::Submission::Code(
            MfaCodeCredential { code: email },
        )),
    )
    .await;
    let preshared_key = match flow_step_result(response).outcome {
        Some(mfa_step_result::Outcome::Completed(completed)) => completed.preshared_key,
        _ => panic!("expected Completed outcome"),
    };
    assert_ne!(preshared_key, "");

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

#[sqlx::test]
async fn test_mfa_flow_oidc_awaits_external_completion(
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
    let (_, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::Oidc],
    )
    .await;
    let started = send_mfa_step_start(&mut context, &token, MfaMethod::Oidc).await;
    let attempt_id = started.step_attempt_id;
    let mut gateway_rx = context.take_gateway_rx();

    let stale_response =
        send_flow_step_finish(&mut context, &token, "superseded-attempt", None).await;
    let (code, message) = assert_error_response_details(&stale_response);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "stale MFA attempt");

    let session_before = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("failed to load OIDC MFA session")
        .expect("OIDC MFA session must remain active");
    let response = send_flow_step_finish(&mut context, &token, &attempt_id, None).await;
    assert!(matches!(
        flow_step_result(response).outcome,
        Some(mfa_step_result::Outcome::AwaitingExternal(_))
    ));
    assert!(
        gateway_rx.try_recv().is_err(),
        "awaiting must not authorize"
    );
    assert!(
        context.bidi_events_rx.try_recv().is_err(),
        "awaiting must not audit a failure"
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
        "awaiting must not create a VPN session"
    );

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("failed to reload OIDC MFA session")
        .expect("OIDC MFA session must remain active");
    assert_eq!(session.failed_attempts, session_before.failed_attempts);
    assert_eq!(session.expires_at, session_before.expires_at);

    let retried = send_mfa_step_start(&mut context, &token, MfaMethod::Oidc).await;
    assert_ne!(retried.step_attempt_id, attempt_id);
    let attempt_id = retried.step_attempt_id;

    let mut conn = context
        .pool
        .acquire()
        .await
        .expect("failed to acquire connection");
    assert!(
        session
            .mark_oidc_completed(&mut conn, &attempt_id)
            .await
            .expect("failed to mark OIDC MFA complete")
    );

    let response = send_flow_step_finish(&mut context, &token, &attempt_id, None).await;
    let preshared_key = match flow_step_result(response).outcome {
        Some(mfa_step_result::Outcome::Completed(completed)) => completed.preshared_key,
        _ => panic!("expected Completed outcome"),
    };
    assert_ne!(preshared_key, "");
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
async fn test_new_protocol_mobile_approve_marks_and_collects_by_poll(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![vec![MfaMethod::MobileApprove.into()]],
    )
    .await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);

    let (_, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::MobileApprove],
    )
    .await;
    let started = send_mfa_step_start(&mut context, &token, MfaMethod::MobileApprove).await;
    let attempt_id = started.step_attempt_id;
    let challenge = started
        .challenge
        .expect("mobile approve StepStart must return a challenge");
    let mut gateway_rx = context.take_gateway_rx();

    let response = send_flow_step_finish(&mut context, &token, &attempt_id, None).await;
    assert!(matches!(
        flow_step_result(response).outcome,
        Some(mfa_step_result::Outcome::AwaitingExternal(_))
    ));
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &context.pool,
            network.id,
            device.id
        )
        .await
        .expect("failed to query authorized VPN sessions")
        .is_empty(),
        "pending approval must not authorize"
    );

    let signature = sign_challenge(&signing_key, &challenge);
    let session_before_stale =
        VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
            .await
            .expect("failed to load mobile approval session")
            .expect("mobile approval session must remain active");
    let stale_response = send_flow_approve(
        &mut context,
        &token,
        "stale-attempt",
        &signature,
        &auth_pub_key,
    )
    .await;
    let (code, message) = assert_error_response_details(&stale_response);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "stale MFA attempt");
    let session_after_stale =
        VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
            .await
            .expect("failed to reload mobile approval session")
            .expect("mobile approval session must remain active");
    assert_eq!(
        session_after_stale.failed_attempts,
        session_before_stale.failed_attempts
    );
    assert_eq!(
        session_after_stale.expires_at,
        session_before_stale.expires_at
    );
    assert!(
        !session_after_stale
            .ephemeral_state
            .expect("mobile approval attempt must remain active")
            .0
            .mobile_approved
    );
    assert!(
        gateway_rx.try_recv().is_err(),
        "stale approval must not authorize"
    );
    assert!(
        context.bidi_events_rx.try_recv().is_err(),
        "stale approval must not emit an event"
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
        "stale approval must not authorize"
    );

    let response =
        send_flow_approve(&mut context, &token, &attempt_id, &signature, &auth_pub_key).await;
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::Empty(()))
    ));
    assert!(gateway_rx.try_recv().is_err(), "mark must not authorize");
    assert!(
        context.bidi_events_rx.try_recv().is_err(),
        "mark must not emit an event"
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
        "mark must not authorize"
    );

    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("failed to reload mobile approval session")
        .expect("mobile approval session must remain active after mark");
    assert!(
        session
            .ephemeral_state
            .expect("mobile approval attempt must remain active")
            .0
            .mobile_approved
    );
    assert_eq!(session.failed_attempts, 0);

    let response = send_flow_step_finish(&mut context, &token, &attempt_id, None).await;
    let preshared_key = match flow_step_result(response).outcome {
        Some(mfa_step_result::Outcome::Completed(completed)) => completed.preshared_key,
        _ => panic!("expected Completed response"),
    };
    assert_ne!(preshared_key, "");
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(matches!(
        timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
            .await
            .expect("timed out waiting for gateway authorization")
            .expect("gateway command channel closed"),
        GatewayCommand::VpnSessionAuthorized(location_id, _, _) if location_id == network.id
    ));
    let event = context
        .bidi_events_rx
        .try_recv()
        .expect("expected mobile-approve success event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Success {
                location,
                mobile_auth_device_name,
                ..
            } => {
                assert_eq!(location.id, network.id);
                assert_eq!(mobile_auth_device_name, Some(device.name.clone()));
            }
            other => panic!("expected MFA success event, got: {other:?}"),
        },
        other => panic!("expected desktop MFA event, got: {other:?}"),
    }

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
#[allow(deprecated)]
async fn test_new_protocol_mobile_approve_advances_non_final_step(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![
            vec![MfaMethod::MobileApprove.into()],
            vec![MfaMethod::Totp.into()],
        ],
    )
    .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);

    let (_, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::MobileApprove, MfaMethod::Totp],
    )
    .await;
    let started = send_mfa_step_start(&mut context, &token, MfaMethod::MobileApprove).await;
    let attempt_id = started.step_attempt_id;
    let challenge = started
        .challenge
        .expect("mobile approve StepStart must return a challenge");
    let mut gateway_rx = context.take_gateway_rx();

    let signature = sign_challenge(&signing_key, &challenge);
    let response =
        send_flow_approve(&mut context, &token, &attempt_id, &signature, &auth_pub_key).await;
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::Empty(()))
    ));
    assert!(gateway_rx.try_recv().is_err(), "mark must not authorize");
    assert!(
        context.bidi_events_rx.try_recv().is_err(),
        "mark must not emit an event"
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
        "mark must not authorize"
    );

    assert!(matches!(
        flow_step_result(send_flow_step_finish(&mut context, &token, &attempt_id, None).await).outcome,
        Some(mfa_step_result::Outcome::Advanced(advanced)) if advanced.next_step == 1
    ));
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &context.pool,
            network.id,
            device.id
        )
        .await
        .expect("failed to query authorized VPN sessions")
        .is_empty(),
        "non-final approval must not authorize"
    );
    assert!(
        gateway_rx.try_recv().is_err(),
        "non-final poll must not authorize"
    );
    assert!(
        context.bidi_events_rx.try_recv().is_err(),
        "non-final poll must not emit an event"
    );
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("failed to load advanced mobile approval session")
        .expect("session must remain active after a non-final step");
    assert_eq!(session.current_step, 1);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
#[allow(deprecated)]
async fn test_new_protocol_mobile_approve_non_final_device_name_reaches_success_event(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![
            vec![MfaMethod::MobileApprove.into()],
            vec![MfaMethod::Totp.into()],
        ],
    )
    .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);

    let (_, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::MobileApprove, MfaMethod::Totp],
    )
    .await;
    let started = send_mfa_step_start(&mut context, &token, MfaMethod::MobileApprove).await;
    let attempt_id = started.step_attempt_id;
    let challenge = started
        .challenge
        .expect("mobile approve StepStart must return a challenge");
    let mut gateway_rx = context.take_gateway_rx();

    let signature = sign_challenge(&signing_key, &challenge);
    let response =
        send_flow_approve(&mut context, &token, &attempt_id, &signature, &auth_pub_key).await;
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::Empty(()))
    ));
    assert!(gateway_rx.try_recv().is_err(), "mark must not authorize");
    assert!(
        context.bidi_events_rx.try_recv().is_err(),
        "mark must not emit an event"
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
        "mark must not authorize"
    );

    assert!(matches!(
        flow_step_result(send_flow_step_finish(&mut context, &token, &attempt_id, None).await).outcome,
        Some(mfa_step_result::Outcome::Advanced(advanced)) if advanced.next_step == 1
    ));
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &context.pool,
            network.id,
            device.id
        )
        .await
        .expect("failed to query authorized VPN sessions")
        .is_empty(),
        "non-final approval must not authorize"
    );
    assert!(
        gateway_rx.try_recv().is_err(),
        "non-final poll must not authorize"
    );
    assert!(
        context.bidi_events_rx.try_recv().is_err(),
        "non-final poll must not emit an event"
    );
    let session = VpnClientMfaSession::<Id>::find_active_by_token(&context.pool, &token)
        .await
        .expect("failed to load advanced mobile approval session")
        .expect("session must remain active after a non-final step");
    assert_eq!(session.current_step, 1);

    let totp_attempt = send_mfa_step_start(&mut context, &token, MfaMethod::Totp).await;
    let response = send_flow_code_finish(
        &mut context,
        &token,
        &totp_attempt.step_attempt_id,
        generate_totp_code(&user),
    )
    .await;
    assert!(matches!(
        flow_step_result(response).outcome,
        Some(mfa_step_result::Outcome::Completed(completed))
            if !completed.preshared_key.is_empty()
    ));
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(matches!(
        timeout(RECEIVE_TIMEOUT, gateway_rx.recv())
            .await
            .expect("timed out waiting for gateway authorization")
            .expect("gateway command channel closed"),
        GatewayCommand::VpnSessionAuthorized(location_id, _, _) if location_id == network.id
    ));
    let event = context
        .bidi_events_rx
        .try_recv()
        .expect("expected mobile-approve success event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Success {
                location,
                attribution,
                mobile_auth_device_name,
                ..
            } => {
                assert_eq!(location.id, network.id);
                assert_eq!(mobile_auth_device_name, Some(device.name.clone()));
                assert_eq!(
                    attribution.snapshot.steps[0].mobile_auth_device_name,
                    Some(device.name.clone())
                );
            }
            other => panic!("expected MFA success event, got: {other:?}"),
        },
        other => panic!("expected desktop MFA event, got: {other:?}"),
    }

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
#[allow(deprecated)]
async fn test_parked_mobile_approval_completes_final_step(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![vec![MfaMethod::MobileApprove.into()]],
    )
    .await;
    let (_user, device) = create_user_with_device(&context.pool).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);
    let (_, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::MobileApprove],
    )
    .await;
    let started = send_mfa_step_start(&mut context, &token, MfaMethod::MobileApprove).await;
    let attempt_id = started.step_attempt_id;
    let challenge = started
        .challenge
        .expect("mobile approval needs a challenge");
    let signature = sign_challenge(&signing_key, &challenge);
    let mut gateway_rx = context.take_gateway_rx();

    context.mock_proxy().send_request(CoreRequest {
        id: 7001,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowRemote(MfaFlowRemoteRequest {
            token: token.clone(),
            step_attempt_id: attempt_id.clone(),
        })),
    });
    task::yield_now().await;

    let stale_response = send_flow_approve(
        &mut context,
        &token,
        "stale-attempt",
        &signature,
        &auth_pub_key,
    )
    .await;
    let (code, message) = assert_error_response_details(&stale_response);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "stale MFA attempt");

    context.mock_proxy().send_request(CoreRequest {
        id: 7003,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowApprove(
            MfaFlowApproveRequest {
                token: token.clone(),
                step_attempt_id: attempt_id,
                proof: Some(MfaMobileApprovalProof {
                    signature,
                    auth_pub_key,
                }),
            },
        )),
    });

    let first = timeout(
        CORE_RESPONSE_TIMEOUT,
        context.mock_proxy_mut().recv_outbound(),
    )
    .await
    .expect("timed out waiting for mobile approval response");
    let second = timeout(
        CORE_RESPONSE_TIMEOUT,
        context.mock_proxy_mut().recv_outbound(),
    )
    .await
    .expect("timed out waiting for parked remote response");
    let mut got_approve = false;
    let mut parked_key = None;
    for response in [first, second] {
        if response.id == 7003 {
            assert!(matches!(
                response.payload,
                Some(core_response::Payload::Empty(()))
            ));
            got_approve = true;
        } else {
            assert_eq!(response.id, 7001);
            let result = flow_remote_result(response);
            let Some(mfa_step_result::Outcome::Completed(completed)) = result.outcome else {
                panic!("expected completed parked result");
            };
            assert!(!completed.preshared_key.is_empty());
            parked_key = Some(completed.preshared_key);
        }
    }
    assert!(got_approve, "missing MfaFlowApprove response");
    assert!(
        !parked_key
            .expect("parked response must contain a key")
            .is_empty(),
        "parked completion must return a key"
    );
    assert_vpn_session_exists(&context.pool, network.id, device.id).await;
    assert!(matches!(
        timeout(RECEIVE_TIMEOUT, gateway_rx.recv()).await,
        Ok(Some(GatewayCommand::VpnSessionAuthorized(id, _, _))) if id == network.id
    ));
    let event = context
        .bidi_events_rx
        .try_recv()
        .expect("expected mobile-approve success event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Success {
                location,
                mobile_auth_device_name,
                ..
            } => {
                assert_eq!(location.id, network.id);
                assert_eq!(mobile_auth_device_name, Some(device.name.clone()));
            }
            other => panic!("expected MFA success event, got: {other:?}"),
        },
        other => panic!("expected desktop MFA event, got: {other:?}"),
    }
    context.finish().await.expect_server_finished().await;
}
#[sqlx::test]
#[allow(deprecated)]
async fn test_parked_mobile_approval_advances_non_final_step(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![
            vec![MfaMethod::MobileApprove.into()],
            vec![MfaMethod::Totp.into()],
        ],
    )
    .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;
    let auth_pub_key = biometric_pub_key(&signing_key);
    let (_, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::MobileApprove, MfaMethod::Totp],
    )
    .await;
    let started = send_mfa_step_start(&mut context, &token, MfaMethod::MobileApprove).await;
    let attempt_id = started.step_attempt_id;
    let signature = sign_challenge(
        &signing_key,
        &started
            .challenge
            .expect("mobile approval needs a challenge"),
    );
    let mut gateway_rx = context.take_gateway_rx();

    context.mock_proxy().send_request(CoreRequest {
        id: 7101,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowRemote(MfaFlowRemoteRequest {
            token: token.clone(),
            step_attempt_id: attempt_id.clone(),
        })),
    });
    task::yield_now().await;
    context.mock_proxy().send_request(CoreRequest {
        id: 7102,
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::MfaFlowApprove(
            MfaFlowApproveRequest {
                token: token.clone(),
                step_attempt_id: attempt_id,
                proof: Some(MfaMobileApprovalProof {
                    signature,
                    auth_pub_key,
                }),
            },
        )),
    });

    let first = timeout(
        CORE_RESPONSE_TIMEOUT,
        context.mock_proxy_mut().recv_outbound(),
    )
    .await
    .expect("timed out waiting for mobile approval response");
    let second = timeout(
        CORE_RESPONSE_TIMEOUT,
        context.mock_proxy_mut().recv_outbound(),
    )
    .await
    .expect("timed out waiting for parked remote response");
    let mut got_approve = false;
    let mut got_advanced = false;
    for response in [first, second] {
        if response.id == 7102 {
            assert!(matches!(
                response.payload,
                Some(core_response::Payload::Empty(()))
            ));
            got_approve = true;
        } else {
            assert_eq!(response.id, 7101);
            assert!(matches!(
                flow_remote_result(response).outcome,
                Some(mfa_step_result::Outcome::Advanced(advanced)) if advanced.next_step == 1
            ));
            got_advanced = true;
        }
    }
    assert!(got_approve, "missing MfaFlowApprove response");
    assert!(got_advanced, "missing advanced MfaFlowRemote response");
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &context.pool,
            network.id,
            device.id
        )
        .await
        .expect("query sessions")
        .is_empty()
    );
    assert!(gateway_rx.try_recv().is_err());
    assert!(context.bidi_events_rx.try_recv().is_err());
    context.finish().await.expect_server_finished().await;
}
#[sqlx::test]
#[allow(deprecated)]
async fn test_multi_step_biometric_flow_completes(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let network = create_multi_step_mfa_network_with_steps(
        &context.pool,
        vec![
            vec![MfaMethod::Totp.into()],
            vec![MfaMethod::Biometric.into()],
        ],
    )
    .await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let signing_key = register_biometric_key(&context.pool, device.id).await;

    let (first_attempt_id, token) = send_mfa_start_multi_step(
        &mut context,
        network.id,
        &device.wireguard_pubkey,
        &[MfaMethod::Totp, MfaMethod::Biometric],
    )
    .await;
    let mut gateway_rx = context.take_gateway_rx();

    let response = send_flow_code_finish(
        &mut context,
        &token,
        &first_attempt_id,
        generate_totp_code(&user),
    )
    .await;
    assert!(matches!(
        flow_step_result(response).outcome,
        Some(mfa_step_result::Outcome::Advanced(advanced)) if advanced.next_step == 1
    ));
    assert!(
        VpnClientSession::get_all_active_device_sessions_in_location(
            &context.pool,
            network.id,
            device.id
        )
        .await
        .expect("failed to query authorized VPN sessions")
        .is_empty(),
        "no VPN session may exist before the final biometric step"
    );
    assert!(
        gateway_rx.try_recv().is_err(),
        "no gateway authorization may occur before the final step"
    );

    let step_started = send_mfa_step_start(&mut context, &token, MfaMethod::Biometric).await;
    let challenge = step_started
        .challenge
        .expect("biometric StepStart must return a challenge");
    assert!(!challenge.is_empty());
    let invalid_response = send_flow_step_finish(
        &mut context,
        &token,
        &step_started.step_attempt_id,
        Some(mfa_flow_step_finish_request::Submission::Biometric(
            MfaBiometricSignature {
                signature: "invalid-signature".to_owned(),
            },
        )),
    )
    .await;
    let (code, message) = assert_error_response_details(&invalid_response);
    assert_eq!(code, Code::Unauthenticated);
    assert_eq!(message, "unauthorized");
    let event = context
        .bidi_events_rx
        .try_recv()
        .expect("invalid signature must emit a failure audit event");
    match event.event {
        BidiStreamEventType::DesktopClientMfa(event) => match *event {
            DesktopClientMfaEvent::Failed {
                method, message, ..
            } => {
                assert_eq!(method, MfaMethod::Biometric);
                assert_eq!(message, "Signed challenge rejected");
            }
            other => panic!("expected failed MFA audit event, got: {other:?}"),
        },
        other => panic!("expected desktop MFA audit event, got: {other:?}"),
    }

    let signature = sign_challenge(&signing_key, &challenge);
    let response = send_flow_step_finish(
        &mut context,
        &token,
        &step_started.step_attempt_id,
        Some(biometric_submission(&signature)),
    )
    .await;
    let preshared_key = match flow_step_result(response).outcome {
        Some(mfa_step_result::Outcome::Completed(completed)) => completed.preshared_key,
        _ => panic!("expected completed biometric response"),
    };
    assert!(!preshared_key.is_empty());
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
