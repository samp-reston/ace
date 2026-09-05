//! The [`Rng`] abstraction.
//!
//! A foundation trait, not a simulation type: a production ECU needs randomness
//! for SecurityAccess seed generation, and must be able to supply its own
//! hardware entropy source. Concrete generators live with their consumer —
//! `ace-sim` provides a seeded, reproducible one for deterministic runs.

// region: Rng Trait

/// A random number generator.
///
/// Implementations used for simulation must be seeded and reproducible: the same
/// seed must produce the same sequence of values, which is what makes runs
/// replayable. Implementations used to generate security material must instead be
/// backed by a suitable entropy source — a reproducible generator is never
/// appropriate there.
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
