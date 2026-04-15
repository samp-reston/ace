/// Pre-configured UdsClient builders for DST tests.
pub mod client;

/// Concrete ServerHandler and SecurityProvider implementations for testing.
///
/// TestHandler provides a minimal in-memory DID store and hook implementations that cover the
/// services exercised by the DST tests.
/// TestSecurityProvider uses a fixed XOR algorithm - seed XOR 0xFF = key - which is trivially
/// predictable and suitable only for test.
pub mod server;

pub mod doip;

pub use server::{TestHandler, TestSecurityProvider};
