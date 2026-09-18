use defguard_common::{
    db::models::{MFAMethod, Session, SessionState, User},
    testing::smtp::configure_working_smtp,
};
use defguard_core::events::ApiEventType;
use defguard_proto::{client_types::MfaMethod, proxy::core_response};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::support::{
    assert_error_response, complete_proxy_handshake, create_polling_token, create_user_with_device,
    generate_totp_code, register_webauthn_key, send_code_mfa_setup_finish,
    send_code_mfa_setup_start, send_mfa_config_authorize, send_mfa_config_send_code,
    send_mfa_config_start, setup_user_totp_mfa, totp_code_from_base32_secret,
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

/// The fallback's email verification enables the factor without a second setup round-trip.
#[sqlx::test]
async fn test_email_fallback_enables_email_factor(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _smtp = configure_working_smtp(&context.pool).await;

    let (user, device) = create_user_with_device(&context.pool).await;
    let polling_token = create_polling_token(&context.pool, device.id).await;
    // Verify that enabling MFA invalidates an existing password-only session.
    let web_session = Session::new(
        user.id,
        SessionState::PasswordVerified,
        "10.0.0.1".into(),
        None,
    );
    web_session.save(&context.pool).await.expect("save session");

    let start_response =
        send_mfa_config_start(&mut context, &polling_token, &device.wireguard_pubkey).await;
    let session = match &start_response.payload {
        Some(core_response::Payload::MfaConfigStart(response)) => response,
        _ => panic!("expected MfaConfigStartResponse"),
    };
    assert!(session.available_methods.is_empty());
    assert!(session.email_fallback);
    let session_token = session.session_token.clone();

    let sent = send_mfa_config_send_code(&mut context, &session_token).await;
    assert!(
        matches!(
            sent.payload,
            Some(core_response::Payload::MfaConfigSendCode(_))
        ),
        "the fallback must send an email code"
    );

    // Reload because mfa_config_start rotated the email secret.
    let user = User::find_by_id(&context.pool, user.id)
        .await
        .expect("find user")
        .expect("user exists");
    let code = user
        .generate_email_mfa_code()
        .expect("generate_email_mfa_code");
    let authorized =
        send_mfa_config_authorize(&mut context, &session_token, MfaMethod::Email, &code).await;
    let issued = match &authorized.payload {
        Some(core_response::Payload::MfaConfigAuthorize(response)) => {
            response.recovery_codes.clone()
        }
        _ => panic!("expected MfaConfigAuthorizeResponse"),
    };
    assert!(
        !issued.is_empty(),
        "the fallback must issue recovery codes when it enables the factor"
    );

    let user = User::find_by_id(&context.pool, user.id)
        .await
        .expect("find user")
        .expect("user exists");
    assert!(user.mfa_enabled, "the fallback must enable MFA");
    assert!(
        user.email_mfa_enabled,
        "the fallback must enable the email factor"
    );
    assert_eq!(
        user.mfa_method,
        MFAMethod::Email,
        "email must become the default MFA method"
    );
    assert_eq!(
        user.recovery_codes, issued,
        "the response must contain the stored recovery codes"
    );
    assert!(
        Session::find_by_id(&context.pool, &web_session.id)
            .await
            .expect("find session")
            .is_none(),
        "enabling MFA must end password-only web sessions"
    );

    let event = context
        .event_rx
        .try_recv()
        .expect("an event must be emitted");
    assert!(
        matches!(*event.event, ApiEventType::MfaEmailEnabled),
        "expected MfaEmailEnabled event"
    );

    // The session remains usable to add a second factor, keeping existing codes.
    let issued = issued.clone();
    let setup = send_code_mfa_setup_start(&mut context, &session_token, MfaMethod::Totp).await;
    let secret = match &setup.payload {
        Some(core_response::Payload::CodeMfaSetupStartResponse(response)) => response
            .totp_secret
            .clone()
            .expect("totp_secret must be returned"),
        _ => panic!("expected CodeMfaSetupStartResponse"),
    };
    let finish = send_code_mfa_setup_finish(
        &mut context,
        &session_token,
        MfaMethod::Totp,
        &totp_code_from_base32_secret(&secret),
    )
    .await;
    match &finish.payload {
        Some(core_response::Payload::CodeMfaSetupFinishResponse(response)) => assert!(
            response.recovery_codes.is_empty(),
            "adding a second factor must not return new recovery codes"
        ),
        _ => panic!("expected CodeMfaSetupFinishResponse"),
    }
    let user = User::find_by_id(&context.pool, user.id)
        .await
        .expect("find user")
        .expect("user exists");
    assert!(user.totp_enabled, "setup must enable the TOTP factor");
    assert!(
        user.email_mfa_enabled,
        "the email factor stays enabled after adding TOTP"
    );
    assert_eq!(
        user.recovery_codes, issued,
        "adding a factor must not change the stored recovery codes"
    );

    context.finish().await.expect_server_finished().await;
}

/// A security key counts as a factor, so the fallback must not clobber the user's codes.
#[sqlx::test]
async fn test_fido2_only_user_is_not_treated_as_no_factor(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _smtp = configure_working_smtp(&context.pool).await;

    let (mut user, device) = create_user_with_device(&context.pool).await;
    // The security key is the user's only factor; its registration issued recovery codes.
    register_webauthn_key(&context.pool, user.id).await;
    let issued = user
        .get_recovery_codes(&context.pool)
        .await
        .expect("get_recovery_codes")
        .expect("codes issued on first factor");
    assert!(
        !issued.is_empty(),
        "the user must start with recovery codes"
    );
    let polling_token = create_polling_token(&context.pool, device.id).await;

    // FIDO2 cannot authorize this flow, so no method is offered — but no fallback either.
    let start_response =
        send_mfa_config_start(&mut context, &polling_token, &device.wireguard_pubkey).await;
    let session = match &start_response.payload {
        Some(core_response::Payload::MfaConfigStart(response)) => response,
        _ => panic!("expected MfaConfigStartResponse"),
    };
    assert!(session.available_methods.is_empty());
    assert!(
        !session.email_fallback,
        "a security-key user must not be offered the email fallback"
    );
    let session_token = session.session_token.clone();

    let sent = send_mfa_config_send_code(&mut context, &session_token).await;
    assert_eq!(assert_error_response(&sent), tonic::Code::PermissionDenied);

    let user = User::find_by_id(&context.pool, user.id)
        .await
        .expect("find user")
        .expect("user exists");
    assert_eq!(
        user.recovery_codes, issued,
        "the fallback must not regenerate a security-key user's recovery codes"
    );
    assert!(
        !user.email_mfa_enabled,
        "the email factor must not be enabled"
    );

    context.finish().await.expect_server_finished().await;
}
