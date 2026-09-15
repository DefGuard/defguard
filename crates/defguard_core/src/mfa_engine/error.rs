//! Domain error types for the MFA engine.
//!
//! The engine is proto-free: its public API returns these typed enums, and only the gRPC handlers
//! convert them to `tonic::Status` (see `grpc::proxy::client_mfa`).

use thiserror::Error;

use super::{authorize::ClientMfaServerError, method::InitiateError};

/// Error surfaced by [`super::MfaEngine::start`] and [`super::MfaEngine::start_multi_step`].
#[derive(Debug, Error)]
pub enum StartError {
    /// A multi-step (2+ step) flow requires a business license.
    #[error("multi-step MFA is not available for this location")]
    MultiStepNotAvailable,
    #[error("MFA plan length does not match the location's flow")]
    PlanLengthMismatch,
    /// The selected method is not set up for this user or device.
    #[error("selected MFA method is not available")]
    MethodNotAvailable,
    #[error("Select MFA method is not available for the device.")]
    BiometricNotConfigured,
    #[error("unexpected error")]
    Internal,
    #[error(transparent)]
    Initiate(#[from] InitiateError),
}

/// Error surfaced by [`super::MfaEngine::step_start`].
#[derive(Debug, Error)]
pub enum StepError {
    #[error("login session not found")]
    SessionNotFound,
    #[error("MFA method is not in the current step")]
    MethodNotInStep,
    #[error("MFA method is not configured for this user")]
    MethodNotConfigured,
    #[error("unexpected error")]
    Internal,
    #[error(transparent)]
    Initiate(#[from] InitiateError),
}

/// Error surfaced by [`super::MfaEngine::finish`].
#[derive(Debug, Error)]
pub enum FinishError {
    #[error("login session not found")]
    SessionNotFound,
    #[error("no MFA attempt in progress")]
    UninitializedStep,
    #[error("OIDC authentication not completed yet")]
    OidcNotCompleted,
    #[error("unauthorized")]
    Unauthorized,
    #[error("Too many failed MFA attempts. Please try connecting again.")]
    AttemptLimit,
    #[error("stale MFA attempt")]
    StaleAttempt,
    #[error("Challenge not found in session")]
    MissingChallenge,
    #[error("Challenge not found in MFA session")]
    MissingBiometricChallenge,
    #[error("{message}")]
    MalformedProof { message: &'static str },
    #[error("unexpected error")]
    Internal,
    #[error(transparent)]
    Event(#[from] ClientMfaServerError),
}

/// Errors reachable from the multi-step finish and mobile-approval methods.
#[derive(Debug, Error)]
pub enum StepFinishError {
    #[error("login session not found")]
    SessionNotFound,
    #[error("no MFA attempt in progress")]
    UninitializedStep,
    #[error("unauthorized")]
    Unauthorized,
    #[error("Too many failed MFA attempts. Please try connecting again.")]
    AttemptLimit,
    #[error("stale MFA attempt")]
    StaleAttempt,
    #[error("Challenge not found in session")]
    MissingChallenge,
    #[error("Challenge not found in MFA session")]
    MissingBiometricChallenge,
    #[error("{message}")]
    MalformedProof { message: &'static str },
    #[error("unexpected error")]
    Internal,
    #[error(transparent)]
    Event(#[from] ClientMfaServerError),
}

/// Internal failures shared by the contract-specific finish methods.
#[derive(Debug, Error)]
pub(super) enum FinishCoreError {
    #[error("unexpected error")]
    Internal,
    #[error(transparent)]
    Event(#[from] ClientMfaServerError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_finish_error_messages_are_frozen() {
        let cases = [
            (FinishError::SessionNotFound, "login session not found"),
            (FinishError::UninitializedStep, "no MFA attempt in progress"),
            (
                FinishError::OidcNotCompleted,
                "OIDC authentication not completed yet",
            ),
            (FinishError::Unauthorized, "unauthorized"),
            (
                FinishError::AttemptLimit,
                "Too many failed MFA attempts. Please try connecting again.",
            ),
            (FinishError::StaleAttempt, "stale MFA attempt"),
            (
                FinishError::MissingChallenge,
                "Challenge not found in session",
            ),
            (
                FinishError::MissingBiometricChallenge,
                "Challenge not found in MFA session",
            ),
            (
                FinishError::MalformedProof {
                    message: "malformed",
                },
                "malformed",
            ),
            (FinishError::Internal, "unexpected error"),
        ];

        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }
}
