// region: Imports

use crate::clock::Duration;

// endregion: Imports

// region: Fault Config

/// Controls which faults the simulation bus may inject and at what rates.
///
/// All probabilities are expressed as `(numerator, denominator)` pairs. For example `(1, 100)`
/// means a 1% chance per eligible event.
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Probability a message is silently dropped.
    pub message_loss: (u32, u32),

    /// Probability two consecutive messages are reordered.
    pub message_reorder: (u32, u32),

    /// Probability a message is delayed. Delay duration is drawn uniformly from `0..max_delay_us`.
    pub message_delay: (u32, u32),
    pub max_delay: Duration,

    /// Probability a message payload byte is corrupted. Applied per-byte independently.
    pub corruption: (u32, u32),

    /// Probability a respponse is replaced with a timeout (i.e suppressed entirely, forcing the
    /// sender to timeout)
    pub timeout: (u32, u32),
}

impl FaultConfig {
    /// No faults - fully deterministic pass-through.
    pub fn none() -> Self {
        Self {
            message_loss: (0, 1),
            message_reorder: (0, 1),
            message_delay: (0, 1),
            max_delay: Duration::ZERO,
            corruption: (0, 1),
            timeout: (0, 1),
        }
    }

    /// Light fault injection - suitable for initial CI runs.
    pub fn light() -> Self {
        Self {
            message_loss: (1, 100),
            message_reorder: (1, 50),
            message_delay: (1, 20),
            max_delay: Duration::from_millis(50),
            corruption: (1, 500),
            timeout: (1, 200),
        }
    }

    /// Heavy fault injection - chaos mode for stress testing.
    pub fn chaos() -> Self {
        Self {
            message_loss: (1, 10),
            message_reorder: (1, 5),
            message_delay: (1, 3),
            max_delay: Duration::from_millis(500),
            corruption: (1, 50),
            timeout: (1, 20),
        }
    }
}

// endregion: Fault Config
