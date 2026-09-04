//! Batteries-included entry point for the ACED diagnostics stack.
//!
//! Alloc-backed by default. For no_std/embedded targets, depend on the
//! individual `ace-*` crates directly instead of this facade.

pub use ace_can as can;
pub use ace_client as client;
pub use ace_core as core;
pub use ace_doip as doip;
pub use ace_gateway as gateway;
pub use ace_macros as macros;
pub use ace_proto as proto;
pub use ace_server as server;
pub use ace_sim as sim;
pub use ace_uds as uds;
