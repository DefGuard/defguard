use defguard_common::{
    db::models::{Session, SessionState, User},
    testing::smtp::configure_working_smtp,
};
use defguard_proto::{client_types::MfaMethod, proxy::core_response};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::support::{
    assert_error_response, complete_proxy_handshake, create_polling_token, create_user_with_device,
    generate_totp_code, send_code_mfa_setup_finish, send_code_mfa_setup_start,
    send_mfa_config_authorize, send_mfa_config_send_code, send_mfa_config_start,
    setup_user_totp_mfa, totp_code_from_base32_secret,
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

/// Email fallback authorizes the session without enabling a factor or issuing recovery codes.
#[sqlx::test]
async fn test_email_fallback_authorizes_without_enabling_a_factor(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
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
    assert!(
        matches!(
            authorized.payload,
            Some(core_response::Payload::MfaConfigAuthorize(_))
        ),
        "a valid email code must authorize the session"
    );

    let user = User::find_by_id(&context.pool, user.id)
        .await
        .expect("find user")
        .expect("user exists");
    assert!(!user.mfa_enabled, "authorization must not enable MFA");
    assert!(
        !user.email_mfa_enabled,
        "authorization must not enable the email factor"
    );
    assert!(
        user.recovery_codes.is_empty(),
        "authorization must not issue recovery codes"
    );

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
    let issued = match &finish.payload {
        Some(core_response::Payload::CodeMfaSetupFinishResponse(response)) => {
            &response.recovery_codes
        }
        _ => panic!("expected CodeMfaSetupFinishResponse"),
    };
    assert!(!issued.is_empty(), "setup must issue recovery codes");

    let user = User::find_by_id(&context.pool, user.id)
        .await
        .expect("find user")
        .expect("user exists");
    assert!(user.totp_enabled, "setup must enable the TOTP factor");
    assert_eq!(
        &user.recovery_codes, issued,
        "the response must contain the stored recovery codes"
    );
    assert!(
        Session::find_by_id(&context.pool, &web_session.id)
            .await
            .expect("find session")
            .is_none(),
        "enabling MFA must end password-only web sessions"
    );

    // Adding a second factor must keep the existing recovery codes.
    let issued = issued.clone();
    let started = send_code_mfa_setup_start(&mut context, &session_token, MfaMethod::Email).await;
    assert!(
        matches!(
            started.payload,
            Some(core_response::Payload::CodeMfaSetupStartResponse(_))
        ),
        "the session must allow a second factor"
    );
    // Reload because setup start rotated the email secret.
    let user = User::find_by_id(&context.pool, user.id)
        .await
        .expect("find user")
        .expect("user exists");
    let code = user
        .generate_email_mfa_code()
        .expect("generate_email_mfa_code");
    let finish =
        send_code_mfa_setup_finish(&mut context, &session_token, MfaMethod::Email, &code).await;
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
    assert!(user.email_mfa_enabled, "setup must enable the email factor");
    assert_eq!(
        user.recovery_codes, issued,
        "adding a factor must not change the stored recovery codes"
    );

    context.finish().await.expect_server_finished().await;
}
