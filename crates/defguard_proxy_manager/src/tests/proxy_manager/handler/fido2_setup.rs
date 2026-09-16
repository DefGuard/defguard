//! Integration tests for configuring a FIDO2 factor through the gRPC Configure-MFA flow.
//! The ceremony is driven with a `SoftPasskey`, like the REST WebAuthn test in
//! `defguard_core/tests/integration/api/auth.rs`.

use defguard_common::db::{
    Id,
    models::{
        WebAuthn,
        settings::{Settings, update_current_settings},
        user::User,
    },
};
use defguard_core::{db::models::enrollment::Token, events::ApiEventType};
use defguard_proto::{client_types::MfaMethod, proxy::core_response};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use webauthn_authenticator_rs::{WebauthnAuthenticator, prelude::Url, softpasskey::SoftPasskey};
use webauthn_rs::prelude::CreationChallengeResponse;

use super::support::{
    assert_error_response, complete_proxy_handshake, create_polling_token, create_user_with_device,
    generate_totp_code, send_code_mfa_setup_finish_fido2, send_code_mfa_setup_start,
    send_mfa_config_authorize, send_mfa_config_start, setup_user_totp_mfa,
};
use crate::tests::common::HandlerTestContext;

/// A domain-based URL so the derived WebAuthn `rp_id` is stable across the test.
const TEST_DEFGUARD_URL: &str = "http://localhost:8000";

/// Makes the ceremony's relying party id match the origin we hand the authenticator.
async fn set_webauthn_origin(context: &HandlerTestContext) -> Url {
    let mut settings = Settings::get_current_settings();
    settings.defguard_url = TEST_DEFGUARD_URL.to_owned();
    update_current_settings(&context.pool, settings)
        .await
        .expect("failed to set defguard_url for WebAuthn test");
    Url::parse(TEST_DEFGUARD_URL).expect("valid origin url")
}

/// Authorize a session with TOTP, returning the token gating factor setup and its user id.
async fn authorized_session(context: &mut HandlerTestContext) -> (String, Id) {
    let (mut user, device) = create_user_with_device(&context.pool).await;
    setup_user_totp_mfa(&context.pool, &mut user).await;
    let polling_token = create_polling_token(&context.pool, device.id).await;

    let start = send_mfa_config_start(context, &polling_token, &device.wireguard_pubkey).await;
    let session_token = match &start.payload {
        Some(core_response::Payload::MfaConfigStart(response)) => response.session_token.clone(),
        _ => panic!("expected MfaConfigStartResponse"),
    };
    let authorized = send_mfa_config_authorize(
        context,
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
    (session_token, user.id)
}

/// Run the FIDO2 setup start and return the parsed creation challenge.
async fn fido2_setup_start(
    context: &mut HandlerTestContext,
    session_token: &str,
) -> CreationChallengeResponse {
    let response = send_code_mfa_setup_start(context, session_token, MfaMethod::Fido2).await;
    match &response.payload {
        Some(core_response::Payload::CodeMfaSetupStartResponse(response)) => {
            assert!(
                response.totp_secret.is_none(),
                "FIDO2 setup must not return a TOTP secret"
            );
            let challenge = response
                .fido2_creation_challenge
                .as_ref()
                .expect("FIDO2 setup must return a creation challenge");
            serde_json::from_str(challenge).expect("challenge must deserialize")
        }
        _ => panic!("expected CodeMfaSetupStartResponse"),
    }
}

#[sqlx::test]
async fn test_fido2_setup_registers_security_key(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let origin = set_webauthn_origin(&context).await;

    let (session_token, user_id) = authorized_session(&mut context).await;

    let ccr = fido2_setup_start(&mut context, &session_token).await;
    let token = Token::find_by_id(&context.pool, &session_token)
        .await
        .expect("token exists");
    assert!(
        token.mfa_setup_state.is_some(),
        "the in-progress passkey registration must be stored on the token"
    );

    // Drive the authenticator and finish the ceremony.
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let rpkc = authenticator
        .do_registration(origin, ccr)
        .expect("software authenticator registration");
    let attestation = serde_json::to_string(&rpkc).expect("serialize attestation");

    let finish =
        send_code_mfa_setup_finish_fido2(&mut context, &session_token, Some("my key"), Some(&attestation))
            .await;
    assert!(
        matches!(
            finish.payload,
            Some(core_response::Payload::CodeMfaSetupFinishResponse(_))
        ),
        "FIDO2 setup must finish successfully"
    );

    assert!(
        WebAuthn::exists_for_user(&context.pool, user_id)
            .await
            .expect("query webauthn"),
        "a security key must be registered for the user"
    );
    let user = User::find_by_id(&context.pool, user_id)
        .await
        .expect("find user")
        .expect("user exists");
    assert!(user.mfa_enabled, "registering a key must enable MFA");

    let token = Token::find_by_id(&context.pool, &session_token)
        .await
        .expect("token exists");
    assert!(
        token.mfa_setup_state.is_none(),
        "the ceremony state must be cleared after a successful finish"
    );

    let event = context.event_rx.try_recv().expect("an event must be emitted");
    assert!(
        matches!(*event.event, ApiEventType::MfaSecurityKeyAdded { .. }),
        "expected MfaSecurityKeyAdded event"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_fido2_finish_without_start_is_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_webauthn_origin(&context).await;

    let (session_token, user_id) = authorized_session(&mut context).await;

    // Syntactically valid JSON, but no ceremony was started.
    let response = send_code_mfa_setup_finish_fido2(
        &mut context,
        &session_token,
        Some("my key"),
        Some("{\"not\":\"an attestation\"}"),
    )
    .await;
    let code = assert_error_response(&response);
    assert_eq!(code, tonic::Code::InvalidArgument);
    assert!(
        !WebAuthn::exists_for_user(&context.pool, user_id)
            .await
            .expect("query webauthn"),
        "no credential must be saved on failure"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_fido2_finish_missing_fields_is_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let origin = set_webauthn_origin(&context).await;

    let (session_token, _user_id) = authorized_session(&mut context).await;
    let ccr = fido2_setup_start(&mut context, &session_token).await;
    let mut authenticator = WebauthnAuthenticator::new(SoftPasskey::new(true));
    let rpkc = authenticator
        .do_registration(origin, ccr)
        .expect("software authenticator registration");
    let attestation = serde_json::to_string(&rpkc).expect("serialize attestation");

    let response =
        send_code_mfa_setup_finish_fido2(&mut context, &session_token, None, Some(&attestation)).await;
    assert_eq!(assert_error_response(&response), tonic::Code::InvalidArgument);

    let response =
        send_code_mfa_setup_finish_fido2(&mut context, &session_token, Some("my key"), None).await;
    assert_eq!(assert_error_response(&response), tonic::Code::InvalidArgument);

    context.finish().await.expect_server_finished().await;
}
