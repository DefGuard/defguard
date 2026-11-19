//! Connection settings for the public HTTP(S) listener.

use std::time::Duration;

use axum_server::accept::{TcpConfigAcceptor, TcpKeepaliveConfig, TcpSocketConfig};

pub(crate) fn keepalive_acceptor() -> TcpConfigAcceptor {
    TcpConfigAcceptor::new(
        TcpSocketConfig::new().keepalive(
            TcpKeepaliveConfig::new()
                .time(Duration::from_secs(60))
                .interval(Duration::from_secs(15)),
        ),
    )
}

#[cfg(test)]
mod tests;
