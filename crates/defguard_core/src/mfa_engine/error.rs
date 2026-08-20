//! Domain error types for the MFA engine.
//!
//! The engine is a proto-free domain module: its public API returns these typed enums, and only
//! the gRPC handlers convert them to `tonic::Status` (see `grpc::proxy::client_mfa`). The
//! `From` impls there are the ticket-03 status table, byte-identical.

use thiserror::Error;

use super::{authorize::ClientMfaServerError, method::InitiateError};

/// Error surfaced by [`super::MfaEngine::start`] and [`super::MfaEngine::start_multi_step`].
#[derive(Debug, Error)]
pub enum StartError {
    /// A multi-step (2+ step) flow requires a business license.
    #[error("multi-step MFA is not available for this location")]
    MultiStepNotAvailable,
    /// The submitted plan length does not match the resolved flow.
    #[error("MFA plan length does not match the location's flow")]
    PlanLengthMismatch,
    /// The selected method is not set up for this user/device (legacy vocabulary).
    #[error("selected MFA method is not available")]
    MethodNotAvailable,
    /// Biometric is not configured for the device.
    #[error("Select MFA method is not available for the device.")]
    BiometricNotConfigured,
    /// An internal failure.
    #[error("unexpected error")]
    Internal,
    /// A step-initiation failure, mapped from [`InitiateError`].
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
