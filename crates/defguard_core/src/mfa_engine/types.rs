//! Domain types for the MFA engine.
//!
//! These are proto-free: the conversions to and from the proto messages live in the gRPC handler
//! (`grpc::proxy::client_mfa`), so the engine can be exercised without a transport.

/// Internal result of creating a session and its first attempt.
#[derive(Debug)]
pub(crate) struct StartedSession {
    pub(crate) token: String,
    pub(crate) step_attempt_id: String,
    pub(crate) challenge: Option<String>,
    /// FIDO2 only: the credentials registered for this user, base64url. The
    /// client offers them to the security key, which answers for the one it
    /// holds.
    pub(crate) credential_ids: Vec<String>,
    pub(crate) superseded_token_hash: Option<String>,
}

/// Result returned by the legacy start contract.
#[derive(Debug)]
pub struct LegacyStartOutcome {
    pub token: String,
    pub challenge: Option<String>,
    pub superseded_token_hash: Option<String>,
}

/// Result returned by the multi-step start contract.
#[derive(Debug)]
pub struct MultiStepStartOutcome {
    pub token: String,
    pub step_attempt_id: String,
    pub challenge: Option<String>,
    /// FIDO2 only: the credentials registered for this user, base64url. The
    /// client offers them to the security key, which answers for the one it
    /// holds.
    pub credential_ids: Vec<String>,
    pub superseded_token_hash: Option<String>,
}

/// Credential passed to the shared method verifier.
///
/// Contract-specific credential types are converted to this representation at their public engine
/// method boundary. It deliberately carries no attempt ID or contract marker.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum VerificationProof {
    Code(String),
    BiometricSignature(String),
    Fido2 {
        signature: Vec<u8>,
        authenticator_data: Vec<u8>,
        credential_id: Vec<u8>,
    },
}

/// Outcome returned by the legacy finish contract.
#[derive(Debug, PartialEq)]
pub enum LegacyFinishOutcome {
    /// The single legacy step completed and a preshared key was minted.
    Completed { preshared_key: String },
    /// Still waiting for external confirmation (OIDC or mobile auth) to be completed.
    AwaitingExternal,
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
