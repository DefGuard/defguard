//! Domain types for the MFA engine.
//!
//! These are proto-free: the conversions to and from the proto messages live in the gRPC handler
//! (`grpc::proxy::client_mfa`), so the engine can be exercised without a transport.

/// Result of `start`. `token` is returned exactly once, `challenge` is `Some` only for a method
/// the client must sign against (biometric or mobile approve), and `superseded_token_hash` names
/// the session this start replaced so the handler can cancel its waiter.
#[derive(Debug)]
pub struct StartOutcome {
    pub token: String,
    pub challenge: Option<String>,
    pub superseded_token_hash: Option<String>,
}

/// A proof submitted to `finish`.
///
/// `code` holds a TOTP code, an email code, or a signed challenge, and `auth_pub_key` the
/// mobile-approve signing device. `code` stays untyped because `ClientMfaFinishRequest` carries no
/// method: at the point a `Proof` is built nothing knows which kind of credential it holds, and the
/// meaning is fixed later by `ephemeral_state.selected_method`, which only `verify` reads.
///
/// `step_attempt_id` binds the proof to one attempt so a stale or duplicate proof cannot advance
/// the step. Pre-2.2 clients omit it and fall back to the step cursor alone.
pub struct Proof {
    pub code: Option<String>,
    pub auth_pub_key: Option<String>,
    pub step_attempt_id: Option<String>,
}

/// Result of `step_start`: the minted attempt id plus an optional biometric / mobile-approve
/// challenge.
#[derive(Debug)]
pub struct StepStarted {
    pub step_attempt_id: String,
    pub challenge: Option<String>,
}

/// Outcome of `finish`.
#[derive(Debug, PartialEq)]
pub enum FinishOutcome {
    /// The step just submitted advanced the flow to `next_step` (0-indexed).
    Advanced { next_step: u32 },
    /// The final step completed and a preshared key was minted.
    Completed { preshared_key: String },
    /// Still waiting for external confirmation (OIDC or mobile auth) to be completed.
    AwaitingExternal,
}

/// Why a step of the submitted plan was refused at `start`.
#[derive(Debug, PartialEq, Eq)]
pub enum StartRejectionReason {
    /// The chosen method is not in this step's allowed set.
    MethodNotInStep,
    /// The step has no methods left once the license filter is applied.
    StepEmptyAfterLicense,
    /// The user cannot satisfy the step. Deliberately opaque.
    StepUnavailable,
}

/// A sparse per-step rejection: only failing steps are returned.
#[derive(Debug)]
pub struct StepRejection {
    pub step: u32,
    pub reason: StartRejectionReason,
}

/// Result of the multi-step `start`. A refused plan creates no session, token, or event.
#[derive(Debug)]
pub enum StartResult {
    Accepted(StartOutcome),
    Rejected(Vec<StepRejection>),
}
