use std::sync::Arc;

use defguard_proto::{
    client_types::{AuthFlowType, AuthInfoRequest},
    proxy::{CoreRequest, core_request},
};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::sync::Semaphore;
use tonic::Code;

use super::support::{assert_error_response, clear_test_license, complete_proxy_handshake};
use crate::tests::common::HandlerTestContext;

fn auth_info_request(id: u64) -> CoreRequest {
    CoreRequest {
        id,
        device_info: None,
        payload: Some(core_request::Payload::AuthInfo(AuthInfoRequest {
            auth_flow_type: AuthFlowType::Enrollment as i32,
            ..Default::default()
        })),
    }
}

#[sqlx::test]
async fn test_bidi_processes_concurrent_requests_with_their_request_ids(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    clear_test_license();
    let semaphore = Arc::new(Semaphore::new(0));
    let mut context = HandlerTestContext::new_with_semaphore(options, Arc::clone(&semaphore)).await;
    complete_proxy_handshake(&mut context).await;

    context.mock_proxy().send_request(auth_info_request(101));
    context.mock_proxy().send_request(auth_info_request(202));
    semaphore.add_permits(2);

    let first = context.mock_proxy_mut().recv_outbound().await;
    let second = context.mock_proxy_mut().recv_outbound().await;
    let mut ids = [first.id, second.id];
    ids.sort_unstable();
    assert_eq!(ids, [101, 202]);
    assert_eq!(assert_error_response(&first), Code::FailedPrecondition);
    assert_eq!(assert_error_response(&second), Code::FailedPrecondition);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_bidi_respects_configured_semaphore_limit(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    clear_test_license();
    let semaphore = Arc::new(Semaphore::new(0));
    let mut context = HandlerTestContext::new_with_semaphore(options, Arc::clone(&semaphore)).await;
    complete_proxy_handshake(&mut context).await;

    context.mock_proxy().send_request(auth_info_request(301));
    context.mock_proxy().send_request(auth_info_request(302));
    context.mock_proxy_mut().expect_no_outbound().await;

    semaphore.add_permits(1);
    let first = context.mock_proxy_mut().recv_outbound().await;
    let second = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(assert_error_response(&first), Code::FailedPrecondition);
    assert_eq!(assert_error_response(&second), Code::FailedPrecondition);
    let mut ids = [first.id, second.id];
    ids.sort_unstable();
    assert_eq!(ids, [301, 302]);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_bidi_request_errors_do_not_prevent_later_requests(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    clear_test_license();
    let mut context = HandlerTestContext::new(options).await;
    complete_proxy_handshake(&mut context).await;

    context.mock_proxy().send_request(auth_info_request(401));
    let failed = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(failed.id, 401);
    assert_eq!(assert_error_response(&failed), Code::FailedPrecondition);

    context.mock_proxy().send_request(auth_info_request(402));
    let subsequent = context.mock_proxy_mut().recv_outbound().await;
    assert_eq!(subsequent.id, 402);
    assert_eq!(assert_error_response(&subsequent), Code::FailedPrecondition);

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_bidi_stream_close_cancels_requests_waiting_for_permits(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    clear_test_license();
    let semaphore = Arc::new(Semaphore::new(0));
    let mut context = HandlerTestContext::new_with_semaphore(options, Arc::clone(&semaphore)).await;
    complete_proxy_handshake(&mut context).await;

    context.mock_proxy().send_request(auth_info_request(501));
    context.mock_proxy().send_request(auth_info_request(502));
    context.mock_proxy_mut().expect_no_outbound().await;

    context.finish().await.expect_server_finished().await;
    assert_eq!(semaphore.available_permits(), 0);
    semaphore.add_permits(1);
    assert_eq!(semaphore.available_permits(), 1);
}
