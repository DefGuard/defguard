use chrono::{TimeDelta, Utc};
use defguard_common::db::{
    Id,
    models::{
        Device, Settings, User, device_identity::DeviceAttestationStatus,
        polling_token::PollingToken,
    },
};
use defguard_core::{
    db::models::enrollment::{
        ENROLLMENT_TOKEN_TYPE, MFA_CONFIG_TOKEN_TYPE, PASSWORD_RESET_TOKEN_TYPE, Token,
    },
    enterprise::db::models::device_identity::{
        DeviceAttestationState, DeviceIdentity, DeviceIdentityBackend,
    },
};
use defguard_proto::{
    client_types::{AttestationFinishRequest, AttestationStartRequest, attestation_start_request},
    device_identity::{
        AttestationChallengeSolution, AttestationIdentity, TpmAttestationIdentity,
        TpmAttestationResponse, attestation_challenge_solution, attestation_identity,
    },
    proxy::{CoreRequest, CoreResponse, core_request, core_response},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tonic::Code;

use super::support::{
    STRONG_PASSWORD, assert_error_response, complete_proxy_handshake, create_enrollment_token,
    create_user_with_device, make_device_info, send_activate_user, start_enrollment_session,
};
use crate::tests::common::HandlerTestContext;

fn attestation_requests(token: &str, device_id: Id) -> [core_request::Payload; 2] {
    [
        core_request::Payload::AttestationStart(AttestationStartRequest {
            token: token.into(),
            device_id,
            device_identity: Some(
                attestation_start_request::DeviceIdentity::AttestationIdentity(
                    AttestationIdentity {
                        backend: Some(attestation_identity::Backend::Tpm(TpmAttestationIdentity {
                            ek_public: vec![1],
                            ek_certificate_chain_der: vec![vec![2]],
                            ak_public: vec![3],
                        })),
                    },
                ),
            ),
        }),
        core_request::Payload::AttestationFinish(AttestationFinishRequest {
            token: token.into(),
            device_id,
            solution: Some(AttestationChallengeSolution {
                session_id: "unverified-challenge".into(),
                result: Some(attestation_challenge_solution::Result::Tpm(
                    TpmAttestationResponse {
                        activation_secret: vec![1; 32],
                        evidence: Some(Default::default()),
                    },
                )),
            }),
        }),
    ]
}

async fn send_attestation_request(
    context: &mut HandlerTestContext,
    payload: core_request::Payload,
) -> CoreResponse {
    context.mock_proxy().send_request(CoreRequest {
        id: 1,
        device_info: Some(make_device_info()),
        payload: Some(payload),
    });
    let response = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(response.id, 1);
    response
}

async fn assert_attestation_error(
    context: &mut HandlerTestContext,
    token: &str,
    device_id: Id,
    expected: Code,
) {
    for (operation, payload) in attestation_requests(token, device_id)
        .into_iter()
        .enumerate()
    {
        let response = send_attestation_request(context, payload).await;
        assert_eq!(
            assert_error_response(&response),
            expected,
            "attestation operation {operation}"
        );
    }
}

#[sqlx::test]
async fn test_attestation_stubs_preserve_device_and_enrollment_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let (user, device) = create_user_with_device(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    start_enrollment_session(&mut context, &token.id).await;
    let before_token = Token::find_by_id(&context.pool, &token.id).await.unwrap();
    let before_device = Device::find_by_id(&context.pool, device.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        before_device.attestation_status,
        DeviceAttestationStatus::NotAttested
    );
    assert!(before_device.attested_at.is_none());
    let before_devices = Device::count(&context.pool).await.unwrap();
    let before_polling_tokens = PollingToken::count(&context.pool).await.unwrap();
    while context.bidi_events_rx.try_recv().is_ok() {}
    let mut gateway_rx = context.take_gateway_rx();

    for payload in attestation_requests(&token.id, device.id) {
        let response = send_attestation_request(&mut context, payload).await;
        assert_eq!(assert_error_response(&response), Code::Unimplemented);
        assert_eq!(
            Device::find_by_id(&context.pool, device.id)
                .await
                .unwrap()
                .as_ref(),
            Some(&before_device)
        );
        assert!(
            DeviceIdentity::find_by_device_id(&context.pool, device.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            DeviceAttestationState::find_for_device(&context.pool, &token.id, device.id)
                .await
                .unwrap()
                .is_none()
        );
        let after_token = Token::find_by_id(&context.pool, &token.id).await.unwrap();
        assert_eq!(after_token.used_at, before_token.used_at);
        assert_eq!(after_token.expires_at, before_token.expires_at);
        assert_eq!(after_token.device_id, before_token.device_id);
        assert_eq!(Device::count(&context.pool).await.unwrap(), before_devices);
        assert_eq!(
            PollingToken::count(&context.pool).await.unwrap(),
            before_polling_tokens
        );
        assert!(matches!(
            context.bidi_events_rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
        assert!(matches!(
            gateway_rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
    }
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_attestation_requires_an_active_enrollment_session(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let (user, device) = create_user_with_device(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    assert_attestation_error(&mut context, &token.id, device.id, Code::Unauthenticated).await;
    assert!(
        Token::find_by_id(&context.pool, &token.id)
            .await
            .unwrap()
            .used_at
            .is_none()
    );
    assert_attestation_error(
        &mut context,
        "nonexistent-token",
        device.id,
        Code::Unauthenticated,
    )
    .await;

    let mut expired = Token::new(
        user.id,
        None,
        None,
        3600,
        Some(ENROLLMENT_TOKEN_TYPE.into()),
    );
    expired.used_at = Some(
        Utc::now().naive_utc()
            - TimeDelta::from_std(Settings::get_current_settings().enrollment_session_timeout())
                .unwrap()
            - TimeDelta::seconds(1),
    );
    expired.save(&context.pool).await.unwrap();
    assert_attestation_error(&mut context, &expired.id, device.id, Code::Unauthenticated).await;

    for token_type in [MFA_CONFIG_TOKEN_TYPE, PASSWORD_RESET_TOKEN_TYPE] {
        let mut wrong_type = Token::new(user.id, None, None, 3600, Some(token_type.into()));
        wrong_type.used_at = Some(Utc::now().naive_utc());
        wrong_type.save(&context.pool).await.unwrap();
        assert_attestation_error(
            &mut context,
            &wrong_type.id,
            device.id,
            Code::PermissionDenied,
        )
        .await;
    }
    start_enrollment_session(&mut context, &token.id).await;
    assert_attestation_error(&mut context, &token.id, device.id, Code::Unimplemented).await;
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_attestation_rejects_disabled_users(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let (mut user, device) = create_user_with_device(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    start_enrollment_session(&mut context, &token.id).await;
    user.is_active = false;
    user.save(&context.pool).await.unwrap();
    assert_attestation_error(&mut context, &token.id, device.id, Code::PermissionDenied).await;
    user.is_active = true;
    user.save(&context.pool).await.unwrap();
    assert_attestation_error(&mut context, &token.id, device.id, Code::Unimplemented).await;
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_attestation_rejects_missing_and_foreign_devices(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let (user, device) = create_user_with_device(&context.pool).await;
    let (_, foreign_device) = create_user_with_device(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    start_enrollment_session(&mut context, &token.id).await;
    assert_attestation_error(
        &mut context,
        &token.id,
        foreign_device.id,
        Code::PermissionDenied,
    )
    .await;
    assert_attestation_error(
        &mut context,
        &token.id,
        device.id.max(foreign_device.id) + 1,
        Code::NotFound,
    )
    .await;
    assert_attestation_error(&mut context, &token.id, device.id, Code::Unimplemented).await;
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_attestation_is_authorized_after_user_activation(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let (user, device) = create_user_with_device(&context.pool).await;
    assert!(!user.is_enrolled());
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    start_enrollment_session(&mut context, &token.id).await;
    let before = Token::find_by_id(&context.pool, &token.id).await.unwrap();
    let response = send_activate_user(&mut context, &token.id, STRONG_PASSWORD, None).await;
    assert!(matches!(
        response.payload,
        Some(core_response::Payload::Empty(()))
    ));
    assert!(
        User::find_by_id(&context.pool, user.id)
            .await
            .unwrap()
            .unwrap()
            .is_enrolled()
    );
    assert_attestation_error(&mut context, &token.id, device.id, Code::Unimplemented).await;
    let after = Token::find_by_id(&context.pool, &token.id).await.unwrap();
    assert_eq!(after.used_at, before.used_at);
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_attestation_cannot_replace_an_existing_identity(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let (user, device) = create_user_with_device(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    start_enrollment_session(&mut context, &token.id).await;
    let identity = DeviceIdentity {
        device_id: device.id,
        backend: DeviceIdentityBackend::Tpm,
        public_key_der: vec![1, 2, 3],
        apple_sign_count: None,
        tpm_ek_public: None,
        tpm_ek_certificate_chain_der: None,
        apple_key_id: None,
    };
    let mut connection = context.pool.acquire().await.unwrap();
    let approved = identity
        .save_verified(
            &mut connection,
            DeviceAttestationStatus::AttestedManually,
            device.created,
        )
        .await
        .unwrap();
    drop(connection);
    assert_attestation_error(&mut context, &token.id, device.id, Code::FailedPrecondition).await;
    assert!(
        DeviceIdentity::find_by_device_id(&context.pool, device.id)
            .await
            .unwrap()
            == Some(identity)
    );
    assert_eq!(
        Device::find_by_id(&context.pool, device.id).await.unwrap(),
        Some(approved)
    );
    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_attestation_requires_identity_and_solution_payloads(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let (user, device) = create_user_with_device(&context.pool).await;
    let token = create_enrollment_token(&context.pool, user.id, Some(user.id)).await;
    start_enrollment_session(&mut context, &token.id).await;
    for mut payload in attestation_requests(&token.id, device.id) {
        match &mut payload {
            core_request::Payload::AttestationStart(request) => request.device_identity = None,
            core_request::Payload::AttestationFinish(request) => request.solution = None,
            _ => unreachable!(),
        }
        let response = send_attestation_request(&mut context, payload).await;
        assert_eq!(assert_error_response(&response), Code::InvalidArgument);
    }
    context.finish().await.expect_server_finished().await;
}
