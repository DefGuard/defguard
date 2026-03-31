/// Password-reset flow tests.
///
/// Because SMTP is not configured in the test environment, the
/// `PasswordResetInit` (= `request_password_reset`) code path that requires a
/// real user + SMTP can only be tested for the "unknown email → silent Empty"
/// case.  For the `PasswordResetStart` / `PasswordReset` paths we manually
/// insert a `PASSWORD_RESET` token via `create_password_reset_token`, which
/// bypasses the need for SMTP entirely.

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use defguard_common::db::models::User;
use defguard_core::events::{BidiStreamEventType, PasswordResetEvent};
use defguard_proto::proxy::core_response;

use crate::tests::common::{HandlerTestContext, TEST_TIMEOUT};
use super::support::{
    STRONG_PASSWORD, assert_error_response, complete_proxy_handshake, create_enrollment_token,
    create_password_reset_token, create_user, send_password_reset, send_password_reset_init,
    send_password_reset_start,
};

// ---------------------------------------------------------------------------
// Test 1: PasswordResetInit with unknown email returns Empty silently
// ---------------------------------------------------------------------------

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
        _ => panic!("expected Empty response for unknown email, got: {:?}", response.payload.as_ref().map(|p| std::mem::discriminant(p))),
    }

    context.finish().await.expect_server_finished().await;
}

// ---------------------------------------------------------------------------
// Test 2: PasswordResetStart with a valid token returns a deadline
// ---------------------------------------------------------------------------

/// `PasswordResetStart` with a manually-inserted PASSWORD_RESET token for an
/// activated user must return `PasswordResetStartResponse { deadline_timestamp > 0 }`
/// and emit a `BidiStreamEvent::PasswordReset(PasswordResetStarted)` event.
#[sqlx::test]
async fn test_password_reset_start_returns_deadline(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    // Create a user and give them a password so `has_password()` is true.
    let mut user = create_user(&context.pool).await;
    user.set_password(STRONG_PASSWORD);
    user.save(&context.pool).await.expect("failed to save user with password");

    // Manually insert a PASSWORD_RESET token.
    let token = create_password_reset_token(&context.pool, &user).await;

    let response = send_password_reset_start(&mut context, &token.id).await;

    let deadline = match &response.payload {
        Some(core_response::Payload::PasswordResetStart(r)) => r.deadline_timestamp,
        _ => panic!(
            "expected PasswordResetStart response, got: {:?}",
            response.payload.as_ref().map(|p| std::mem::discriminant(p))
        ),
    };
    assert!(deadline > 0, "deadline_timestamp must be positive");

    // A BidiStreamEvent::PasswordReset(PasswordResetStarted) must have been emitted.
    let event = tokio::time::timeout(TEST_TIMEOUT, context.bidi_events_rx.recv())
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

// ---------------------------------------------------------------------------
// Test 3: Full password-reset flow succeeds
// ---------------------------------------------------------------------------

/// Full flow: insert token → start → reset with a strong password.
/// The handler must return `Empty`, the user's password hash must change in
/// the DB, and a `PasswordResetCompleted` event must be emitted.
#[sqlx::test]
async fn test_password_reset_completes_successfully(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    let mut user = create_user(&context.pool).await;
    user.set_password(STRONG_PASSWORD);
    user.save(&context.pool).await.expect("failed to save user with password");

    let token = create_password_reset_token(&context.pool, &user).await;

    // Start the session (consumes the PasswordResetStarted event).
    let start_response = send_password_reset_start(&mut context, &token.id).await;
    assert!(
        matches!(start_response.payload, Some(core_response::Payload::PasswordResetStart(_))),
        "start must succeed"
    );
    let _ = tokio::time::timeout(TEST_TIMEOUT, context.bidi_events_rx.recv()).await;

    // Reset the password.
    const NEW_PASSWORD: &str = "NewPass2!";
    let response = send_password_reset(&mut context, &token.id, NEW_PASSWORD).await;

    match &response.payload {
        Some(core_response::Payload::Empty(())) => {}
        _ => panic!(
            "expected Empty on successful password reset, got: {:?}",
            response.payload.as_ref().map(|p| std::mem::discriminant(p))
        ),
    }

    // Verify the password hash changed in the DB.
    let updated = User::find_by_username(&context.pool, &user.username)
        .await
        .expect("db query failed")
        .expect("user not found");
    assert!(updated.has_password(), "user must still have a password hash");
    // The new hash must differ from the original (old STRONG_PASSWORD hash stored before).
    assert_ne!(
        updated.password_hash,
        user.password_hash,
        "password hash must have changed after reset"
    );

    // A BidiStreamEvent::PasswordReset(PasswordResetCompleted) must have been emitted.
    let event = tokio::time::timeout(TEST_TIMEOUT, context.bidi_events_rx.recv())
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

// ---------------------------------------------------------------------------
// Test 4: Weak password in PasswordReset returns InvalidArgument
// ---------------------------------------------------------------------------

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
    user.save(&context.pool).await.expect("failed to save user with password");

    let token = create_password_reset_token(&context.pool, &user).await;

    // Start the session.
    let start_response = send_password_reset_start(&mut context, &token.id).await;
    assert!(
        matches!(start_response.payload, Some(core_response::Payload::PasswordResetStart(_))),
        "start must succeed"
    );
    let _ = tokio::time::timeout(TEST_TIMEOUT, context.bidi_events_rx.recv()).await;

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

// ---------------------------------------------------------------------------
// Test 5: PasswordResetStart with enrollment token returns PermissionDenied
// ---------------------------------------------------------------------------

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
