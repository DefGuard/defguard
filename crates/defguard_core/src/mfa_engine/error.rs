//! Domain error types for the MFA engine.
//!
//! The engine is proto-free: its public API returns these typed enums, and only the gRPC handlers
//! convert them to `tonic::Status` (see `grpc::proxy::client_mfa`).

use thiserror::Error;

use super::{authorize::ClientMfaServerError, method::InitiateError};

/// Errors returned by the legacy and multi-step start methods.
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

/// Internal failures returned by shared finish helpers.
#[derive(Debug, Error)]
pub(super) enum FinishCoreError {
    #[error("unexpected error")]
    Internal,
    #[error(transparent)]
    Event(#[from] ClientMfaServerError),
}
