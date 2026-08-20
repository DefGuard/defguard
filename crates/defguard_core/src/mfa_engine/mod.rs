//! Connect-time multi-step MFA engine.
//!
//! The engine owns the step cursor and attempt lifecycle over the durable
//! [`VpnClientMfaSession`](defguard_common::db::models::vpn_client_mfa_session::VpnClientMfaSession)
//! store. The gRPC handlers in `grpc::proxy::client_mfa` stay thin adapters that convert the
//! frozen proto messages to and from the domain types here; the engine never sees a proto message.

pub mod authorize;
pub mod method;
pub mod types;
