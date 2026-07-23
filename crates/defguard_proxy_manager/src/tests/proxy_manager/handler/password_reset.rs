use defguard_common::{
    db::{
        Id,
        models::{
            User,
            settings::{Settings, update_current_settings},
        },
    },
    testing::smtp::MockSmtpServer,
};
use defguard_core::events::{BidiStreamEventType, PasswordResetEvent};
use defguard_proto::proxy::core_response;
use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
    query_scalar,
};
use tokio::time::timeout;

use super::support::{
    STRONG_PASSWORD, assert_error_response, complete_proxy_handshake, create_enrollment_token,
    create_password_reset_token, create_user, send_password_reset, send_password_reset_init,
    send_password_reset_start,
};
use crate::tests::common::{HandlerTestContext, TEST_TIMEOUT};

/// `PasswordResetInit` for a completely unknown email must return `Empty`
/// (the server intentionally hides whether the address exists).
#[sqlx::test]
async fn test_password_reset_init_silent_success_for_unknown_email(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Send a PasswordResetInit for an email that does not exist in the DB.
    let response = send_password_reset_init(&mut context, "nobody@example.invalid").await;

    match &response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty response for unknown email, got: {:?}",
            response.payload.as_ref().map(std::mem::discriminant)
        ),
    }

    context.finish().await.expect_server_finished().await;
}

/// `PasswordResetStart` with a manually-inserted PASSWORD_RESET token for an
/// activated user must return `PasswordResetStartResponse { deadline_timestamp > 0 }`
/// and emit a `BidiStreamEvent::PasswordReset(PasswordResetStarted)` event.
#[sqlx::test]
async fn test_password_reset_start_returns_deadline(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Create a user and give them a password so `has_password()` is true.
    let mut user = create_user(&context.pool).await;
    user.set_password(STRONG_PASSWORD);
    user.save(&context.pool)
        .await
        .expect("failed to save user with password");

    // Manually insert a PASSWORD_RESET token.
    let token = create_password_reset_token(&context.pool, &user).await;

    let response = send_password_reset_start(&mut context, &token.id).await;

    let deadline = match &response.payload {
        Some(core_response::Payload::PasswordResetStart(r)) => r.deadline_timestamp,
        _ => panic!(
            "expected PasswordResetStart response, got: {:?}",
            response.payload.as_ref().map(std::mem::discriminant)
        ),
    };
    assert!(deadline > 0, "deadline_timestamp must be positive");

    // A BidiStreamEvent::PasswordReset(PasswordResetStarted) must have been emitted.
    let event = timeout(TEST_TIMEOUT, context.bidi_events_rx.recv())
        .await
        .expect("timed out waiting for BidiStreamEvent")
        .expect("bidi_events_rx closed");
    match event.event {
        BidiStreamEventType::PasswordReset(e) => match *e {
            PasswordResetEvent::PasswordResetStarted => {}
            other => panic!("expected PasswordResetStarted event, got: {other:?}"),
        },
        other => panic!("expected BidiStreamEventType::PasswordReset, got: {other:?}"),
    }

    context.finish().await.expect_server_finished().await;
}

/// Full flow: insert token → start → reset with a strong password.
/// The handler must return `Empty`, the user's password hash must change in
/// the DB, and a `PasswordResetCompleted` event must be emitted.
#[sqlx::test]
async fn test_password_reset_completes_successfully(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let mut user = create_user(&context.pool).await;
    user.set_password(STRONG_PASSWORD);
    user.save(&context.pool)
        .await
        .expect("failed to save user with password");

    let token = create_password_reset_token(&context.pool, &user).await;

    // Start the session (consumes the PasswordResetStarted event).
    let start_response = send_password_reset_start(&mut context, &token.id).await;
    match &start_response.payload {
        Some(core_response::Payload::PasswordResetStart(_)) => {}
        _ => panic!(
            "expected PasswordResetStart response, got: {:?}",
            start_response.payload.as_ref().map(std::mem::discriminant)
        ),
    }
    let _ = timeout(TEST_TIMEOUT, context.bidi_events_rx.recv()).await;

    // Reset the password.
    const NEW_PASSWORD: &str = "NewPass2!";
    let response = send_password_reset(&mut context, &token.id, NEW_PASSWORD).await;

    match &response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty on successful password reset, got: {:?}",
            response.payload.as_ref().map(std::mem::discriminant)
        ),
    }

    // Verify the password hash changed in the DB.
    let updated = User::find_by_username(&context.pool, &user.username)
        .await
        .expect("db query failed")
        .expect("user not found");
    assert!(
        updated.has_password(),
        "user must still have a password hash"
    );
    // The new hash must differ from the original (old STRONG_PASSWORD hash stored before).
    assert_ne!(
        updated.password_hash, user.password_hash,
        "password hash must have changed after reset"
    );

    // A BidiStreamEvent::PasswordReset(PasswordResetCompleted) must have been emitted.
    let event = timeout(TEST_TIMEOUT, context.bidi_events_rx.recv())
        .await
        .expect("timed out waiting for BidiStreamEvent")
        .expect("bidi_events_rx closed");
    match event.event {
        BidiStreamEventType::PasswordReset(e) => match *e {
            PasswordResetEvent::PasswordResetCompleted => {}
            other => panic!("expected PasswordResetCompleted event, got: {other:?}"),
        },
        other => panic!("expected BidiStreamEventType::PasswordReset, got: {other:?}"),
    }

    context.finish().await.expect_server_finished().await;
}

/// Submitting a weak password to `PasswordReset` (after a valid start) must
/// return `InvalidArgument`.
#[sqlx::test]
async fn test_password_reset_weak_password_returns_error(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let mut user = create_user(&context.pool).await;
    user.set_password(STRONG_PASSWORD);
    user.save(&context.pool)
        .await
        .expect("failed to save user with password");

    let token = create_password_reset_token(&context.pool, &user).await;

    // Start the session.
    let start_response = send_password_reset_start(&mut context, &token.id).await;
    match &start_response.payload {
        Some(core_response::Payload::PasswordResetStart(_)) => {}
        _ => panic!(
            "expected PasswordResetStart response, got: {:?}",
            start_response.payload.as_ref().map(std::mem::discriminant)
        ),
    }
    let _ = timeout(TEST_TIMEOUT, context.bidi_events_rx.recv()).await;

    // Submit a weak password.
    let response = send_password_reset(&mut context, &token.id, "weak").await;

    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::InvalidArgument,
        "weak password must return InvalidArgument"
    );

    context.finish().await.expect_server_finished().await;
}

/// Using an enrollment token (wrong type) in `PasswordResetStart` must be
/// rejected with `PermissionDenied`.
#[sqlx::test]
async fn test_password_reset_start_wrong_token_type_returns_error(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let user = create_user(&context.pool).await;
    // Create an ENROLLMENT token (wrong type for password reset).
    let token = create_enrollment_token(&context.pool, user.id, None).await;

    let response = send_password_reset_start(&mut context, &token.id).await;

    let code = assert_error_response(&response);
    assert_eq!(
        code,
        tonic::Code::PermissionDenied,
        "enrollment token used in PasswordResetStart must return PermissionDenied"
    );

    context.finish().await.expect_server_finished().await;
}

/// An externally-managed (LDAP) user with password management disabled must
/// receive a "disabled" notification email and no password reset token must be
/// created. The HTTP/gRPC response is indistinguishable from an unknown email.
#[sqlx::test]
async fn test_password_reset_init_disabled_user_no_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Capture outgoing mail so we can assert the "disabled" notification is sent.
    let smtp = MockSmtpServer::start().await;
    smtp.configure(&context.pool).await;

    // Enable LDAP password management disabling (preserving the SMTP config just
    // written to settings).
    let mut settings = Settings::get_current_settings();
    settings.ldap_disable_password_management = true;
    update_current_settings(&context.pool, settings)
        .await
        .unwrap();

    // Create an LDAP-sourced user without a local password.
    let mut user = create_user(&context.pool).await;
    user.from_ldap = true;
    user.save(&context.pool)
        .await
        .expect("failed to save LDAP user");

    let response = send_password_reset_init(&mut context, &user.email).await;

    // Response must be Empty — indistinguishable from unknown email.
    match &response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty response for disabled user, got: {:?}",
            response.payload.as_ref().map(std::mem::discriminant)
        ),
    }

    // No PASSWORD_RESET token must have been created ...
    assert_eq!(
        count_password_reset_tokens(&context.pool, user.id).await,
        0,
        "no password reset token should be created for disabled user"
    );

    // ... but the user must receive a "password reset disabled" notification.
    let mail = smtp.wait_for(|m| m.sent_to(&user.email)).await;
    assert!(
        mail.body_contains("Password reset disabled"),
        "externally-managed user must receive the password-reset-disabled email"
    );

    context.finish().await.expect_server_finished().await;
}

/// A user without a local password who is NOT externally-managed must receive
/// the same silent `Empty` response and no token — no feedback sent at all.
#[sqlx::test]
async fn test_password_reset_init_passwordless_user_silent_no_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Create a user with no password and no external IdP affiliation.
    let user = create_user(&context.pool).await;

    let response = send_password_reset_init(&mut context, &user.email).await;

    // Response must be Empty — silent, same as unknown email.
    match &response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty response for passwordless user, got: {:?}",
            response.payload.as_ref().map(std::mem::discriminant)
        ),
    }

    // No PASSWORD_RESET token must have been created.
    assert_eq!(
        count_password_reset_tokens(&context.pool, user.id).await,
        0,
        "no password reset token should be created for passwordless user"
    );

    context.finish().await.expect_server_finished().await;
}

/// Configure a valid public proxy URL so the reset-mail step in
/// `request_password_reset` can build its enrollment link. The mail send itself
/// is fire-and-forget and a no-op without SMTP, so this only needs to parse.
async fn set_public_proxy_url(pool: &PgPool) {
    let mut settings = Settings::get_current_settings();
    settings.public_proxy_url = "https://proxy.example.com".to_owned();
    update_current_settings(pool, settings)
        .await
        .expect("failed to set public_proxy_url");
}

/// Fetch the `id` of the single PASSWORD_RESET token for a user, asserting
/// exactly one exists.
async fn fetch_single_password_reset_token_id(pool: &PgPool, user_id: Id) -> String {
    let ids: Vec<String> =
        query_scalar("SELECT id FROM token WHERE user_id = $1 AND token_type = 'PASSWORD_RESET'")
            .bind(user_id)
            .fetch_all(pool)
            .await
            .expect("failed to query password reset tokens");
    assert_eq!(
        ids.len(),
        1,
        "expected exactly one PASSWORD_RESET token, found {}",
        ids.len()
    );
    ids.into_iter().next().unwrap()
}

/// Count the PASSWORD_RESET tokens for a user.
async fn count_password_reset_tokens(pool: &PgPool, user_id: Id) -> i64 {
    query_scalar("SELECT COUNT(*) FROM token WHERE user_id = $1 AND token_type = 'PASSWORD_RESET'")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("failed to query password reset token count")
}

/// Regression test for issue #3388: a passwordless user linked to an external
/// OIDC provider whose IdP does NOT disable password management must be able to
/// obtain a password reset. `PasswordResetInit` must create a PASSWORD_RESET
/// token and emit a `PasswordResetRequested` event (the reset email is sent via
/// the fire-and-forget mailer, which is not observable in tests).
///
/// Before the fix this user falls into the silent branch and no token is
/// created, so this test fails (red).
#[sqlx::test]
async fn test_password_reset_init_oidc_user_creates_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_public_proxy_url(&context.pool).await;

    // Passwordless user linked to an external OIDC provider. Password management
    // is NOT disabled (the default), so they may hold a local password.
    let mut user = create_user(&context.pool).await;
    user.openid_sub = Some("oidc-sub-123".to_owned());
    user.save(&context.pool)
        .await
        .expect("failed to save OIDC user");

    let response = send_password_reset_init(&mut context, &user.email).await;

    match &response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty response for OIDC user, got: {:?}",
            response.payload.as_ref().map(std::mem::discriminant)
        ),
    }

    // A PASSWORD_RESET token must have been created so the user can set a password.
    assert_eq!(
        count_password_reset_tokens(&context.pool, user.id).await,
        1,
        "a password reset token must be created for a passwordless OIDC user"
    );

    // A BidiStreamEvent::PasswordReset(PasswordResetRequested) must have been emitted.
    let event = timeout(TEST_TIMEOUT, context.bidi_events_rx.recv())
        .await
        .expect("timed out waiting for BidiStreamEvent")
        .expect("bidi_events_rx closed");
    match event.event {
        BidiStreamEventType::PasswordReset(e) => match *e {
            PasswordResetEvent::PasswordResetRequested => {}
            other => panic!("expected PasswordResetRequested event, got: {other:?}"),
        },
        other => panic!("expected BidiStreamEventType::PasswordReset, got: {other:?}"),
    }

    context.finish().await.expect_server_finished().await;
}

/// Regression test for issue #3388: a passwordless user synced from LDAP whose
/// IdP does NOT disable password management (the default) must be able to obtain
/// a password reset. `PasswordResetInit` must create a PASSWORD_RESET token.
///
/// Before the fix this user falls into the silent branch and no token is
/// created, so this test fails (red).
#[sqlx::test]
async fn test_password_reset_init_ldap_user_creates_token(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_public_proxy_url(&context.pool).await;

    // Passwordless LDAP-sourced user. `ldap_disable_password_management` is left
    // at its default (false), so they may hold a local password.
    let mut user = create_user(&context.pool).await;
    user.from_ldap = true;
    user.save(&context.pool)
        .await
        .expect("failed to save LDAP user");

    let response = send_password_reset_init(&mut context, &user.email).await;

    match &response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty response for LDAP user, got: {:?}",
            response.payload.as_ref().map(std::mem::discriminant)
        ),
    }

    assert_eq!(
        count_password_reset_tokens(&context.pool, user.id).await,
        1,
        "a password reset token must be created for a passwordless LDAP user"
    );

    context.finish().await.expect_server_finished().await;
}

/// Regression test for issue #3388, full flow: a passwordless OIDC user must be
/// able to complete a password reset end to end - init creates a token, start
/// succeeds, and reset sets their first local password.
///
/// Before the fix, `start_password_reset` rejects the user with
/// `PermissionDenied` because of its `!user.has_password()` guard, so this test
/// fails (red) at the start step.
#[sqlx::test]
async fn test_password_reset_completes_for_oidc_user(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;
    set_public_proxy_url(&context.pool).await;

    let mut user = create_user(&context.pool).await;
    user.openid_sub = Some("oidc-sub-456".to_owned());
    user.save(&context.pool)
        .await
        .expect("failed to save OIDC user");
    assert!(
        !user.has_password(),
        "precondition: OIDC user starts without a local password"
    );

    // Init: must create a reset token and emit PasswordResetRequested.
    let init_response = send_password_reset_init(&mut context, &user.email).await;
    match &init_response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty response on init, got: {:?}",
            init_response.payload.as_ref().map(std::mem::discriminant)
        ),
    }
    let token_id = fetch_single_password_reset_token_id(&context.pool, user.id).await;
    // Drain the PasswordResetRequested event.
    let _ = timeout(TEST_TIMEOUT, context.bidi_events_rx.recv()).await;

    // Start: must succeed even though the user has no password yet.
    let start_response = send_password_reset_start(&mut context, &token_id).await;
    match &start_response.payload {
        Some(core_response::Payload::PasswordResetStart(_)) => {}
        _ => panic!(
            "start must succeed for a passwordless OIDC user, got: {:?}",
            start_response.payload.as_ref().map(std::mem::discriminant)
        ),
    }
    // Drain the PasswordResetStarted event.
    let _ = timeout(TEST_TIMEOUT, context.bidi_events_rx.recv()).await;

    // Reset: sets the user's first local password.
    const NEW_PASSWORD: &str = "NewPass2!";
    let reset_response = send_password_reset(&mut context, &token_id, NEW_PASSWORD).await;
    match &reset_response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty on successful password reset, got: {:?}",
            reset_response.payload.as_ref().map(std::mem::discriminant)
        ),
    }

    // The user must now have a local password set in the DB.
    let updated = User::find_by_username(&context.pool, &user.username)
        .await
        .expect("db query failed")
        .expect("user not found");
    assert!(
        updated.has_password(),
        "OIDC user must have a local password after completing the reset"
    );

    // A PasswordResetCompleted event must have been emitted.
    let event = timeout(TEST_TIMEOUT, context.bidi_events_rx.recv())
        .await
        .expect("timed out waiting for BidiStreamEvent")
        .expect("bidi_events_rx closed");
    match event.event {
        BidiStreamEventType::PasswordReset(e) => match *e {
            PasswordResetEvent::PasswordResetCompleted => {}
            other => panic!("expected PasswordResetCompleted event, got: {other:?}"),
        },
        other => panic!("expected BidiStreamEventType::PasswordReset, got: {other:?}"),
    }

    context.finish().await.expect_server_finished().await;
}
