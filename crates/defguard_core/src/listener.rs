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
mod tests {
    use std::{
        net::{Ipv4Addr, SocketAddr},
        time::Duration,
    };

    use axum::{Router, routing::get};
    use axum_server::{Handle, accept::Accept};
    use socket2::SockRef;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        time::timeout,
    };

    use super::{ConnectionTimeouts, KeepaliveAcceptor, configure_http_builder};

    const TEST_TIMEOUT: Duration = Duration::from_secs(1);
    const CLOSE_DEADLINE: Duration = Duration::from_secs(5);

    async fn spawn_server() -> (SocketAddr, Handle<SocketAddr>) {
        let listener = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = Handle::new();
        let mut server = axum_server::from_tcp(listener)
            .unwrap()
            .acceptor(KeepaliveAcceptor)
            .handle(handle.clone());
        configure_http_builder(
            server.http_builder(),
            ConnectionTimeouts {
                http1_header_read: TEST_TIMEOUT,
                http2_keepalive_interval: TEST_TIMEOUT,
                http2_keepalive_timeout: TEST_TIMEOUT,
            },
        );
        let app = Router::new().route("/", get(|| async { "ok" }));
        tokio::spawn(server.serve(app.into_make_service()));
        handle.listening().await;
        (addr, handle)
    }

    async fn read_until_closed(stream: &mut TcpStream) -> Vec<u8> {
        let mut received = Vec::new();
        timeout(CLOSE_DEADLINE, stream.read_to_end(&mut received))
            .await
            .expect("the core server should close the idle connection")
            .expect("reading from the connection should succeed");
        received
    }

    #[tokio::test]
    async fn test_idle_keepalive_connection_is_closed_after_response() {
        let (addr, _handle) = spawn_server().await;
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();

        let received = read_until_closed(&mut stream).await;
        assert!(received.starts_with(b"HTTP/1.1 200"));
    }

    #[tokio::test]
    async fn test_incomplete_request_headers_are_cut_off() {
        let (addr, _handle) = spawn_server().await;
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n")
            .await
            .unwrap();

        read_until_closed(&mut stream).await;
    }

    #[tokio::test]
    async fn test_accepted_connections_have_tcp_keepalive() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
        let _client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        assert!(!SockRef::from(&stream).keepalive().unwrap());

        let (stream, ()) = KeepaliveAcceptor.accept(stream, ()).await.unwrap();

        assert!(SockRef::from(&stream).keepalive().unwrap());
    }
}
