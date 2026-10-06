//! Workspace-level DST and integration test crate.
//!
//! Structure:
//!     - harness - SimRunner setup, tick helpers, assertion helpers
//!     - fixtures - concrete ServerHandler + SecurityProvider impls for testing
//!     - dst - property tests organised by service group
//!
//! Every module is gated on `cfg(test)`. This crate exists only to host tests, so
//! in a non-test build its `#[test]` functions are stripped and every import,
//! helper, and fixture they rely on is correctly reported as dead. Gating keeps
//! `cargo build` and `cargo check` warning-clean without deleting test support
//! code or suppressing the `dead_code` lint.

#[cfg(test)]
pub mod dst;

#[cfg(test)]
pub mod fixtures;

/// Reusable DST Infrastructure.
///
/// Provides:
///     - DstScenario - wires a UdsClient and UdsServer onto a SimBus
///     - tick helpers - tick_n, tick_until_quiet
///     - assertion helpers - expect_positive, expect_nrc, expect_timeout,
///     expect_periodic, assert_session, assert_security
#[cfg(test)]
pub mod harness;
