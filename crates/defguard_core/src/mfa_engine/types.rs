//! Domain types for the MFA engine.
//!
//! These are proto-free by design: the conversions to and from the frozen proto messages live in
//! the gRPC handler (`grpc::proxy::client_mfa`), so the engine can be exercised - and reasoned
//! about - without a transport.

/// Result of `start`, carrying the raw token (returned exactly once), the optional step-0
/// challenge, and the hash of any session that was superseded so the handler can cancel its
/// waiter.
///
/// `challenge` is `Some` only when the step's method requires one (biometric or mobile
/// approve), where the client needs it to sign the proof.
#[derive(Debug)]
pub struct StartOutcome {
    pub token: String,
    pub challenge: Option<String>,
    pub superseded_token_hash: Option<String>,
}

/// A proof submitted to `finish`.
///
/// - `code` carries the TOTP code, the email code, or a signed challenge.
/// - `auth_pub_key` carries the mobile-approve signing device.
/// - `step_attempt_id` binds the proof to a specific attempt so a stale or duplicate proof cannot
///   advance the step. Pre-2.2 clients omit it and the legacy single-step path keeps working with
///   `None`, where the binding falls back to the step cursor alone.
///
/// `code` is deliberately one untyped field rather than a per-method enum. `ClientMfaFinishRequest`
/// carries no method (that is why initializing a step is mandatory - see ticket 05 §5), so at the
/// point a `Proof` is built, nothing here knows which kind of credential it holds; the meaning is
/// fixed later by `ephemeral_state.selected_method`, which only `verify` reads. An enum would
/// force the handler to guess the method, which is strictly worse than carrying it untyped.
/// Splitting these fields properly needs the proto to name the method, which it does not.
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
    /// The final step completed; the flow was collected and a preshared key minted.
    Completed { preshared_key: String },
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

/// Result of the multi-step `start`: the session was accepted, or the plan was refused with
/// sparse rejections. A refused plan creates no session, token, or event.
#[derive(Debug)]
pub enum StartResult {
    Accepted(StartOutcome),
    Rejected(Vec<StepRejection>),
}
