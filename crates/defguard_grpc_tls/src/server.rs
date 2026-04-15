//! Server-side mTLS utilities for gateway and proxy gRPC servers.

use tonic::{
    Request, Status,
    transport::server::{TcpConnectInfo, TlsConnectInfo},
};
use x509_parser::prelude::*;

/// Returns a tonic interceptor closure that enforces the Core client certificate serial.
///
/// On every incoming RPC the interceptor:
/// 1. Reads the peer certificate from [`TlsConnectInfo`] (populated by tonic's TLS stack).
/// 2. Parses its serial via `x509_parser`.
/// 3. Rejects the request with [`Status::unauthenticated`] if the serial does not match
///    `expected_serial` (case-insensitive, colon-separated hex comparison).
///
/// When `expected_serial` is `None` the check is skipped entirely, which allows the
/// same service builder chain to be used in plain-HTTP (no-TLS) development mode.
///
/// # Usage
///
/// Place this interceptor **outermost** in the `ServiceBuilder` chain so that
/// authentication runs before any other middleware:
///
/// ```rust,ignore
/// ServiceBuilder::new()
///     .layer(tonic::service::interceptor(certificate_serial_interceptor(Some(serial))))
///     .layer(/* version layer */)
///     .service(/* gRPC service */)
/// ```
pub fn certificate_serial_interceptor(
    expected_serial: Option<String>,
) -> impl Fn(Request<()>) -> Result<Request<()>, Status> + Clone + Send + 'static {
    move |req| {
        let Some(ref serial) = expected_serial else {
            return Ok(req);
        };

        let certs = req
            .extensions()
            .get::<TlsConnectInfo<TcpConnectInfo>>()
            .and_then(|info| info.peer_certs())
            .ok_or_else(|| Status::unauthenticated("Missing client certificate"))?;

        let der = certs
            .first()
            .ok_or_else(|| Status::unauthenticated("Empty client certificate chain"))?;

        let (_, cert) = parse_x509_certificate(der)
            .map_err(|_| Status::unauthenticated("Invalid client certificate"))?;

        let peer_serial = cert.tbs_certificate.raw_serial_as_string();

        if !peer_serial.eq_ignore_ascii_case(serial) {
            return Err(Status::unauthenticated(
                "Client certificate serial mismatch",
            ));
        }

        Ok(req)
    }
}
