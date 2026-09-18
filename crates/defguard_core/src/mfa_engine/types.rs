//! Domain types for the MFA engine.
//!
//! These are proto-free: the conversions to and from the proto messages live in the gRPC handler
//! (`grpc::proxy::client_mfa`), so the engine can be exercised without a transport.

/// Result returned by a start method. `token` is returned exactly once, and
/// `step_attempt_id` identifies the initial attempt for the multi-step contract.
#[derive(Debug)]
pub struct StartOutcome {
    pub token: String,
    pub step_attempt_id: String,
    pub challenge: Option<String>,
    /// FIDO2 only: the credentials registered for this user, base64url. The
    /// client offers them to the security key, which answers for the one it
    /// holds.
    pub credential_ids: Vec<String>,
    pub superseded_token_hash: Option<String>,
}

/// Compatibility proof shared by the legacy and attempt-bound finish paths.
#[derive(Debug, Eq, PartialEq)]
pub struct Proof {
    pub code: Option<String>,
    /// Legacy mobile public key or base64-encoded FIDO2 signature.
    pub auth_pub_key: Option<String>,
    pub step_attempt_id: Option<String>,
    /// FIDO2 authenticator data in the transitional packed representation.
    pub auth_data: Option<Vec<u8>>,
    /// FIDO2 credential selected by the client. Names the security key in use, so verification goes
    /// straight to its public key.
    pub credential_id: Option<Vec<u8>>,
}

/// Outcome returned by a finish operation.
#[derive(Debug, PartialEq)]
pub enum FinishOutcome {
    /// The step just submitted advanced the flow to `next_step` (0-indexed).
    Advanced { next_step: u32 },
    /// The final step completed and a preshared key was minted.
    Completed { preshared_key: String },
    /// Still waiting for external confirmation (OIDC or mobile auth) to be completed.
    AwaitingExternal,
}
