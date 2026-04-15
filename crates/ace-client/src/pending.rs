// region: Imports

use ace_sim::clock::{Duration, Instant};

// endregion: Imports

// region: P2 State

/// Tracks which timeout phase a pending request is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum P2State {
    /// Waiting for initial response within P2 timeout.
    Waiting,
    /// An 0x78 Response Pending was received - now waiting within P2* timeout.
    Extended,
}

// endregion: P2 State

// region: Pending Request

/// A request that has been sent and is waiting a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRequest {
    /// The request SID byte - used to match incoming responses.
    pub sid: u8,

    /// The time at which this request was sent.
    pub sent_at: Instant,

    /// The deadline by which a response must arrive.
    pub deadline: Instant,

    /// Current timeout phase.
    pub p2_state: P2State,
}

impl PendingRequest {
    pub fn new(sid: u8, sent_at: Instant, p2_timeout: Duration) -> Self {
        Self {
            sid,
            sent_at,
            deadline: sent_at + p2_timeout,
            p2_state: P2State::Waiting,
        }
    }

    /// Returns true if the deadline has passed.
    pub fn is_expired(&self, now: Instant) -> bool {
        now > self.deadline
    }

    /// Transitions to the extended P2* timeout after receiving 0x78.
    pub fn extend(&mut self, now: Instant, p2_extended_timeout: Duration) {
        self.deadline = now + p2_extended_timeout;
        self.p2_state = P2State::Extended;
    }
}

// endregion: Pending Request
