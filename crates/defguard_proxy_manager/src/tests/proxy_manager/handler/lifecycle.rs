use defguard_core::events::ProxyConnectionEvent;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::support::complete_proxy_handshake;
use crate::tests::common::{HandlerTestContext, reload_proxy};

#[sqlx::test]
async fn test_proxy_marked_connected_after_handshake(_: PgPoolOptions, options: PgConnectOptions) {
    let mut context = HandlerTestContext::new(options).await;

    // Proxy row as created, snapshotted before the handler task was spawned:
    // connected_at must be None or older than disconnected_at. Re-reading the
    // row from the database here would race the handler's mark_connected()
    // write, which happens as soon as the bidi stream is established - before
    // the InitialInfo message that complete_proxy_handshake() waits for.
    assert!(
        !context.proxy.is_connected(),
        "proxy should not be connected before handshake"
    );

    complete_proxy_handshake(&mut context).await;
    let mut connection_events_rx = context.take_connection_events_rx();

    let proxy_after = context.reload_proxy().await;
    assert!(
        proxy_after.is_connected(),
        "proxy should be connected after handshake"
    );
    assert!(
        proxy_after.connected_at.is_some(),
        "connected_at should be set"
    );
    assert_eq!(
        connection_events_rx.recv().await,
        Some(ProxyConnectionEvent::Connected {
            proxy_id: context.proxy.id,
            proxy_name: context.proxy.name.clone(),
        })
    );

    context.finish().await.expect_server_finished().await;
}

#[sqlx::test]
async fn test_proxy_marked_disconnected_when_stream_closes(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    complete_proxy_handshake(&mut context).await;

    let proxy_id = context.proxy.id;
    let proxy_name = context.proxy.name.clone();
    let mut connection_events_rx = context.take_connection_events_rx();
    let pool = context.pool.clone();
    let mock_proxy = context.finish().await;

    let proxy_after = reload_proxy(&pool, proxy_id).await;
    assert!(
        !proxy_after.is_connected(),
        "proxy should be disconnected after stream closes"
    );
    assert!(
        proxy_after.disconnected_at.is_some(),
        "disconnected_at should be set after stream close"
    );
    assert_eq!(
        connection_events_rx.recv().await,
        Some(ProxyConnectionEvent::Connected {
            proxy_id,
            proxy_name: proxy_name.clone(),
        })
    );
    assert_eq!(
        connection_events_rx.recv().await,
        Some(ProxyConnectionEvent::Disconnected {
            proxy_id,
            proxy_name,
        })
    );

    mock_proxy.expect_server_finished().await;
}

#[sqlx::test]
async fn test_proxy_marked_disconnected_when_stream_errors(
    _: PgPoolOptions,
    options: PgConnectOptions,
) {
    let mut context = HandlerTestContext::new(options).await;

    complete_proxy_handshake(&mut context).await;

    context
        .mock_proxy_mut()
        .send_stream_error(tonic::Status::internal("mock proxy stream failure"));

    let proxy_id = context.proxy.id;
    let pool = context.pool.clone();
    let mock_proxy = context.finish_after_error().await;

    let proxy_after = reload_proxy(&pool, proxy_id).await;
    assert!(
        !proxy_after.is_connected(),
        "proxy should be disconnected after stream error"
    );
    assert!(
        proxy_after.disconnected_at.is_some(),
        "disconnected_at should be set after stream error"
    );

    mock_proxy.expect_server_finished().await;
}
