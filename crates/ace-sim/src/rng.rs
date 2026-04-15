// region: Rng Trait

/// A seeded random number generator for deterministic simulation.
///
/// All randomness in the simulation must flow through this trait. The same seed must produce the
/// same sequence of values - this is what makes simulation runs reproducible and replayable.
pub trait Rng {
    fn next_u64(&mut self) -> u64;

    /// Returns a random `u32`.
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Returns a random `u8`.
    fn next_u8(&mut self) -> u8 {
        self.next_u64() as u8
    }

    // Returns true with probability `numerator / denominator`.
    fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        if denominator == 0 {
            return false;
        }

        (self.next_u32() % denominator) < numerator
    }

    /// Returns a random `usize` in `0..len`.
    fn index(&mut self, len: usize) -> usize {
        if len == 0 {
            return 0;
        }

        (self.next_u64() as usize) % len
    }
}

// endregion: Rng Trait

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
