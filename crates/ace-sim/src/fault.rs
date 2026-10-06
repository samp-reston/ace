// region: Imports

use crate::clock::Duration;
use ace_core::Vec;

// endregion: Imports

// region: Scripted Faults

/// Maximum number of one-shot scripted faults retained by a bus.
pub const MAX_SCRIPTED_FAULTS: usize = 16;

/// A one-shot mutation applied to the next send attempt.
///
/// These actions deliberately describe transport/frame behavior, not UDS, DoIP, or ISO-TP
/// concepts. A consumer that knows a protocol may use `ReplaceByte` to create a malformed
/// protocol frame, but the simulation layer does not interpret the byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptedFault {
    /// Suppress the next send attempt completely.
    DropNext,

    /// Add a deterministic delay to the next accepted message.
    DelayNext(Duration),

    /// Enqueue the next accepted message twice.
    DuplicateNext,

    /// Replace one byte in the next accepted payload. An out-of-range offset leaves the
    /// payload unchanged rather than panicking.
    ReplaceByte { offset: u16, value: u8 },
}

/// A bounded FIFO sequence of one-shot faults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultPlan {
    actions: Vec<ScriptedFault, MAX_SCRIPTED_FAULTS>,
}

impl FaultPlan {
    pub const fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    /// Appends a fault to the end of the plan.
    pub fn push(&mut self, fault: ScriptedFault) -> Result<(), FaultPlanError> {
        self.actions.push(fault).map_err(|_| FaultPlanError::Full)
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Removes and returns the next one-shot action.
    pub(crate) fn next(&mut self) -> Option<ScriptedFault> {
        if self.actions.is_empty() {
            None
        } else {
            Some(self.actions.remove(0))
        }
    }
}

impl Default for FaultPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors from exceeding the bounded scripted-fault plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultPlanError {
    Full,
}

// endregion: Scripted Faults

// region: Fault Config

/// Controls which faults the simulation bus may inject and at what rates.
///
/// All probabilities are expressed as `(numerator, denominator)` pairs. For example `(1, 100)`
/// means a 1% chance per eligible event. `script` adds deterministic one-shot frame actions on
/// top of the probabilistic faults.
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

    /// Probability a response is replaced with a timeout (i.e. suppressed entirely, forcing the
    /// sender to timeout).
    pub timeout: (u32, u32),

    /// Deterministic one-shot frame actions, consumed in FIFO order.
    pub script: FaultPlan,
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
            script: FaultPlan::new(),
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
            script: FaultPlan::new(),
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
            script: FaultPlan::new(),
        }
    }
}

// endregion: Fault Config
