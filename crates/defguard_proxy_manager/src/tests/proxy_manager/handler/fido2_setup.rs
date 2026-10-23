//! Integration tests for configuring a FIDO2 factor through the gRPC Configure-MFA flow.
//! The ceremony is driven with a software token whose attestation CA each test trusts on its own.

use defguard_common::db::{
    Id,
    models::{
        WebAuthn,
        settings::{Settings, update_current_settings},
        user::User,
        webauthn::{AuthenticatorKind, RegistrationSource},
    },
};
use defguard_core::{
    db::models::enrollment::Token,
    enterprise::fido2::attestation::{certificate_sha256, trust_test_authenticator},
    events::ApiEventType,
};
use defguard_proto::{client_types::MfaMethod, proxy::core_response};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use webauthn_authenticator_rs::{
    AuthenticatorBackend, WebauthnAuthenticator,
    prelude::Url,
    softpasskey::SoftPasskey,
    softtoken::{AAGUID, SoftToken},
};
use webauthn_rs::prelude::{CreationChallengeResponse, RegisterPublicKeyCredential};

use super::support::{
    assert_error_response, complete_proxy_handshake, create_polling_token, create_user_with_device,
    generate_totp_code, send_code_mfa_setup_finish_fido2, send_code_mfa_setup_start,
    send_mfa_config_authorize, send_mfa_config_start, set_test_license_business,
    set_test_license_enterprise, setup_user_totp_mfa,
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

const SOFT_TOKEN_DESCRIPTION: &str = "Test soft token";

/// Returns the token with the DER of its attestation CA.
fn soft_token(trusted: bool) -> (SoftToken, Vec<u8>) {
    let (token, ca) = SoftToken::new(true).expect("software token");
    let ca_der = ca.to_der().expect("soft token CA encodes");
    if trusted {
        trust_test_authenticator(&ca_der, AAGUID, SOFT_TOKEN_DESCRIPTION)
            .expect("trust soft token CA");
    }
    (token, ca_der)
}

fn register(
    authenticator: impl AuthenticatorBackend,
    origin: Url,
    ccr: CreationChallengeResponse,
) -> String {
    let rpkc: RegisterPublicKeyCredential = WebauthnAuthenticator::new(authenticator)
        .do_registration(origin, ccr)
        .expect("software authenticator registration");
    serde_json::to_string(&rpkc).expect("serialize attestation")
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
    set_test_license_enterprise();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let origin = set_webauthn_origin(&context).await;

    let (session_token, user_id) = authorized_session(&mut context).await;

    // Trusted before the ceremony starts, which snapshots the CA list into its state.
    let (authenticator, ca_der) = soft_token(true);
    let ccr = fido2_setup_start(&mut context, &session_token).await;
    let token = Token::find_by_id(&context.pool, &session_token)
        .await
        .expect("token exists");
    assert!(
        token.mfa_setup_state.is_some(),
        "the in-progress passkey registration must be stored on the token"
    );

    let attestation = register(authenticator, origin, ccr);

    let finish = send_code_mfa_setup_finish_fido2(
        &mut context,
        &session_token,
        Some("my key"),
        Some(&attestation),
    )
    .await;
    if let Some(core_response::Payload::CoreError(err)) = &finish.payload {
        panic!("FIDO2 setup must finish successfully, got: {}", err.message);
    }
    assert!(
        matches!(
            finish.payload,
            Some(core_response::Payload::CodeMfaSetupFinishResponse(_))
        ),
        "FIDO2 setup must finish successfully"
    );

    let keys = WebAuthn::all_for_user(&context.pool, user_id)
        .await
        .expect("query webauthn");
    assert_eq!(
        keys.len(),
        1,
        "a security key must be registered for the user"
    );
    assert_eq!(keys[0].authenticator_kind, AuthenticatorKind::Hardware);
    assert_eq!(
        keys[0].authenticator_name.as_deref(),
        Some(SOFT_TOKEN_DESCRIPTION)
    );
    assert_eq!(keys[0].registration_source, RegistrationSource::Desktop);
    assert_eq!(keys[0].attestation_aaguid, Some(AAGUID));
    assert_eq!(
        keys[0].attestation_root_sha256,
        Some(certificate_sha256(&ca_der))
    );
    assert!(keys[0].attestation_valid_until.is_some());
    assert!(keys[0].registered_at.is_some());
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

    let event = context
        .event_rx
        .try_recv()
        .expect("an event must be emitted");
    assert!(
        matches!(*event.event, ApiEventType::MfaSecurityKeyAdded { .. }),
        "expected MfaSecurityKeyAdded event"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_fido2_finish_without_start_is_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_enterprise();
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
        !WebAuthn::exists_for_user(&context.pool, user_id, None)
            .await
            .expect("query webauthn"),
        "no credential must be saved on failure"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_fido2_finish_missing_fields_is_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    set_test_license_enterprise();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let origin = set_webauthn_origin(&context).await;

    let (session_token, _user_id) = authorized_session(&mut context).await;
    let (authenticator, _) = soft_token(true);
    let ccr = fido2_setup_start(&mut context, &session_token).await;
    let attestation = register(authenticator, origin, ccr);

    let response =
        send_code_mfa_setup_finish_fido2(&mut context, &session_token, None, Some(&attestation))
            .await;
    assert_eq!(
        assert_error_response(&response),
        tonic::Code::InvalidArgument
    );

    let response =
        send_code_mfa_setup_finish_fido2(&mut context, &session_token, Some("my key"), None).await;
    assert_eq!(
        assert_error_response(&response),
        tonic::Code::InvalidArgument
    );

    context.finish().await.expect_server_finished().await;
}

async fn assert_registration_rejected(
    authenticator: impl AuthenticatorBackend,
    options: PgConnectOptions,
) {
    set_test_license_enterprise();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let origin = set_webauthn_origin(&context).await;

    let (session_token, user_id) = authorized_session(&mut context).await;
    let ccr = fido2_setup_start(&mut context, &session_token).await;
    let attestation = register(authenticator, origin, ccr);

    let response = send_code_mfa_setup_finish_fido2(
        &mut context,
        &session_token,
        Some("my key"),
        Some(&attestation),
    )
    .await;
    assert_eq!(
        assert_error_response(&response),
        tonic::Code::InvalidArgument
    );
    assert!(
        !WebAuthn::exists_for_user(&context.pool, user_id, None)
            .await
            .expect("query webauthn"),
        "an untrusted authenticator must not be saved"
    );
    let token = Token::find_by_id(&context.pool, &session_token)
        .await
        .expect("token exists");
    assert!(
        token.mfa_setup_state.is_none(),
        "a rejected attestation must consume the setup challenge"
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_fido2_self_attested_key_is_rejected(_: PgPoolOptions, options: PgConnectOptions) {
    assert_registration_rejected(SoftPasskey::new(true), options).await;
}

#[sqlx::test]
async fn test_fido2_untrusted_attestation_ca_is_rejected(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    assert_registration_rejected(soft_token(false).0, options).await;
}

/// FIDO2 security keys as an MFA factor need the Enterprise tier. A Business license must not
/// start a FIDO2 setup, even when the client skips the lock that the UI shows.
#[sqlx::test]
async fn test_fido2_setup_start_requires_enterprise_license(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_business();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_webauthn_origin(&context).await;

    let (session_token, _user_id) = authorized_session(&mut context).await;

    let response = send_code_mfa_setup_start(&mut context, &session_token, MfaMethod::Fido2).await;
    assert_eq!(
        assert_error_response(&response),
        tonic::Code::InvalidArgument
    );
    let token = Token::find_by_id(&context.pool, &session_token)
        .await
        .expect("token exists");
    assert!(
        token.mfa_setup_state.is_none(),
        "a refused FIDO2 setup must not store a passkey registration"
    );

    set_test_license_enterprise();
    fido2_setup_start(&mut context, &session_token).await;

    context.finish().await.expect_server_finished().await;
}

/// The finish checks the license again, so a setup started under Enterprise cannot save a key
/// after the license drops to Business.
#[sqlx::test]
async fn test_fido2_setup_finish_requires_enterprise_license(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    set_test_license_enterprise();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let origin = set_webauthn_origin(&context).await;

    let (session_token, user_id) = authorized_session(&mut context).await;
    let (authenticator, _) = soft_token(true);
    let ccr = fido2_setup_start(&mut context, &session_token).await;
    let attestation = register(authenticator, origin, ccr);

    set_test_license_business();
    let response = send_code_mfa_setup_finish_fido2(
        &mut context,
        &session_token,
        Some("my key"),
        Some(&attestation),
    )
    .await;
    assert_eq!(
        assert_error_response(&response),
        tonic::Code::InvalidArgument
    );
    assert!(
        !WebAuthn::exists_for_user(&context.pool, user_id, None)
            .await
            .expect("query webauthn"),
        "a refused FIDO2 finish must not save a security key"
    );

    // The same attestation finishes once the license allows FIDO2 again, so only the license
    // refused it.
    set_test_license_enterprise();
    let finish = send_code_mfa_setup_finish_fido2(
        &mut context,
        &session_token,
        Some("my key"),
        Some(&attestation),
    )
    .await;
    assert!(
        matches!(
            finish.payload,
            Some(core_response::Payload::CodeMfaSetupFinishResponse(_))
        ),
        "FIDO2 setup must finish under an Enterprise license"
    );

    context.finish().await.expect_server_finished().await;
}
