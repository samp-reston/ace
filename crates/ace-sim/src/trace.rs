//! Deterministic bus trace recording and replay.
//!
//! A seed makes a run repeatable, but a seed alone is not a durable explanation of a failure:
//! changing fault probabilities, queue bounds, or code can make the same seed take a different
//! path. [`TraceBus`] records the bus operations and observable results of one run. [`TraceReplay`]
//! then replays those results while checking that the caller performs the same operations in the
//! same order.
//!
//! The trace is deliberately below protocol knowledge. It records messages and clock advances,
//! not DoIP, UDS, or ISO-TP events. A consumer can therefore use it with a bare [`SimBus`]-shaped
//! transport, while scenario-specific assertions remain in the consumer crate.

use crate::bus::{Envelope, SimBus};
use crate::clock::{Duration, Instant};
use crate::fault::FaultConfig;
use crate::io::NodeAddress;
use ace_core::Vec;

/// One observable operation in a recorded bus run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceStep<const MAX_DATA: usize, const MAX_QUEUED: usize> {
    /// A send attempt, including whether faults or capacity accepted it.
    Send {
        src: NodeAddress,
        dst: NodeAddress,
        data: Vec<u8, MAX_DATA>,
        min_latency: Duration,
        accepted: bool,
    },

    /// A clock advance and the messages that became observable at its end.
    Tick {
        duration: Duration,
        now: Instant,
        delivered: Vec<Envelope<MAX_DATA>, MAX_QUEUED>,
    },
}

/// A bounded trace of bus operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace<const MAX_DATA: usize, const MAX_QUEUED: usize, const MAX_STEPS: usize> {
    steps: Vec<TraceStep<MAX_DATA, MAX_QUEUED>, MAX_STEPS>,
}

impl<const MAX_DATA: usize, const MAX_QUEUED: usize, const MAX_STEPS: usize>
    Trace<MAX_DATA, MAX_QUEUED, MAX_STEPS>
{
    pub const fn new() -> Self {
        Self { steps: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.steps.is_full()
    }

    /// Borrows the recorded operations in execution order.
    ///
    /// A consumer can serialize these bounded records or inspect them while diagnosing a
    /// failing run; replay itself does not require allocation or a standard-library format.
    pub fn steps(&self) -> &[TraceStep<MAX_DATA, MAX_QUEUED>] {
        self.steps.as_slice()
    }

    pub fn replay(&self) -> TraceReplay<'_, MAX_DATA, MAX_QUEUED, MAX_STEPS> {
        TraceReplay {
            trace: self,
            cursor: 0,
            now: Instant::ZERO,
        }
    }
}

impl<const MAX_DATA: usize, const MAX_QUEUED: usize, const MAX_STEPS: usize> Default
    for Trace<MAX_DATA, MAX_QUEUED, MAX_STEPS>
{
    fn default() -> Self {
        Self::new()
    }
}

/// Errors raised while recording a trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceError {
    /// The trace reached its configured `MAX_STEPS` bound.
    Full,

    /// A send payload exceeded the trace's `MAX_DATA` bound.
    PayloadTooLarge,
}

/// A bus wrapper that records each operation and its observable result.
///
/// The wrapper is intentionally fallible where the underlying [`SimBus`] is bounded: a full
/// trace is reported before the underlying bus is touched, so recording failure cannot silently
/// produce an untraceable state transition.
pub struct TraceBus<const MAX_DATA: usize, const MAX_QUEUED: usize, const MAX_STEPS: usize> {
    bus: SimBus<MAX_DATA, MAX_QUEUED>,
    trace: Trace<MAX_DATA, MAX_QUEUED, MAX_STEPS>,
}

impl<const MAX_DATA: usize, const MAX_QUEUED: usize, const MAX_STEPS: usize>
    TraceBus<MAX_DATA, MAX_QUEUED, MAX_STEPS>
{
    pub fn new(seed: u64, faults: FaultConfig) -> Self {
        Self {
            bus: SimBus::new(seed, faults),
            trace: Trace::new(),
        }
    }

    pub fn now(&self) -> Instant {
        self.bus.now()
    }

    pub fn send(
        &mut self,
        src: NodeAddress,
        dst: NodeAddress,
        data: &[u8],
    ) -> Result<bool, TraceError> {
        self.send_with_min_latency(src, dst, data, Duration::ZERO)
    }

    pub fn send_with_min_latency(
        &mut self,
        src: NodeAddress,
        dst: NodeAddress,
        data: &[u8],
        min_latency: Duration,
    ) -> Result<bool, TraceError> {
        self.ensure_recordable(data)?;

        let accepted = self
            .bus
            .send_with_min_latency(src.clone(), dst.clone(), data, min_latency);
        let mut payload = Vec::new();
        payload
            .extend_from_slice(data)
            .map_err(|_| TraceError::PayloadTooLarge)?;

        self.trace
            .steps
            .push(TraceStep::Send {
                src,
                dst,
                data: payload,
                min_latency,
                accepted,
            })
            .map_err(|_| TraceError::Full)?;

        Ok(accepted)
    }

    pub fn tick(
        &mut self,
        duration: Duration,
    ) -> Result<Vec<Envelope<MAX_DATA>, MAX_QUEUED>, TraceError> {
        self.ensure_step_recordable()?;

        let delivered = self.bus.tick(duration);
        let now = self.bus.now();
        let recorded = delivered.clone();

        self.trace
            .steps
            .push(TraceStep::Tick {
                duration,
                now,
                delivered: recorded,
            })
            .map_err(|_| TraceError::Full)?;

        Ok(delivered)
    }

    pub fn trace(&self) -> &Trace<MAX_DATA, MAX_QUEUED, MAX_STEPS> {
        &self.trace
    }

    pub fn into_trace(self) -> Trace<MAX_DATA, MAX_QUEUED, MAX_STEPS> {
        self.trace
    }

    fn ensure_recordable(&self, data: &[u8]) -> Result<(), TraceError> {
        if data.len() > MAX_DATA {
            return Err(TraceError::PayloadTooLarge);
        }

        self.ensure_step_recordable()
    }

    fn ensure_step_recordable(&self) -> Result<(), TraceError> {
        if self.trace.is_full() {
            Err(TraceError::Full)
        } else {
            Ok(())
        }
    }
}

/// Errors raised when a replayed operation differs from the recorded run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayError {
    /// The caller attempted an operation after the trace was consumed.
    Exhausted,

    /// The trace contains a tick where the caller attempted a send.
    ExpectedSend,

    /// The trace contains a send where the caller attempted a tick.
    ExpectedTick,

    /// The send endpoints, payload, or minimum latency differ from the recording.
    SendMismatch,

    /// The tick duration differs from the recording.
    TickMismatch,

    /// Replay ended before every recorded operation was consumed.
    Incomplete,
}

/// Replays a [`Trace`] and validates the caller's operation sequence.
pub struct TraceReplay<'a, const MAX_DATA: usize, const MAX_QUEUED: usize, const MAX_STEPS: usize> {
    trace: &'a Trace<MAX_DATA, MAX_QUEUED, MAX_STEPS>,
    cursor: usize,
    now: Instant,
}

impl<'a, const MAX_DATA: usize, const MAX_QUEUED: usize, const MAX_STEPS: usize>
    TraceReplay<'a, MAX_DATA, MAX_QUEUED, MAX_STEPS>
{
    pub fn now(&self) -> Instant {
        self.now
    }

    pub fn send(
        &mut self,
        src: &NodeAddress,
        dst: &NodeAddress,
        data: &[u8],
        min_latency: Duration,
    ) -> Result<bool, ReplayError> {
        let step = self.next_step()?;
        let accepted = match step {
            TraceStep::Send {
                src: recorded_src,
                dst: recorded_dst,
                data: recorded_data,
                min_latency: recorded_latency,
                accepted,
            } => {
                if recorded_src != src
                    || recorded_dst != dst
                    || recorded_data.as_slice() != data
                    || *recorded_latency != min_latency
                {
                    return Err(ReplayError::SendMismatch);
                }
                *accepted
            }
            TraceStep::Tick { .. } => return Err(ReplayError::ExpectedSend),
        };

        self.cursor += 1;
        Ok(accepted)
    }

    pub fn tick(
        &mut self,
        duration: Duration,
    ) -> Result<Vec<Envelope<MAX_DATA>, MAX_QUEUED>, ReplayError> {
        let (now, delivered) = match self.next_step()? {
            TraceStep::Tick {
                duration: recorded_duration,
                now,
                delivered,
            } => {
                if *recorded_duration != duration {
                    return Err(ReplayError::TickMismatch);
                }
                (*now, delivered.clone())
            }
            TraceStep::Send { .. } => return Err(ReplayError::ExpectedTick),
        };

        self.cursor += 1;
        self.now = now;
        Ok(delivered)
    }

    pub fn finish(&self) -> Result<(), ReplayError> {
        if self.cursor == self.trace.len() {
            Ok(())
        } else {
            Err(ReplayError::Incomplete)
        }
    }

    pub fn remaining(&self) -> usize {
        self.trace.len().saturating_sub(self.cursor)
    }

    fn next_step(&self) -> Result<&TraceStep<MAX_DATA, MAX_QUEUED>, ReplayError> {
        self.trace
            .steps
            .get(self.cursor)
            .ok_or(ReplayError::Exhausted)
    }
}
