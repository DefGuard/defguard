//! Connection settings for the public HTTP(S) listener.

use std::time::Duration;

use axum_server::accept::{TcpConfigAcceptor, TcpKeepaliveConfig, TcpSocketConfig};

pub(super) const HTTP1_HEADER_READ_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const HTTP2_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(20);
/// Time to wait for a PING acknowledgement. This does not expire responsive idle connections.
pub(super) const HTTP2_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(20);
const TCP_KEEPALIVE_TIME: Duration = Duration::from_secs(60);
const TCP_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(15);

pub(crate) fn keepalive_acceptor() -> TcpConfigAcceptor {
    TcpConfigAcceptor::new(
        TcpSocketConfig::new().keepalive(
            TcpKeepaliveConfig::new()
                .time(TCP_KEEPALIVE_TIME)
                .interval(TCP_KEEPALIVE_INTERVAL),
        ),
    )
}

#[cfg(test)]
mod tests;
