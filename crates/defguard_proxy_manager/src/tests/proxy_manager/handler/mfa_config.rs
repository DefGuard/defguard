use defguard_proto::{client_types::MfaMethod, proxy::core_response};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::support::{
    assert_error_response, complete_proxy_handshake, create_polling_token, create_user_with_device,
    generate_totp_code, send_code_mfa_setup_start, send_mfa_config_authorize,
    send_mfa_config_start, setup_user_totp_mfa,
};
use crate::tests::common::HandlerTestContext;

/// The session token must gate factor setup until the user proves a configured factor.
#[sqlx::test]
async fn test_mfa_config_session_gates_factor_setup(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let polling_token = create_polling_token(&context.pool, device.id).await;

    let start_response =
        send_mfa_config_start(&mut context, &polling_token, &device.wireguard_pubkey).await;
    let session = match &start_response.payload {
        Some(core_response::Payload::MfaConfigStart(response)) => response,
        _ => panic!("expected MfaConfigStartResponse"),
    };
    assert_eq!(session.available_methods, vec![MfaMethod::Totp as i32]);
    assert!(!session.email_fallback);
    let session_token = session.session_token.clone();

    let unauthorized =
        send_code_mfa_setup_start(&mut context, &session_token, MfaMethod::Totp).await;
    assert_error_response(&unauthorized);

    let authorized = send_mfa_config_authorize(
        &mut context,
        &session_token,
        MfaMethod::Totp,
        &generate_totp_code(&user),
    )
    .await;
    assert!(
        matches!(
            authorized.payload,
            Some(core_response::Payload::MfaConfigAuthorize(_))
        ),
        "a valid TOTP code must authorize the session"
    );

    let setup = send_code_mfa_setup_start(&mut context, &session_token, MfaMethod::Totp).await;
    match &setup.payload {
        Some(core_response::Payload::CodeMfaSetupStartResponse(response)) => {
            assert!(response.totp_secret.is_some());
        }
        _ => panic!("expected CodeMfaSetupStartResponse"),
    }

    let replayed = send_mfa_config_authorize(
        &mut context,
        &session_token,
        MfaMethod::Totp,
        &generate_totp_code(&user),
    )
    .await;
    assert_error_response(&replayed);

    context.finish().await.expect_server_finished().await;
}
