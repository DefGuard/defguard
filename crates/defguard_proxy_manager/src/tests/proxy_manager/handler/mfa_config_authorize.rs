//! Authorizing an MFA configuration session with FIDO2 and OIDC, as opposed to a TOTP or email
//! code.

use std::sync::atomic::{AtomicU64, Ordering};

use base64::{
    Engine,
    prelude::{BASE64_STANDARD, BASE64_URL_SAFE_NO_PAD},
};
use defguard_common::{
    db::{
        Id,
        models::{
            User, WebAuthn,
            settings::{Settings, update_current_settings},
        },
    },
    testing::smtp::configure_working_smtp,
};
use defguard_core::{
    db::models::enrollment::{MfaConfigAuthState, Token},
    enterprise::handlers::openid_login::{MfaOidcState, build_state},
};
use defguard_proto::{
    client_types::{AuthFlowType, AuthInfoRequest, MfaConfigStartResponse, MfaMethod},
    proxy::{
        ClientMfaOidcAuthenticateRequest, CoreRequest, CoreResponse, core_request, core_response,
    },
};
use ed25519_dalek::{Signer, SigningKey};
use getrandom::{SysRng, rand_core::UnwrapErr};
use reqwest::Url;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use tonic::Code;
use webauthn_authenticator_rs::{WebauthnAuthenticator, softpasskey::SoftPasskey};
use webauthn_rs::prelude::{Passkey, Uuid};

use super::support::{
    assert_error_response, assert_error_response_details, clear_test_license,
    complete_proxy_handshake, create_oidc_provider, create_polling_token, create_user,
    create_user_with_device, link_user_oidc_identity, make_device_info, make_oidc_code,
    send_code_mfa_setup_start, send_mfa_config_authorize, send_mfa_config_authorize_fido2,
    send_mfa_config_fido2_challenge, send_mfa_config_start, set_public_proxy_url,
    set_test_license_business,
};
use crate::tests::common::{HandlerTestContext, MockOidcProvider};

/// A domain-based URL so the WebAuthn `rp_id` is stable across the test.
const TEST_DEFGUARD_URL: &str = "http://localhost:8000";
const TEST_RP_ID: &str = "localhost";

/// Starts an MFA configuration session for a fresh user set up by `configure_user`, returning
/// the response, the user and whatever `configure_user` returned.
async fn start_session<R>(
    context: &mut HandlerTestContext,
    configure_user: impl AsyncFnOnce(&PgPool, &mut User<Id>) -> R,
) -> (MfaConfigStartResponse, User<Id>, R) {
    let (mut user, device) = create_user_with_device(&context.pool).await;
    let configured = configure_user(&context.pool, &mut user).await;
    let polling_token = create_polling_token(&context.pool, device.id).await;
    let response = send_mfa_config_start(context, &polling_token, &device.wireguard_pubkey).await;
    match response.payload {
        Some(core_response::Payload::MfaConfigStart(response)) => (response, user, configured),
        other => panic!(
            "expected MfaConfigStartResponse, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    }
}

fn assert_authorized(response: &CoreResponse) {
    assert!(
        matches!(
            response.payload,
            Some(core_response::Payload::MfaConfigAuthorize(_))
        ),
        "expected MfaConfigAuthorizeResponse, got: {:?}",
        response.payload.as_ref().map(std::mem::discriminant)
    );
}

/// An authorized session lets the user start setting up a factor.
async fn assert_setup_allowed(context: &mut HandlerTestContext, session_token: &str) {
    let setup = send_code_mfa_setup_start(context, session_token, MfaMethod::Totp).await;
    assert!(
        matches!(
            setup.payload,
            Some(core_response::Payload::CodeMfaSetupStartResponse(_))
        ),
        "an authorized session must allow factor setup"
    );
}

/// Registers a security key whose private key the test holds.
///
/// A `SoftPasskey` registration yields a well-formed `Passkey`, whose public key is then
/// swapped for an Ed25519 key the test can sign with.
async fn register_signing_key(pool: &PgPool, user_id: Id) -> (SigningKey, Vec<u8>) {
    let mut settings = Settings::get_current_settings();
    settings.defguard_url = TEST_DEFGUARD_URL.to_owned();
    update_current_settings(pool, settings)
        .await
        .expect("failed to set defguard_url for WebAuthn test");
    let webauthn = Settings::get_current_settings()
        .build_webauthn()
        .expect("build webauthn");
    let (ccr, registration) = webauthn
        .start_passkey_registration(Uuid::new_v4(), "user", "user", None)
        .expect("start passkey registration");
    let origin = Url::parse(TEST_DEFGUARD_URL).expect("valid origin url");
    let credential = WebauthnAuthenticator::new(SoftPasskey::new(true))
        .do_registration(origin, ccr)
        .expect("soft passkey registration");
    let passkey = webauthn
        .finish_passkey_registration(&credential, &registration)
        .expect("finish passkey registration");

    let signing_key = SigningKey::generate(&mut UnwrapErr(SysRng));
    let mut passkey = serde_json::to_value(&passkey).expect("serialize passkey");
    passkey["cred"]["cred"]["key"] = json!({
        "EC_OKP": {
            "curve": "ED25519",
            "x": BASE64_URL_SAFE_NO_PAD.encode(signing_key.verifying_key().as_bytes()),
        }
    });
    let passkey: Passkey = serde_json::from_value(passkey).expect("deserialize passkey");
    WebAuthn::new(user_id, "test key".to_owned(), &passkey)
        .expect("new webauthn key")
        .save(pool)
        .await
        .expect("save webauthn key");
    (signing_key, passkey.cred_id().to_vec())
}

/// Signs `challenge` the way a security key does for `ctap_hid_fido2`'s verifier, returning the
/// base64url signature and the authenticator data.
fn sign_challenge(signing_key: &SigningKey, challenge: &str) -> (String, Vec<u8>) {
    // rpIdHash, flags (user present + verified), signature counter.
    let mut auth_data = Sha256::digest(TEST_RP_ID).to_vec();
    auth_data.push(0x05);
    auth_data.extend_from_slice(&1_u32.to_be_bytes());
    let mut message = auth_data.clone();
    message.extend_from_slice(&Sha256::digest(challenge));
    let signature = signing_key.sign(&message);
    (
        BASE64_URL_SAFE_NO_PAD.encode(signature.to_bytes()),
        auth_data,
    )
}

/// Requests a FIDO2 challenge, returning it and the offered credential ids.
async fn fido2_challenge(
    context: &mut HandlerTestContext,
    session_token: &str,
) -> (String, Vec<String>) {
    let response = send_mfa_config_fido2_challenge(context, session_token).await;
    match response.payload {
        Some(core_response::Payload::MfaConfigFido2Challenge(response)) => {
            (response.challenge, response.credential_ids)
        }
        other => panic!(
            "expected MfaConfigFido2ChallengeResponse, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    }
}

#[sqlx::test]
async fn test_fido2_authorizes_session(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let (session, _, (signing_key, credential_id)) =
        start_session(&mut context, async |pool, user| {
            register_signing_key(pool, user.id).await
        })
        .await;
    assert_eq!(session.available_methods, vec![MfaMethod::Fido2 as i32]);
    let session_token = session.session_token;

    let (challenge, credential_ids) = fido2_challenge(&mut context, &session_token).await;
    assert_eq!(
        credential_ids,
        vec![BASE64_URL_SAFE_NO_PAD.encode(&credential_id)]
    );
    let (signature, auth_data) = sign_challenge(&signing_key, &challenge);
    let authorized = send_mfa_config_authorize_fido2(
        &mut context,
        &session_token,
        Some(signature),
        Some(auth_data),
        Some(credential_id),
    )
    .await;
    assert_authorized(&authorized);

    let token = Token::find_by_id(&context.pool, &session_token)
        .await
        .expect("find token");
    assert!(
        token.mfa_setup_state.is_none(),
        "authorization must clear the authorization state"
    );
    assert_setup_allowed(&mut context, &session_token).await;

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_fido2_challenge_requires_security_key(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _smtp = configure_working_smtp(&context.pool).await;

    let (session, _, ()) = start_session(&mut context, async |_, _| {}).await;
    assert!(session.email_fallback);

    let challenge = send_mfa_config_fido2_challenge(&mut context, &session.session_token).await;
    assert_eq!(assert_error_response(&challenge), Code::PermissionDenied);
    let authorized = send_mfa_config_authorize_fido2(
        &mut context,
        &session.session_token,
        Some(String::new()),
        Some(Vec::new()),
        None,
    )
    .await;
    assert_eq!(assert_error_response(&authorized), Code::PermissionDenied);

    context.finish().await.expect_server_finished().await;
}

/// Each challenge verifies once: a wrong assertion burns it, and so does any later replay.
#[sqlx::test]
async fn test_fido2_challenge_is_single_use(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let (session, _, (signing_key, credential_id)) =
        start_session(&mut context, async |pool, user| {
            register_signing_key(pool, user.id).await
        })
        .await;
    assert_eq!(session.available_methods, vec![MfaMethod::Fido2 as i32]);
    let session_token = session.session_token;

    let without_challenge = send_mfa_config_authorize_fido2(
        &mut context,
        &session_token,
        Some(String::new()),
        Some(Vec::new()),
        None,
    )
    .await;
    assert_eq!(
        assert_error_response(&without_challenge),
        Code::FailedPrecondition
    );

    let (challenge, _) = fido2_challenge(&mut context, &session_token).await;
    let (signature, auth_data) = sign_challenge(&signing_key, "not the issued challenge");
    let rejected = send_mfa_config_authorize_fido2(
        &mut context,
        &session_token,
        Some(signature),
        Some(auth_data),
        Some(credential_id.clone()),
    )
    .await;
    assert_eq!(assert_error_response(&rejected), Code::Unauthenticated);

    // A correct assertion over the burnt challenge no longer verifies.
    let (signature, auth_data) = sign_challenge(&signing_key, &challenge);
    let replayed = send_mfa_config_authorize_fido2(
        &mut context,
        &session_token,
        Some(signature),
        Some(auth_data),
        Some(credential_id.clone()),
    )
    .await;
    assert_eq!(assert_error_response(&replayed), Code::FailedPrecondition);

    // A fresh challenge still authorizes.
    let (challenge, _) = fido2_challenge(&mut context, &session_token).await;
    let (signature, auth_data) = sign_challenge(&signing_key, &challenge);
    let authorized = send_mfa_config_authorize_fido2(
        &mut context,
        &session_token,
        Some(signature),
        Some(auth_data),
        Some(credential_id),
    )
    .await;
    assert_authorized(&authorized);

    context.finish().await.expect_server_finished().await;
}

async fn send_oidc_auth_info(
    context: &mut HandlerTestContext,
    session_token: &str,
) -> CoreResponse {
    static AUTH_INFO_CTR: AtomicU64 = AtomicU64::new(6000);
    context.mock_proxy().send_request(CoreRequest {
        id: AUTH_INFO_CTR.fetch_add(1, Ordering::Relaxed),
        device_info: None,
        payload: Some(core_request::Payload::AuthInfo(AuthInfoRequest {
            state: Some(session_token.to_owned()),
            auth_flow_type: AuthFlowType::Mfa as i32,
            ..Default::default()
        })),
    });
    context.mock_proxy_mut().recv_outbound().await
}

/// Sends `AuthInfo` for the MFA flow and returns the payload the authorize URL's state carries.
async fn oidc_auth_info_state(context: &mut HandlerTestContext, session_token: &str) -> String {
    let response = send_oidc_auth_info(context, session_token).await;
    let auth_info = match &response.payload {
        Some(core_response::Payload::AuthInfo(response)) => response,
        other => panic!(
            "expected AuthInfo response, got: {:?}",
            other.as_ref().map(std::mem::discriminant)
        ),
    };
    let url = Url::parse(&auth_info.url).expect("failed to parse authorize URL");
    let state = url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, value)| value.into_owned())
        .expect("authorize URL must carry a state parameter");
    let decoded = BASE64_STANDARD
        .decode(state.as_bytes())
        .expect("state must be base64");
    let decoded = String::from_utf8(decoded).expect("state must be UTF-8");
    let (_csrf, payload) = decoded
        .split_once('.')
        .expect("state must be <csrf>.<payload>");
    payload.to_owned()
}

/// Sends the Edge OIDC callback for `state_payload`, authenticating as `sub`.
async fn send_oidc_callback(
    context: &mut HandlerTestContext,
    state_payload: &str,
    sub: &str,
) -> CoreResponse {
    let nonce = "mfa-config-oidc-nonce";
    static OIDC_CALLBACK_CTR: AtomicU64 = AtomicU64::new(6250);
    context.mock_proxy().send_request(CoreRequest {
        id: OIDC_CALLBACK_CTR.fetch_add(1, Ordering::Relaxed),
        device_info: Some(make_device_info()),
        payload: Some(core_request::Payload::ClientMfaOidcAuthenticate(
            ClientMfaOidcAuthenticateRequest {
                code: make_oidc_code(sub, sub, nonce),
                state: build_state(Some(state_payload.to_owned())).secret().clone(),
                nonce: nonce.to_owned(),
            },
        )),
    });
    context.mock_proxy_mut().recv_outbound().await
}

/// Sets up a business license and a mock OpenID provider.
async fn setup_oidc(context: &HandlerTestContext) -> MockOidcProvider {
    set_test_license_business();
    let mock = MockOidcProvider::start().await;
    let _provider = create_oidc_provider(&context.pool, &mock).await;
    set_public_proxy_url(&context.pool, &mock.base_url).await;
    mock
}

#[sqlx::test]
async fn test_oidc_authorizes_session(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _mock = setup_oidc(&context).await;

    let (session, user, ()) = start_session(&mut context, async |pool, user| {
        link_user_oidc_identity(pool, user).await;
    })
    .await;
    assert_eq!(session.available_methods, vec![MfaMethod::Oidc as i32]);
    assert!(
        !session.email_fallback,
        "OIDC must replace the email fallback"
    );
    let session_token = session.session_token;

    let state = oidc_auth_info_state(&mut context, &session_token).await;
    let MfaOidcState { token, attempt_id } =
        MfaOidcState::parse(&state).expect("state must carry an attempt id");
    assert_eq!(token, session_token);
    let stored = Token::find_by_id(&context.pool, &session_token)
        .await
        .expect("find token")
        .get_mfa_config_auth_state();
    assert_eq!(
        stored,
        Some(MfaConfigAuthState::Oidc {
            attempt_id,
            completed: false
        })
    );

    let pending =
        send_mfa_config_authorize(&mut context, &session_token, MfaMethod::Oidc, "").await;
    assert_eq!(assert_error_response(&pending), Code::FailedPrecondition);

    let callback = send_oidc_callback(&mut context, &state, &user.email).await;
    assert!(
        matches!(callback.payload, Some(core_response::Payload::Empty(()))),
        "expected Empty after the OIDC callback, got: {:?}",
        callback.payload.as_ref().map(std::mem::discriminant)
    );

    let authorized =
        send_mfa_config_authorize(&mut context, &session_token, MfaMethod::Oidc, "").await;
    assert_authorized(&authorized);
    assert_setup_allowed(&mut context, &session_token).await;

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

/// Without a business license OIDC cannot authorize, leaving the email fallback.
#[sqlx::test]
async fn test_oidc_requires_license(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _smtp = configure_working_smtp(&context.pool).await;
    let _mock = setup_oidc(&context).await;
    clear_test_license();

    let (session, _, ()) = start_session(&mut context, async |pool, user| {
        link_user_oidc_identity(pool, user).await;
    })
    .await;
    assert!(session.available_methods.is_empty());
    assert!(session.email_fallback);

    let authorized =
        send_mfa_config_authorize(&mut context, &session.session_token, MfaMethod::Oidc, "").await;
    assert_eq!(assert_error_response(&authorized), Code::PermissionDenied);

    context.finish().await.expect_server_finished().await;
}

/// A user without a linked OIDC identity cannot use OIDC, leaving the email fallback.
#[sqlx::test]
async fn test_unlinked_user_keeps_email_fallback(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _smtp = configure_working_smtp(&context.pool).await;
    let _mock = setup_oidc(&context).await;

    let (session, _, ()) = start_session(&mut context, async |_, _| {}).await;
    assert!(session.available_methods.is_empty());
    assert!(session.email_fallback);

    let auth_info = send_oidc_auth_info(&mut context, &session.session_token).await;
    assert_eq!(assert_error_response(&auth_info), Code::PermissionDenied);

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

/// A completed OIDC attempt survives until the client authorizes, whatever starts a new attempt.
#[sqlx::test]
async fn test_oidc_completion_survives_new_attempt(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _mock = setup_oidc(&context).await;

    let (session, user, _) = start_session(&mut context, async |pool, user| {
        link_user_oidc_identity(pool, user).await;
        register_signing_key(pool, user.id).await
    })
    .await;
    assert_eq!(
        session.available_methods,
        vec![MfaMethod::Fido2 as i32, MfaMethod::Oidc as i32]
    );
    let session_token = session.session_token;

    let state = oidc_auth_info_state(&mut context, &session_token).await;
    let callback = send_oidc_callback(&mut context, &state, &user.email).await;
    assert!(matches!(
        callback.payload,
        Some(core_response::Payload::Empty(()))
    ));

    let auth_info = send_oidc_auth_info(&mut context, &session_token).await;
    assert_eq!(assert_error_response(&auth_info), Code::FailedPrecondition);
    let challenge = send_mfa_config_fido2_challenge(&mut context, &session_token).await;
    assert_eq!(assert_error_response(&challenge), Code::FailedPrecondition);

    let authorized =
        send_mfa_config_authorize(&mut context, &session_token, MfaMethod::Oidc, "").await;
    assert_authorized(&authorized);

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

/// A callback for a superseded attempt neither completes the new one nor ends the session.
#[sqlx::test]
async fn test_oidc_rejects_stale_attempt(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _mock = setup_oidc(&context).await;

    let (session, user, ()) = start_session(&mut context, async |pool, user| {
        link_user_oidc_identity(pool, user).await;
    })
    .await;
    let session_token = session.session_token;

    let stale = oidc_auth_info_state(&mut context, &session_token).await;
    let current = oidc_auth_info_state(&mut context, &session_token).await;
    assert_ne!(stale, current, "each AuthInfo must start a new attempt");

    let callback = send_oidc_callback(&mut context, &stale, &user.email).await;
    let (code, message) = assert_error_response_details(&callback);
    assert_eq!(code, Code::InvalidArgument);
    assert_eq!(message, "stale OIDC MFA attempt");

    let pending =
        send_mfa_config_authorize(&mut context, &session_token, MfaMethod::Oidc, "").await;
    assert_eq!(assert_error_response(&pending), Code::FailedPrecondition);

    let callback = send_oidc_callback(&mut context, &current, &user.email).await;
    assert!(matches!(
        callback.payload,
        Some(core_response::Payload::Empty(()))
    ));
    let authorized =
        send_mfa_config_authorize(&mut context, &session_token, MfaMethod::Oidc, "").await;
    assert_authorized(&authorized);

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}

/// Authenticating as someone else ends the session.
#[sqlx::test]
async fn test_oidc_foreign_identity_ends_session(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    let _mock = setup_oidc(&context).await;

    let mut other = create_user(&context.pool).await;
    link_user_oidc_identity(&context.pool, &mut other).await;
    let (session, _, ()) = start_session(&mut context, async |pool, user| {
        link_user_oidc_identity(pool, user).await;
    })
    .await;
    let session_token = session.session_token;

    let state = oidc_auth_info_state(&mut context, &session_token).await;
    let callback = send_oidc_callback(&mut context, &state, &other.email).await;
    let (code, message) = assert_error_response_details(&callback);
    assert_eq!(code, Code::Unauthenticated);
    assert_eq!(message, "unauthorized");

    assert!(
        Token::find_by_id(&context.pool, &session_token)
            .await
            .is_err(),
        "a foreign identity must end the session"
    );
    let authorized =
        send_mfa_config_authorize(&mut context, &session_token, MfaMethod::Oidc, "").await;
    assert_error_response(&authorized);

    clear_test_license();
    context.finish().await.expect_server_finished().await;
}
