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
