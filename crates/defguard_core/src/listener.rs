//! Connection settings for the public HTTP(S) listener.

use std::{future::Ready, io, time::Duration};

use axum_server::accept::{Accept, DefaultAcceptor};
use hyper_util::{
    rt::{TokioExecutor, TokioTimer},
    server::conn::auto::Builder,
};
use socket2::{SockRef, TcpKeepalive};
use tokio::net::TcpStream;
use tracing::warn;

const HTTP1_HEADER_READ_TIMEOUT: Duration = Duration::from_secs(30);
const HTTP2_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
const HTTP2_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(20);
const TCP_KEEPALIVE_TIME: Duration = Duration::from_secs(60);
const TCP_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Clone, Copy, Debug)]
pub(crate) struct ConnectionTimeouts {
    /// Also bounds how long an idle HTTP/1 keep-alive connection waits for its next request.
    pub(crate) http1_header_read: Duration,
    pub(crate) http2_keepalive_interval: Duration,
    pub(crate) http2_keepalive_timeout: Duration,
}

impl Default for ConnectionTimeouts {
    fn default() -> Self {
        Self {
            http1_header_read: HTTP1_HEADER_READ_TIMEOUT,
            http2_keepalive_interval: HTTP2_KEEPALIVE_INTERVAL,
            http2_keepalive_timeout: HTTP2_KEEPALIVE_TIMEOUT,
        }
    }
}

/// Hyper ignores its timeouts unless a timer is configured.
/// HTTP/2 keepalive detects unresponsive peers but does not expire responsive idle connections.
pub(crate) fn configure_http_builder(
    builder: &mut Builder<TokioExecutor>,
    timeouts: ConnectionTimeouts,
) {
    builder
        .http1()
        .timer(TokioTimer::new())
        .header_read_timeout(timeouts.http1_header_read);
    builder
        .http2()
        .timer(TokioTimer::new())
        .keep_alive_interval(timeouts.http2_keepalive_interval)
        .keep_alive_timeout(timeouts.http2_keepalive_timeout);
}

/// Enables keepalive on every accepted socket, including upgraded WebSocket connections.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct KeepaliveAcceptor;

impl<S> Accept<TcpStream, S> for KeepaliveAcceptor {
    type Stream = TcpStream;
    type Service = S;
    type Future = Ready<io::Result<(TcpStream, S)>>;

    fn accept(&self, stream: TcpStream, service: S) -> Self::Future {
        let keepalive = TcpKeepalive::new()
            .with_time(TCP_KEEPALIVE_TIME)
            .with_interval(TCP_KEEPALIVE_INTERVAL);
        if let Err(err) = SockRef::from(&stream).set_tcp_keepalive(&keepalive) {
            warn!("Failed to enable TCP keepalive on an accepted connection: {err}");
        }
        DefaultAcceptor.accept(stream, service)
    }
}

#[cfg(test)]
mod tests;
