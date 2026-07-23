//! Test-only helpers shared across the workspace.
//!
//! Everything under this module is gated behind the `test-support` feature and
//! is intended to be pulled in via `[dev-dependencies]` by other crates'
//! integration tests.

pub mod smtp;
