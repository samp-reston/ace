//! Seeded generator for reproducible runs. The [`Rng`] trait lives in `ace-core`.

// region: Imports

pub use ace_core::rng::Rng;

// endregion: Imports

// region: XorShift64

/// A fast, seedable Xorshift64 RNG
///
/// Not cryptographically secure - suitable for simulation only. Produces a full cycle of 2^64-1
/// values before repeating
#[derive(Debug, Clone)]
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    /// Creates a new RNG with the given seed.
    ///
    /// Seed must be non-zero - passing 0 will be replaced with a default non-zero seed.
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0xDECA_FC0F_FEEC_AFE1
            } else {
                seed
            },
        }
    }
}

impl Rng for Xorshift64 {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

// endregion: XorShift64
