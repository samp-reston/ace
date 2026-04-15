// region: Imports

// endregion: Imports

// region: Instant

/// A point in time represented as microseconds since an arbitrary epoch.
///
/// The epoch is not defined - only differences between `Instant` values
/// are meaningful. On simulation targets the epoch is the start of the
/// simulation. On real targets it is typically system boot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(u64);

impl Instant {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub fn from_micros(us: u64) -> Self {
        Self(us)
    }

    #[inline]
    pub fn as_micros(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn checked_duration_since(&self, earlier: Self) -> Option<Duration> {
        self.0.checked_sub(earlier.0).map(Duration::from_micros)
    }
}

// endregion: Instant

// region: Duration

/// A span of time in microseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(u64);

impl Duration {
    pub const ZERO: Self = Self(0);

    #[inline]
    pub fn from_micros(us: u64) -> Self {
        Self(us)
    }

    #[inline]
    pub const fn from_millis(ms: u64) -> Self {
        Self(ms * 1_000)
    }

    #[inline]
    pub fn from_secs(s: u64) -> Self {
        Self(s * 1_000_000)
    }

    #[inline]
    pub fn as_micros(&self) -> u64 {
        self.0
    }

    #[inline]
    pub fn as_millis(&self) -> u64 {
        self.0 / 1_000
    }

    #[inline]
    pub fn as_secs(&self) -> u64 {
        self.0 / 1_000_000
    }
}

impl core::ops::Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, rhs: Duration) -> Self::Output {
        Instant(self.0 + rhs.0)
    }
}

impl core::ops::Sub<Duration> for Instant {
    type Output = Instant;
    fn sub(self, rhs: Duration) -> Self::Output {
        Instant(self.0.saturating_sub(rhs.0))
    }
}

// endregion: Duration

// region: Clock Trait

/// Provides the current time as an [`Instant`].
///
/// Implementations must be deterministic within a simulation context -
/// the same sequence of `now()` calls must return the same values given
/// the same simulation inputs. On real targets this wraps the hardware
/// timer or OS monotonic clock.
pub trait Clock {
    fn now(&self) -> Instant;
}

// endregion: Clock Trait

// region: SimClock

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
