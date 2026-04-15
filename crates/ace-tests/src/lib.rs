///! Workspace-level RST and integration test crate.
///!
///! Structure:
///!     - harness - SimRunner setup, tick helpers, assertion helpers
///!     - fixtures - concrete ServerHandler + SecurityProvider impls for testing
///!     - dst - property tests organised by service group

/// Reusable DST Infrastructure.
///
/// Provides:
///     - DstScenario - wires a UdsClient and UdsServer onto a SimBus
///     - tick helpers - tick_n, tick_until_quiet
///     - assertion helpers - expect_positive, expect_nrc, expect_timeout,
///     expect_periodic, assert_session, asset_security
pub mod harness;

pub mod fixtures;

pub mod dst;
