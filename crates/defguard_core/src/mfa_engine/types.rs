use defguard_common::db::{Id, models::vpn_client_session::VpnClientMfaMethod};
use defguard_proto::client_types::{
    ClientMfaStepStartResponse, MfaAdvanced, MfaAwaitingExternal, MfaCompleted,
    MfaStartRejectionReason, MfaStepRejection, MfaStepResult, mfa_step_result,
};

/// A resolved MFA flow frozen at `start`: the governing flow's id plus the ordered, per-step
/// allowed method sets as resolved for the user.
pub struct StartPlan {
    pub flow_id: Id,
    pub steps: Vec<Vec<VpnClientMfaMethod>>,
}

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

/// A proof submitted to `finish`: the optional code and optional auth public key. `code` carries
/// the TOTP / email code or a signed-challenge signature; `auth_pub_key` carries the mobile
/// approve signing device.
pub struct Proof {
    pub code: Option<String>,
    pub auth_pub_key: Option<String>,
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
    /// An out-of-band step has not resolved yet; the client keeps polling.
    AwaitingExternal,
}

impl From<FinishOutcome> for MfaStepResult {
    fn from(value: FinishOutcome) -> Self {
        let outcome = match value {
            FinishOutcome::Advanced { next_step } => {
                mfa_step_result::Outcome::Advanced(MfaAdvanced { next_step })
            }
            FinishOutcome::Completed { preshared_key } => {
                mfa_step_result::Outcome::Completed(MfaCompleted { preshared_key })
            }
            FinishOutcome::AwaitingExternal => {
                mfa_step_result::Outcome::AwaitingExternal(MfaAwaitingExternal {})
            }
        };
        MfaStepResult {
            outcome: Some(outcome),
        }
    }
}

impl From<StepStarted> for ClientMfaStepStartResponse {
    fn from(value: StepStarted) -> Self {
        Self {
            step_attempt_id: value.step_attempt_id,
            challenge: value.challenge,
        }
    }
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

impl From<StartRejectionReason> for MfaStartRejectionReason {
    fn from(value: StartRejectionReason) -> Self {
        match value {
            StartRejectionReason::MethodNotInStep => Self::MfaStartRejectionMethodNotInStep,
            StartRejectionReason::StepEmptyAfterLicense => {
                Self::MfaStartRejectionStepEmptyAfterLicense
            }
            StartRejectionReason::StepUnavailable => Self::MfaStartRejectionStepUnavailable,
        }
    }
}

impl From<StepRejection> for MfaStepRejection {
    fn from(value: StepRejection) -> Self {
        Self {
            step: value.step,
            reason: MfaStartRejectionReason::from(value.reason) as i32,
        }
    }
}

/// Result of the multi-step `start`: the session was accepted, or the plan was refused with
/// sparse rejections. A refused plan creates no session, token, or event.
#[derive(Debug)]
pub enum StartResult {
    Accepted(StartOutcome),
    Rejected(Vec<StepRejection>),
}
