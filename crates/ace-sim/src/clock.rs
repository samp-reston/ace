//! Virtual clock. The time primitives themselves live in `ace-core`.

// region: Imports

pub use ace_core::clock::{Clock, Duration, Instant};

// endregion: Imports

// region: SimClock

/// A clock advanced explicitly by the simulation rather than by wall time.
#[derive(Debug, Clone)]
pub struct SimClock {
    now: Instant,
}

impl SimClock {
    pub fn new() -> Self {
        Self { now: Instant::ZERO }
    }

    /// Advances the clock by `duration`.
    pub fn advance(&mut self, duration: Duration) {
        self.now = self.now + duration;
    }

    /// Sets the clock to an absolute instant.
    pub fn set(&mut self, instant: Instant) {
        self.now = instant
    }
}

impl Default for SimClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SimClock {
    fn now(&self) -> Instant {
        self.now
    }
}

// endregion: SimClock
